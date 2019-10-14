use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::time::Duration;
use std::{fs, io, mem, thread};

use crossterm_utils::{ErrorKind, Result};

use lazy_static::lazy_static;

use crate::{InputEvent, InternalEvent, KeyEvent, MouseButton, MouseEvent};

/// An internal event provider interface.
pub(crate) trait InternalEventProvider: Send {
    /// Pauses the provider.
    ///
    /// This method must be called when all the receivers were dropped.
    fn pause(&mut self);

    /// Creates a new `InternalEvent` receiver.
    fn receiver(&mut self) -> Receiver<InternalEvent>;
}

lazy_static! {
    /// A shared internal event provider.
    static ref INTERNAL_EVENT_PROVIDER: Mutex<Box<dyn InternalEventProvider>> =
        Mutex::new(default_internal_event_provider());
}

/// Creates a new default internal event provider.
fn default_internal_event_provider() -> Box<dyn InternalEventProvider> {
    #[cfg(unix)]
    Box::new(UnixInternalEventProvider::new())
    // TODO 1.0: #[cfg(windows)]
}

/// A internal event senders wrapper.
///
/// The main purpose of this structure is to make the list of senders
/// easily sharable (clone) & maintainable.
#[derive(Clone)]
struct UnixInternalEventChannels {
    senders: Arc<Mutex<Vec<Sender<InternalEvent>>>>,
}

impl UnixInternalEventChannels {
    /// Creates a new `UnixInternalEventChannels`.
    fn new() -> UnixInternalEventChannels {
        UnixInternalEventChannels {
            senders: Arc::new(Mutex::new(vec![])),
        }
    }

    /// Sends an `InternalEvent` to all available channels. Returns `false` if the
    /// list of channels is empty.
    ///
    /// # Notes
    ///
    /// Channel is removed if the receiving end was dropped.
    ///
    fn send(&self, event: InternalEvent) -> bool {
        let mut guard = self.senders.lock().unwrap();
        guard.retain(|sender| sender.send(event.clone()).is_ok());

        !guard.is_empty()
    }

    /// Creates a new `InternalEvent` receiver.
    fn receiver(&self) -> Receiver<InternalEvent> {
        let (tx, rx) = mpsc::channel();

        let mut guard = self.senders.lock().unwrap();
        guard.push(tx);

        rx
    }
}

/// An UNIX `InternalEventProvider` implementation.
pub(crate) struct UnixInternalEventProvider {
    /// A list of channels.
    channels: UnixInternalEventChannels,
    /// A reading thread.
    reading_thread: Option<TtyReadingThread>,
}

impl UnixInternalEventProvider {
    fn new() -> UnixInternalEventProvider {
        UnixInternalEventProvider {
            channels: UnixInternalEventChannels::new(),
            reading_thread: None,
        }
    }
}

impl InternalEventProvider for UnixInternalEventProvider {
    /// Shuts down the reading thread (if exists).
    fn pause(&mut self) {
        // Thread will shutdown on it's own once dropped.
        self.reading_thread = None;
    }

    /// Creates a new `InternalEvent` receiver and spawns a new reading
    /// thread (or reuses the existing one).
    fn receiver(&mut self) -> Receiver<InternalEvent> {
        let rx = self.channels.receiver();

        if self.reading_thread.is_none() {
            let reading_thread = TtyReadingThread::new(self.channels.clone());
            self.reading_thread = Some(reading_thread);
        }

        rx
    }
}

/// A simple standard input (or `/dev/tty`) wrapper for bytes reading or checking
/// if there's an input available.
struct TtyRaw {
    fd: Option<RawFd>,
}

impl TtyRaw {
    fn new() -> TtyRaw {
        let fd = if unsafe { libc::isatty(libc::STDIN_FILENO) == 1 } {
            Some(libc::STDIN_FILENO)
        } else {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
                .ok()
                .map(|f| f.as_raw_fd())
        };

        TtyRaw { fd }
    }

