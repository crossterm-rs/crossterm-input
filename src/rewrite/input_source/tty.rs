use crate::new::sys::InputSource;
use crate::sys::unix::get_tty_fd;
use crate::{InputEvent, KeyEvent, MouseButton, MouseEvent};
use crossterm_utils::{ErrorKind, Result};
use std::io;
use std::io::Read;
use std::iter::FromIterator;
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::str::FromStr;

pub struct TTYInputSource {
    source: RawFd,
    leftover: Option<u8>,
}

impl TTYInputSource {
    pub fn new() -> TTYInputSource {
        TTYInputSource::from_file_descriptor(get_tty_fd())
    }

    pub fn from_file_descriptor(input_fd: RawFd) -> TTYInputSource {
        TTYInputSource {
            leftover: None,
            source: input_fd,
        }
    }
}

impl InputSource for TTYInputSource {
    fn input_event(&mut self) -> Result<Option<InputEvent>> {
        let source = &mut self.source;

        if let Some(c) = self.leftover {
            // we have a leftover byte, use it
            self.leftover = None;
            return parse_event(c, &mut source.bytes().flatten());
        } else {
            // Here we read two bytes at a time. We need to distinguish between single ESC key presses,
            // and escape sequences (which start with ESC or a x1B byte). The idea is that if this is
            // an escape sequence, we will read multiple bytes (the first byte being ESC) but if this
            // is a single ESC keypress, we will only read a single byte.
            let mut buf = [0u8; 2];

            match source.read(&mut buf) {
                Ok(0) => return Ok(None),
                Ok(1) => match buf[0] {
                    b'\x1B' => return Ok(Some(InputEvent::Keyboard(KeyEvent::Esc))),
                    c => {
                        return parse_event(c, &mut source.bytes().flatten());
                    }
                },
                Ok(2) => {
                    let option_iter = &mut Some(buf[1]).into_iter();
                    let iter = option_iter.map(|c| Ok(c)).chain(source.bytes());

                    match parse_event(buf[0], &mut iter.flatten()) {
                        Ok(event) => {
                            self.leftover = option_iter.next();
                            Ok(event)
                        }
                        Err(e) => Err(e),
                    }
                }
                Ok(_) => unreachable!(),
                Err(e) => return Err(e.into()),
            }
        }
    }
}

/// Parse an Event from `item` and possibly subsequent bytes through `iter`.
pub(crate) fn parse_event<I>(item: u8, iter: &mut I) -> Result<Option<InputEvent>>
where
    I: Iterator<Item = u8>,
{
    let input_event = match item {
        b'\x1B' => {
            let byte = iter.next();
            // This is an escape character, leading a control sequence.
            match byte {
                Some(b'O') => {
                    match iter.next() {
                        // F1-F4
                        Some(val @ b'P'..=b'S') => {
                            InputEvent::Keyboard(KeyEvent::F(1 + val - b'P'))
                        }
                        _ => {
                            return Err(ErrorKind::IoError(io::Error::new(
                                io::ErrorKind::Other,
                                "Could not parse an event",
                            )))
                        }
                    }
                }
                Some(b'[') => {
                    // This is a CSI sequence.
                    parse_csi(iter)
                }
                Some(b'\x1B') => InputEvent::Keyboard(KeyEvent::Esc),
                Some(c) => InputEvent::Keyboard(KeyEvent::Alt(parse_utf8_char(c, iter)?)),
                None => InputEvent::Keyboard(KeyEvent::Esc),
            }
        }
        b'\r' | b'\n' => InputEvent::Keyboard(KeyEvent::Enter),
        b'\t' => InputEvent::Keyboard(KeyEvent::Tab),
        b'\x7F' => InputEvent::Keyboard(KeyEvent::Backspace),
        c @ b'\x01'..=b'\x1A' => InputEvent::Keyboard(KeyEvent::Ctrl((c - 0x1 + b'a') as char)),
        c @ b'\x1C'..=b'\x1F' => InputEvent::Keyboard(KeyEvent::Ctrl((c - 0x1C + b'4') as char)),
        b'\0' => InputEvent::Keyboard(KeyEvent::Null),
        c => InputEvent::Keyboard(KeyEvent::Char(parse_utf8_char(c, iter)?)),
    };

    Ok(Some(input_event))
}

