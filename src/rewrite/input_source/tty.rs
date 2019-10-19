use crate::new::sys::InputSource;
use crate::sys::unix::{tty_fd, FileDesc, TtyPoll};
use crate::{InputEvent, InternalEvent, KeyEvent, MouseButton, MouseEvent};
use crossterm_utils::{ErrorKind, Result};
use std::io;
use std::io::Read;
use std::iter::FromIterator;
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::str::FromStr;

pub struct TTYInputSource {
    source: TtyPoll,
}

impl TTYInputSource {
    pub fn new() -> TTYInputSource {
        TTYInputSource::from_file_descriptor(tty_fd().unwrap())
    }

    pub fn from_file_descriptor(input_fd: FileDesc) -> TTYInputSource {
        TTYInputSource {
            source: TtyPoll::new(input_fd),
        }
    }
}

impl InputSource for TTYInputSource {
    fn input_event(&mut self) -> Result<Option<InputEvent>> {
        match self.source.tty_poll() {
            Ok(Some(InternalEvent::Input(event))) => return Ok(Some(event)),
            Ok(Some(InternalEvent::CursorPosition(_, _))) => return Ok(None),
            Ok(None) => Ok(None),
            Err(e) => return Err(e),
        }
    }
}
