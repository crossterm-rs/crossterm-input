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