    fn raw_fd(&self) -> Result<RawFd> {
        if let Some(fd) = self.fd {
            return Ok(fd);
        }

        Err(ErrorKind::IoError(io::Error::new(
            io::ErrorKind::Other,
            "Unable to open TTY",
        )))
    }

    /// Reads a single byte.
    fn read(&self) -> Result<u8> {
        let fd = self.raw_fd()?;

        let mut buf: [u8; 1] = [0];
        let read = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 1) };

        if read == -1 {
            Err(ErrorKind::IoError(io::Error::last_os_error()))
        } else {
            Ok(buf[0])
        }
    }

    /// Returns `true` if there's an input available within the given
    /// `timeout`.
    fn select(&self, timeout: Duration) -> Result<bool> {
        let fd = self.raw_fd()?;

        let mut timeout = libc::timeval {
            tv_sec: timeout.as_secs() as libc::time_t,
            tv_usec: timeout.subsec_micros() as libc::suseconds_t,
        };

        let sel = unsafe {
            let mut raw_fd_set = mem::uninitialized::<libc::fd_set>();
            libc::FD_ZERO(&mut raw_fd_set);
            libc::FD_SET(fd, &mut raw_fd_set);

            libc::select(
                1,
                &mut raw_fd_set,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut timeout,
            )
        };

        match sel {
            1 => Ok(true),
            -1 => Err(ErrorKind::IoError(io::Error::last_os_error())),
            _ => Ok(false),
        }
    }
}

/// A stdin (or /dev/tty) reading thread.
///
/// The reading thread will shutdown on it's own once you drop the
/// `TtyReadingThread`.
struct TtyReadingThread {
    /// A signal to shutdown the thread.
    ///
    /// If `load(Ordering::SeqCst)` returns `true`, the thread must exit.
    shutdown: Arc<AtomicBool>,
    /// A reading thread join handle (if exists).
    handle: Option<thread::JoinHandle<()>>,
}

impl TtyReadingThread {
    /// Creates a new `TtyReadingThread`.
    ///
    /// # Arguments
    ///
    /// * `channels` - a list of channels to send all `InternalEvent`s to.
    fn new(channels: UnixInternalEventChannels) -> TtyReadingThread {
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_signal = shutdown.clone();
        let handle = thread::spawn(move || {
            // Be extra careful and avoid unwraps, expects, ... and any kind of panic

            let tty_raw = TtyRaw::new();
            let mut buffer: Vec<u8> = Vec::with_capacity(32);

            // TODO We should use better approach for signalling to avoid unnecessary looping
            loop {
                if let Ok(true) = tty_raw.select(Duration::from_millis(100)) {
                    if let Ok(byte) = tty_raw.read() {
                        buffer.push(byte);

                        let input_available = match tty_raw.select(Duration::from_secs(0)) {
                            Ok(input_available) => input_available,
                            Err(_) => {
                                // select() failed, assume false and continue
                                false
                            }
                        };

                        match parse_event(&buffer, input_available) {
                            // Not enough info to parse the event, wait for more bytes
                            Ok(None) => {}
                            // Clear the input buffer and send the event
                            Ok(Some(event)) => {
                                buffer.clear();

                                if !channels.send(event) {
                                    // TODO This is pretty ugly. Thread should be stopped from outside.
                                    INTERNAL_EVENT_PROVIDER.lock().unwrap().pause();
                                }
                            }
                            // Malformed sequence, clear the buffer
                            Err(_) => buffer.clear(),
                        }
                    }
                }

                if shutdown_signal.load(Ordering::SeqCst) {
                    break;
                }
            }
        });

        TtyReadingThread {
            shutdown,
            handle: Some(handle),
        }
    }
}

impl Drop for TtyReadingThread {
    fn drop(&mut self) {
        // Signal the thread to shutdown
        self.shutdown.store(true, Ordering::SeqCst);

        // Wait for the thread shutdown
        let handle = self.handle.take().unwrap();
        handle.join().unwrap();
    }
}

pub(crate) fn internal_event_receiver() -> Receiver<InternalEvent> {
    INTERNAL_EVENT_PROVIDER.lock().unwrap().receiver()
}