/// Parses a CSI sequence, just after reading ^[
/// Returns Event::Unknown if an unrecognized sequence is found.
/// Most of this parsing code is been taken over from 'termion`.
fn parse_csi<I>(iter: &mut I) -> InputEvent
where
    I: Iterator<Item = u8>,
{
    match iter.next() {
        Some(b'[') => match iter.next() {
            // NOTE (@imdaveho): cannot find when this occurs;
            // having another '[' after ESC[ not a likely scenario
            Some(val @ b'A'..=b'E') => InputEvent::Keyboard(KeyEvent::F(1 + val - b'A')),
            _ => InputEvent::Unknown,
        },
        Some(b'D') => InputEvent::Keyboard(KeyEvent::Left),
        Some(b'C') => InputEvent::Keyboard(KeyEvent::Right),
        Some(b'A') => InputEvent::Keyboard(KeyEvent::Up),
        Some(b'B') => InputEvent::Keyboard(KeyEvent::Down),
        Some(b'H') => InputEvent::Keyboard(KeyEvent::Home),
        Some(b'F') => InputEvent::Keyboard(KeyEvent::End),
        Some(b'Z') => InputEvent::Keyboard(KeyEvent::BackTab),
        Some(b'M') => {
            // X10 emulation mouse encoding: ESC [ CB Cx Cy (6 characters only).
            // NOTE (@imdaveho): cannot find documentation on this
            let mut next = || iter.next().unwrap();

            let cb = next() as i8 - 32;
            // (1, 1) are the coords for upper left.
            let cx = next().saturating_sub(32) as u16;
            let cy = next().saturating_sub(32) as u16;

            InputEvent::Mouse(match cb & 0b11 {
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
            })
        }
        Some(b'<') => {
            // xterm mouse handling:
            // ESC [ < Cb ; Cx ; Cy (;) (M or m)
            let mut buf = Vec::new();
            let mut byte_char = iter.next().unwrap();
            while match byte_char {
                b'm' | b'M' => false,
                _ => true,
            } {
                buf.push(byte_char);
                byte_char = iter.next().unwrap();
            }

            let mut cbi_numbers = split_into_numbers(String::from_utf8(buf).unwrap()).into_iter();
            let button_identifier = cbi_numbers.next().unwrap();
            let mouse_x = cbi_numbers.next().unwrap();
            let mouse_y = cbi_numbers.next().unwrap();

            match button_identifier {
                0..=2 | 64..=65 => {
                    let button = match button_identifier {
                        0 => MouseButton::Left,
                        1 => MouseButton::Middle,
                        2 => MouseButton::Right,
                        64 => MouseButton::WheelUp,
                        65 => MouseButton::WheelDown,
                        _ => unreachable!(),
                    };
                    match byte_char {
                        b'M' => InputEvent::Mouse(MouseEvent::Press(button, mouse_x, mouse_y)),
                        b'm' => InputEvent::Mouse(MouseEvent::Release(mouse_x, mouse_y)),
                        _ => InputEvent::Unknown,
                    }
                }
                32 => InputEvent::Mouse(MouseEvent::Hold(mouse_x, mouse_y)),
                3 => InputEvent::Mouse(MouseEvent::Release(mouse_x, mouse_y)),
                _ => InputEvent::Unknown,
            }
        }
        Some(c @ b'0'..=b'9') => {
            // Numbered escape code.
            let mut buf = Vec::new();
            buf.push(c);
            let mut character = iter.next().unwrap();

            // The final byte of a CSI sequence can be in the range 64-126, so
            // let's keep reading anything else.
            while character < 64 || character > 126 {
                buf.push(character);
                character = iter.next().unwrap();
            }

            match character {
                // rxvt mouse encoding:
                // ESC [ Cb ; Cx ; Cy ; M
                b'M' => {
                    let csi_numbers = split_into_numbers(String::from_utf8(buf).unwrap());

                    let mouse_event_identifier = csi_numbers[0];
                    let mouse_x = csi_numbers[1];
                    let mouse_y = csi_numbers[2];

                    InputEvent::Mouse(match mouse_event_identifier {
                        32 => MouseEvent::Press(MouseButton::Left, mouse_x, mouse_y),
                        33 => MouseEvent::Press(MouseButton::Middle, mouse_x, mouse_y),
                        34 => MouseEvent::Press(MouseButton::Right, mouse_x, mouse_y),
                        35 => MouseEvent::Release(mouse_x, mouse_y),
                        64 => MouseEvent::Hold(mouse_x, mouse_y),
                        96 | 97 => MouseEvent::Press(MouseButton::WheelUp, mouse_x, mouse_y),
                        _ => MouseEvent::Unknown,
                    })
                }
                // Special key code.
                b'~' => {
                    // This CSI sequence can be a list of semicolon-separated numbers.
                    let csi_numbers = split_into_numbers(String::from_utf8(buf).unwrap());

                    if csi_numbers.is_empty() {
                        return InputEvent::Unknown;
                    }

                    // TODO: handle multiple values for key modifiers (ex: values [3, 2] means Shift+Delete)
                    if csi_numbers.len() > 1 {
                        return InputEvent::Unknown;
                    }

                    match csi_numbers[0] {
                        1 | 7 => InputEvent::Keyboard(KeyEvent::Home),
                        2 => InputEvent::Keyboard(KeyEvent::Insert),
                        3 => InputEvent::Keyboard(KeyEvent::Delete),
                        4 | 8 => InputEvent::Keyboard(KeyEvent::End),
                        5 => InputEvent::Keyboard(KeyEvent::PageUp),
                        6 => InputEvent::Keyboard(KeyEvent::PageDown),
                        v @ 11..=15 => InputEvent::Keyboard(KeyEvent::F((v - 10) as u8)),
                        v @ 17..=21 => InputEvent::Keyboard(KeyEvent::F((v - 11) as u8)),
                        v @ 23..=24 => InputEvent::Keyboard(KeyEvent::F((v - 12) as u8)),
                        _ => InputEvent::Unknown,
                    }
                }
                e => match (buf.last().unwrap(), e) {
                    (53, 65) => InputEvent::Keyboard(KeyEvent::CtrlUp),
                    (53, 66) => InputEvent::Keyboard(KeyEvent::CtrlDown),
                    (53, 67) => InputEvent::Keyboard(KeyEvent::CtrlRight),
                    (53, 68) => InputEvent::Keyboard(KeyEvent::CtrlLeft),
                    (50, 65) => InputEvent::Keyboard(KeyEvent::ShiftUp),
                    (50, 66) => InputEvent::Keyboard(KeyEvent::ShiftDown),
                    (50, 67) => InputEvent::Keyboard(KeyEvent::ShiftRight),
                    (50, 68) => InputEvent::Keyboard(KeyEvent::ShiftLeft),
                    _ => InputEvent::Unknown,
                },
            }
        }
        _ => InputEvent::Unknown,
    }
}