//
// Event parsing
//
// This code (& previous one) are kind of ugly. We have to think about this,
// because it's really not maintainable, no tests, etc.
//
// Every fn returns Result<Option<InputEvent>>
//
// Ok(None) -> wait for more bytes
// Err(_) -> failed to parse event, clear the buffer
// Ok(Some(event)) -> we have event, clear the buffer
//

fn could_not_parse_event_error() -> ErrorKind {
    ErrorKind::IoError(io::Error::new(
        io::ErrorKind::Other,
        "Could not parse an event",
    ))
}

fn parse_event(buffer: &[u8], input_available: bool) -> Result<Option<InternalEvent>> {
    if buffer.is_empty() {
        return Ok(None);
    }

    match buffer[0] {
        b'\x1B' => {
            if buffer.len() == 1 {
                if input_available {
                    // Possible Esc sequence
                    Ok(None)
                } else {
                    Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
                        KeyEvent::Esc,
                    ))))
                }
            } else {
                match buffer[1] {
                    b'O' => {
                        if buffer.len() == 2 {
                            Ok(None)
                        } else {
                            match buffer[2] {
                                // F1-F4
                                val @ b'P'..=b'S' => Ok(Some(InternalEvent::Input(
                                    InputEvent::Keyboard(KeyEvent::F(1 + val - b'P')),
                                ))),
                                _ => Err(could_not_parse_event_error()),
                            }
                        }
                    }
                    b'[' => parse_csi(buffer),
                    b'\x1B' => Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
                        KeyEvent::Esc,
                    )))),
                    _ => parse_utf8_char(buffer),
                }
            }
        }
        b'\r' | b'\n' => Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
            KeyEvent::Enter,
        )))),
        b'\t' => Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
            KeyEvent::Tab,
        )))),
        b'\x7F' => Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
            KeyEvent::Backspace,
        )))),
        c @ b'\x01'..=b'\x1A' => Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
            KeyEvent::Ctrl((c as u8 - 0x1 + b'a') as char),
        )))),
        c @ b'\x1C'..=b'\x1F' => Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
            KeyEvent::Ctrl((c as u8 - 0x1C + b'4') as char),
        )))),
        b'\0' => Ok(Some(InternalEvent::Input(InputEvent::Keyboard(
            KeyEvent::Null,
        )))),
        _ => parse_utf8_char(buffer),
    }
}

fn parse_csi(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    assert!(buffer.starts_with(&[b'\x1B', b'['])); // ESC [

    if buffer.len() == 2 {
        return Ok(None);
    }

    let input_event = match buffer[2] {
        b'[' => {
            if buffer.len() == 3 {
                None
            } else {
                match buffer[3] {
                    // NOTE (@imdaveho): cannot find when this occurs;
                    // having another '[' after ESC[ not a likely scenario
                    val @ b'A'..=b'E' => Some(InputEvent::Keyboard(KeyEvent::F(1 + val - b'A'))),
                    _ => Some(InputEvent::Unknown),
                }
            }
        }
        b'D' => Some(InputEvent::Keyboard(KeyEvent::Left)),
        b'C' => Some(InputEvent::Keyboard(KeyEvent::Right)),
        b'A' => Some(InputEvent::Keyboard(KeyEvent::Up)),
        b'B' => Some(InputEvent::Keyboard(KeyEvent::Down)),
        b'H' => Some(InputEvent::Keyboard(KeyEvent::Home)),
        b'F' => Some(InputEvent::Keyboard(KeyEvent::End)),
        b'Z' => Some(InputEvent::Keyboard(KeyEvent::BackTab)),
        b'M' => return parse_csi_x10_mouse(buffer),
        b'<' => return parse_csi_xterm_mouse(buffer),
        b'0'..=b'9' => {
            // Numbered escape code.
            if buffer.len() == 3 {
                None
            } else {
                // The final byte of a CSI sequence can be in the range 64-126, so
                // let's keep reading anything else.
                let last_byte = *buffer.last().unwrap();
                if last_byte < 64 || last_byte > 126 {
                    None
                } else {
                    match buffer[buffer.len() - 1] {
                        b'M' => return parse_csi_rxvt_mouse(buffer),
                        b'~' => return parse_csi_special_key_code(buffer),
                        b'R' => return parse_csi_cursor_position(buffer),
                        _ => return parse_csi_modifier_key_code(buffer),
                    }
                }
            }
        }
        _ => Some(InputEvent::Unknown),
    };

    Ok(input_event.map(InternalEvent::Input))
}