fn split_into_numbers(string: String) -> Vec<u16> {
    string
        .split(';')
        .map(|n| n.parse::<u16>().unwrap())
        .collect()
}

/// Parse `c` as either a single byte ASCII char or a variable size UTF-8 char.
fn parse_utf8_char<I>(byte_char: u8, iter: &mut I) -> Result<char>
where
    I: Iterator<Item = u8>,
{
    let error = Err(ErrorKind::IoError(io::Error::new(
        io::ErrorKind::Other,
        "Input character is not valid UTF-8",
    )));

    if byte_char.is_ascii() {
        Ok(byte_char as char)
    } else {
        let mut bytes = Vec::new();
        bytes.push(byte_char);

        while let Some(next) = iter.next() {
            bytes.push(next);
            if let Ok(st) = std::str::from_utf8(&bytes) {
                return Ok(st.chars().next().unwrap());
            }
            if bytes.len() >= 4 {
                return error;
            }
        }

        return error;
    }
}

#[cfg(test)]
mod tests {
    use super::parse_utf8_char;

    #[test]
    fn test_parse_utf8() {
        let st = "abcéŷ¤£€ù%323";
        let ref mut bytes = st.bytes();
        let chars = st.chars();
        for c in chars {
            let b = bytes.next().unwrap();
            assert_eq!(c, parse_utf8_char(b, bytes).unwrap());
        }
    }
}