fn next_parsed<T>(iter: &mut dyn Iterator<Item = &str>) -> Result<T>
where
    T: std::str::FromStr,
{
    iter.next()
        .ok_or_else(|| could_not_parse_event_error())?
        .parse::<T>()
        .map_err(|_| could_not_parse_event_error())
}

fn parse_csi_cursor_position(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    // ESC [ Cy ; Cx R
    //   Cy - cursor row number (starting from 1)
    //   Cx - cursor column number (starting from 1)
    assert!(buffer.starts_with(&[b'\x1B', b'['])); // ESC [
    assert!(buffer.ends_with(&[b'R']));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;

    let mut split = s.split(';');

    let y = next_parsed::<u16>(&mut split)? - 1;
    let x = next_parsed::<u16>(&mut split)? - 1;

    Ok(Some(InternalEvent::CursorPosition(x, y)))
}

fn parse_csi_modifier_key_code(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    assert!(buffer.starts_with(&[b'\x1B', b'['])); // ESC [

    let modifier = buffer[buffer.len() - 2];
    let key = buffer[buffer.len() - 1];

    let input_event = match (modifier, key) {
        (53, 65) => InputEvent::Keyboard(KeyEvent::CtrlUp),
        (53, 66) => InputEvent::Keyboard(KeyEvent::CtrlDown),
        (53, 67) => InputEvent::Keyboard(KeyEvent::CtrlRight),
        (53, 68) => InputEvent::Keyboard(KeyEvent::CtrlLeft),
        (50, 65) => InputEvent::Keyboard(KeyEvent::ShiftUp),
        (50, 66) => InputEvent::Keyboard(KeyEvent::ShiftDown),
        (50, 67) => InputEvent::Keyboard(KeyEvent::ShiftRight),
        (50, 68) => InputEvent::Keyboard(KeyEvent::ShiftLeft),
        _ => InputEvent::Unknown,
    };

    Ok(Some(InternalEvent::Input(input_event)))
}

fn parse_csi_special_key_code(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    assert!(buffer.starts_with(&[b'\x1B', b'['])); // ESC [
    assert!(buffer.ends_with(&[b'~']));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    // This CSI sequence can be a list of semicolon-separated numbers.
    let first = next_parsed::<u8>(&mut split)?;

    if next_parsed::<u8>(&mut split).is_ok() {
        // TODO: handle multiple values for key modifiers (ex: values [3, 2] means Shift+Delete)
        return Ok(Some(InternalEvent::Input(InputEvent::Unknown)));
    }

    let input_event = match first {
        1 | 7 => InputEvent::Keyboard(KeyEvent::Home),
        2 => InputEvent::Keyboard(KeyEvent::Insert),
        3 => InputEvent::Keyboard(KeyEvent::Delete),
        4 | 8 => InputEvent::Keyboard(KeyEvent::End),
        5 => InputEvent::Keyboard(KeyEvent::PageUp),
        6 => InputEvent::Keyboard(KeyEvent::PageDown),
        v @ 11..=15 => InputEvent::Keyboard(KeyEvent::F(v - 10)),
        v @ 17..=21 => InputEvent::Keyboard(KeyEvent::F(v - 11)),
        v @ 23..=24 => InputEvent::Keyboard(KeyEvent::F(v - 12)),
        _ => InputEvent::Unknown,
    };

    Ok(Some(InternalEvent::Input(input_event)))
}

fn parse_csi_rxvt_mouse(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    // rxvt mouse encoding:
    // ESC [ Cb ; Cx ; Cy ; M

    assert!(buffer.starts_with(&[b'\x1B', b'['])); // ESC [
    assert!(buffer.ends_with(&[b'M']));

    let s = std::str::from_utf8(&buffer[2..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    let cb = next_parsed::<u16>(&mut split)?;
    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    let mouse_input_event = match cb {
        32 => MouseEvent::Press(MouseButton::Left, cx, cy),
        33 => MouseEvent::Press(MouseButton::Middle, cx, cy),
        34 => MouseEvent::Press(MouseButton::Right, cx, cy),
        35 => MouseEvent::Release(cx, cy),
        64 => MouseEvent::Hold(cx, cy),
        96 | 97 => MouseEvent::Press(MouseButton::WheelUp, cx, cy),
        _ => MouseEvent::Unknown,
    };

    Ok(Some(InternalEvent::Input(InputEvent::Mouse(
        mouse_input_event,
    ))))
}

fn parse_csi_x10_mouse(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    // X10 emulation mouse encoding: ESC [ M CB Cx Cy (6 characters only).
    // NOTE (@imdaveho): cannot find documentation on this

    assert!(buffer.starts_with(&[b'\x1B', b'[', b'M'])); // ESC [ M

    if buffer.len() < 6 {
        return Ok(None);
    }

    let cb = buffer[3] as i8 - 32;
    // See http://www.xfree86.org/current/ctlseqs.html#Mouse%20Tracking
    // The upper left character position on the terminal is denoted as 1,1.
    // Subtract 1 to keep it synced with cursor
    let cx = buffer[4].saturating_sub(32) as u16 - 1;
    let cy = buffer[5].saturating_sub(32) as u16 - 1;

    let mouse_input_event = match cb & 0b11 {
        0 => {
            if cb & 0x40 != 0 {
                MouseEvent::Press(MouseButton::WheelUp, cx, cy)
            } else {
                MouseEvent::Press(MouseButton::Left, cx, cy)
            }
        }
        1 => {
            if cb & 0x40 != 0 {
                MouseEvent::Press(MouseButton::WheelDown, cx, cy)
            } else {
                MouseEvent::Press(MouseButton::Middle, cx, cy)
            }
        }
        2 => MouseEvent::Press(MouseButton::Right, cx, cy),
        3 => MouseEvent::Release(cx, cy),
        _ => MouseEvent::Unknown,
    };

    Ok(Some(InternalEvent::Input(InputEvent::Mouse(
        mouse_input_event,
    ))))
}

fn parse_csi_xterm_mouse(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    // ESC [ < Cb ; Cx ; Cy (;) (M or m)

    assert!(buffer.starts_with(&[b'\x1B', b'[', b'<'])); // ESC [ <

    if !buffer.ends_with(&[b'm']) && !buffer.ends_with(&[b'M']) {
        return Ok(None);
    }

    let s = std::str::from_utf8(&buffer[3..buffer.len() - 1])
        .map_err(|_| could_not_parse_event_error())?;
    let mut split = s.split(';');

    let cb = next_parsed::<u16>(&mut split)?;

    // See http://www.xfree86.org/current/ctlseqs.html#Mouse%20Tracking
    // The upper left character position on the terminal is denoted as 1,1.
    // Subtract 1 to keep it synced with cursor
    let cx = next_parsed::<u16>(&mut split)? - 1;
    let cy = next_parsed::<u16>(&mut split)? - 1;

    let input_event = match cb {
        0..=2 | 64..=65 => {
            let button = match cb {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                64 => MouseButton::WheelUp,
                65 => MouseButton::WheelDown,
                _ => unreachable!(),
            };
            match buffer.last().unwrap() {
                b'M' => InputEvent::Mouse(MouseEvent::Press(button, cx, cy)),
                b'm' => InputEvent::Mouse(MouseEvent::Release(cx, cy)),
                _ => InputEvent::Unknown,
            }
        }
        32 => InputEvent::Mouse(MouseEvent::Hold(cx, cy)),
        3 => InputEvent::Mouse(MouseEvent::Release(cx, cy)),
        _ => InputEvent::Unknown,
    };

    Ok(Some(InternalEvent::Input(input_event)))
}

fn parse_utf8_char(buffer: &[u8]) -> Result<Option<InternalEvent>> {
    match std::str::from_utf8(buffer) {
        Ok(s) => {
            let event = s
                .chars()
                .next()
                .ok_or_else(|| could_not_parse_event_error())
                .map(KeyEvent::Char)
                .map(InputEvent::Keyboard)
                .map(InternalEvent::Input)?;

            Ok(Some(event))
        }
        Err(_) => {
            // from_utf8 failed, but we have to check if we need more bytes for code point
            // and if all the bytes we have no are valid

            let required_bytes = match buffer[0] {
                // https://en.wikipedia.org/wiki/UTF-8#Description
                (0x00..=0x7F) => 1, // 0xxxxxxx
                (0xC0..=0xDF) => 2, // 110xxxxx 10xxxxxx
                (0xE0..=0xEF) => 3, // 1110xxxx 10xxxxxx 10xxxxxx
                (0xF0..=0xF7) => 4, // 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
                (0x80..=0xBF) | (0xF8..=0xFF) => return Err(could_not_parse_event_error()),
            };

            // More than 1 byte, check them for 10xxxxxx pattern
            if required_bytes > 1 && buffer.len() > 1 {
                for byte in &buffer[1..] {
                    if byte & !0b0011_1111 != 0b1000_0000 {
                        return Err(could_not_parse_event_error());
                    }
                }
            }

            if buffer.len() < required_bytes {
                // All bytes looks good so far, but we need more of them
                Ok(None)
            } else {
                Err(could_not_parse_event_error())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esc_key() {
        assert_eq!(
            parse_event("\x1B".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Esc))),
        );
    }

    #[test]
    fn test_possible_esc_sequence() {
        assert_eq!(parse_event("\x1B".as_bytes(), true).unwrap(), None,);
    }

    #[test]
    fn test_parse_event_subsequent_calls() {
        // The main purpose of this test is to check if we're passing
        // correct slice to other parse_ functions.

        // parse_csi_cursor_position
        assert_eq!(
            parse_event("\x1B[20;10R".as_bytes(), false).unwrap(),
            Some(InternalEvent::CursorPosition(9, 19))
        );

        // parse_csi
        assert_eq!(
            parse_event("\x1B[D".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Left))),
        );

        // parse_csi_modifier_key_code
        assert_eq!(
            parse_event("\x1B[2D".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(
                KeyEvent::ShiftLeft
            ))),
        );

        // parse_csi_special_key_code
        assert_eq!(
            parse_event("\x1B[3~".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Delete))),
        );

        // parse_csi_rxvt_mouse
        assert_eq!(
            parse_event("\x1B[32;30;40;M".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(MouseEvent::Press(
                MouseButton::Left,
                29,
                39
            ))))
        );

        // parse_csi_x10_mouse
        assert_eq!(
            parse_event("\x1B[M0\x60\x70".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(MouseEvent::Press(
                MouseButton::Left,
                63,
                79
            ))))
        );

        // parse_csi_xterm_mouse
        assert_eq!(
            parse_event("\x1B[<0;20;10;M".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(MouseEvent::Press(
                MouseButton::Left,
                19,
                9
            ))))
        );

        // parse_utf8_char
        assert_eq!(
            parse_event("Ž".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Char(
                'Ž'
            )))),
        );
    }

    #[test]
    fn test_parse_event() {
        assert_eq!(
            parse_event("\t".as_bytes(), false).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Tab))),
        );
    }

    #[test]
    fn test_parse_csi_cursor_position() {
        assert_eq!(
            parse_csi_cursor_position("\x1B[20;10R".as_bytes()).unwrap(),
            Some(InternalEvent::CursorPosition(9, 19))
        );
    }

    #[test]
    fn test_parse_csi() {
        assert_eq!(
            parse_csi("\x1B[D".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Left))),
        );
    }

    #[test]
    fn test_parse_csi_modifier_key_code() {
        assert_eq!(
            parse_csi_modifier_key_code("\x1B[2D".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(
                KeyEvent::ShiftLeft
            ))),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code() {
        assert_eq!(
            parse_csi_special_key_code("\x1B[3~".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Delete))),
        );
    }

    #[test]
    fn test_parse_csi_special_key_code_multiple_values_not_supported() {
        assert_eq!(
            parse_csi_special_key_code("\x1B[3;2~".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Unknown)),
        );
    }

    #[test]
    fn test_parse_csi_rxvt_mouse() {
        assert_eq!(
            parse_csi_rxvt_mouse("\x1B[32;30;40;M".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(MouseEvent::Press(
                MouseButton::Left,
                29,
                39
            ))))
        );
    }

    #[test]
    fn test_parse_csi_x10_mouse() {
        assert_eq!(
            parse_csi_x10_mouse("\x1B[M0\x60\x70".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(MouseEvent::Press(
                MouseButton::Left,
                63,
                79
            ))))
        );
    }

    #[test]
    fn test_parse_csi_xterm_mouse() {
        assert_eq!(
            parse_csi_xterm_mouse("\x1B[<0;20;10;M".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(MouseEvent::Press(
                MouseButton::Left,
                19,
                9
            ))))
        );
        assert_eq!(
            parse_csi_xterm_mouse("\x1B[<0;20;10M".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(MouseEvent::Press(
                MouseButton::Left,
                19,
                9
            ))))
        );
        assert_eq!(
            parse_csi_xterm_mouse("\x1B[<0;20;10;m".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(
                MouseEvent::Release(19, 9)
            )))
        );
        assert_eq!(
            parse_csi_xterm_mouse("\x1B[<0;20;10m".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Mouse(
                MouseEvent::Release(19, 9)
            )))
        );
    }

    #[test]
    fn test_utf8() {
        // https://www.php.net/manual/en/reference.pcre.pattern.modifiers.php#54805

        // 'Valid ASCII' => "a",
        assert_eq!(
            parse_utf8_char("a".as_bytes()).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Char(
                'a'
            )))),
        );

        // 'Valid 2 Octet Sequence' => "\xc3\xb1",
        assert_eq!(
            parse_utf8_char(&[0xC3, 0xB1]).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Char(
                'ñ'
            )))),
        );

        // 'Invalid 2 Octet Sequence' => "\xc3\x28",
        assert!(parse_utf8_char(&[0xC3, 0x28]).is_err());

        // 'Invalid Sequence Identifier' => "\xa0\xa1",
        assert!(parse_utf8_char(&[0xA0, 0xA1]).is_err());

        // 'Valid 3 Octet Sequence' => "\xe2\x82\xa1",
        assert_eq!(
            parse_utf8_char(&[0xE2, 0x81, 0xA1]).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Char(
                '\u{2061}'
            )))),
        );

        // 'Invalid 3 Octet Sequence (in 2nd Octet)' => "\xe2\x28\xa1",
        assert!(parse_utf8_char(&[0xE2, 0x28, 0xA1]).is_err());

        // 'Invalid 3 Octet Sequence (in 3rd Octet)' => "\xe2\x82\x28",
        assert!(parse_utf8_char(&[0xE2, 0x82, 0x28]).is_err());

        // 'Valid 4 Octet Sequence' => "\xf0\x90\x8c\xbc",
        assert_eq!(
            parse_utf8_char(&[0xF0, 0x90, 0x8C, 0xBC]).unwrap(),
            Some(InternalEvent::Input(InputEvent::Keyboard(KeyEvent::Char(
                '𐌼'
            )))),
        );

        // 'Invalid 4 Octet Sequence (in 2nd Octet)' => "\xf0\x28\x8c\xbc",
        assert!(parse_utf8_char(&[0xF0, 0x28, 0x8C, 0xBC]).is_err());

        // 'Invalid 4 Octet Sequence (in 3rd Octet)' => "\xf0\x90\x28\xbc",
        assert!(parse_utf8_char(&[0xF0, 0x90, 0x28, 0xBC]).is_err());

        // 'Invalid 4 Octet Sequence (in 4th Octet)' => "\xf0\x28\x8c\x28",
        assert!(parse_utf8_char(&[0xF0, 0x28, 0x8C, 0x28]).is_err());
    }
}
