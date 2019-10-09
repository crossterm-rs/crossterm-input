//! This is a UNIX specific implementation for input related action.

use std::{char, sync::mpsc};

use crossterm_utils::{csi, write_cout, Result};

use crate::sys::unix::event_receiver;
use crate::{input::Input, InputEvent, KeyEvent};

pub(crate) struct UnixInput;

impl UnixInput {
    pub fn new() -> UnixInput {
        UnixInput {}
    }
}

impl Input for UnixInput {
    fn read_char(&self) -> Result<char> {
        let mut reader = self.read_sync();
        loop {
            if let Some(InputEvent::Keyboard(KeyEvent::Char(ch))) = reader.next() {
                return Ok(ch);
            }
        }
    }

    fn read_async(&self) -> AsyncReader {
        AsyncReader::new(None)
    }

    fn read_until_async(&self, delimiter: u8) -> AsyncReader {
        let sentinel = match delimiter {
            b'\n' | b'\r' => Some(KeyEvent::Enter),
            27 => Some(KeyEvent::Esc),
            c if c.is_ascii() => Some(KeyEvent::Char(c as char)),
            _ => None,
        }
        .map(InputEvent::Keyboard);

        AsyncReader::new(sentinel)
    }

    fn read_sync(&self) -> SyncReader {
        SyncReader::new()
    }

    fn enable_mouse_mode(&self) -> Result<()> {
        write_cout!(&format!(
            "{}h{}h{}h{}h",
            csi!("?1000"),
            csi!("?1002"),
            csi!("?1015"),
            csi!("?1006")
        ))?;
        Ok(())
    }

    fn disable_mouse_mode(&self) -> Result<()> {
        write_cout!(&format!(
            "{}l{}l{}l{}l",
            csi!("?1006"),
            csi!("?1015"),
            csi!("?1002"),
            csi!("?1000")
        ))?;
        Ok(())
    }
}

/// An asynchronous input reader (not blocking).
///
/// `AsyncReader` implements the [`Iterator`](https://doc.rust-lang.org/std/iter/index.html#iterator)
/// trait. Documentation says:
///
/// > An iterator has a method, `next`, which when called, returns an `Option<Item>`. `next` will return
/// > `Some(Item)` as long as there are elements, and once they've all been exhausted, will return `None`
/// > to indicate that iteration is finished. Individual iterators may choose to resume iteration, and
/// > so calling `next` again may or may not eventually start returning `Some(Item)` again at some point.
///
/// `AsyncReader` is an individual iterator and it doesn't use `None` to indicate that the iteration is
/// finished. You can expect additional `Some(InputEvent)` after calling `next` even if you have already
/// received `None`.
///
/// # Notes
///
/// * It requires enabled raw mode (see the
///   [`crossterm_screen`](https://docs.rs/crossterm_screen/) crate documentation to learn more).
/// * A thread is spawned/reused to read the input.
/// * The reading thread is cleaned up when you drop the `AsyncReader`.
/// * See the [`SyncReader`](struct.SyncReader.html) if you want a blocking,
///   or a less resource hungry reader.
///
/// # Examples
///
/// ```no_run
/// use std::{thread, time::Duration};
///
/// use crossterm_input::{input, InputEvent, KeyEvent, RawScreen};
///
/// fn main() {
///     println!("Press 'ESC' to quit.");
///
///     // Enable raw mode and keep the `_raw` around otherwise the raw mode will be disabled
///     let _raw = RawScreen::into_raw_mode();
///
///     // Create an input from our screen
///     let input = input();
///
///     // Create an async reader
///     let mut reader = input.read_async();
///
///     loop {
///         if let Some(event) = reader.next() { // Not a blocking call
///             match event {
///                 InputEvent::Keyboard(KeyEvent::Esc) => {
///                     println!("Program closing ...");
///                     break;
///                  }
///                  InputEvent::Mouse(event) => { /* Mouse event */ }
///                  _ => { /* Other events */ }
///             }
///         }
///         thread::sleep(Duration::from_millis(50));
///     }
/// } // `reader` dropped <- thread cleaned up, `_raw` dropped <- raw mode disabled
/// ```
pub struct AsyncReader {
    rx: Option<mpsc::Receiver<InputEvent>>,
    sentinel: Option<InputEvent>,
}

impl AsyncReader {
    /// Creates a new `AsyncReader`.
    ///
    /// # Notes
    ///
    /// * A thread is spawned/reused to read the input.
    /// * The reading thread is cleaned up when you drop the `AsyncReader`.
    fn new(sentinel: Option<InputEvent>) -> AsyncReader {
        AsyncReader {
            rx: Some(event_receiver()),
            sentinel,
        }
    }

    // TODO If we we keep the Drop semantics, do we really need this in the public API? It's useless as
    //      there's no `start`, etc.
    /// Stops the input reader.
    ///
    /// # Notes
    ///
    /// * You don't need to call this method, because it will be automatically called when the
    ///   `AsyncReader` is dropped.
    pub fn stop(&mut self) {
        self.rx = None;
    }
}

impl Iterator for AsyncReader {
    type Item = InputEvent;

    /// Tries to read the next input event (not blocking).
    ///
    /// `None` doesn't mean that the iteration is finished. See the
    /// [`AsyncReader`](struct.AsyncReader.html) documentation for more information.
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(rx) = self.rx.take() {
            let result = match rx.try_recv() {
                Ok(x) => Some(x),
                Err(mpsc::TryRecvError::Empty) => None,
                // TODO 1.0: We should propagate errors here?
                Err(mpsc::TryRecvError::Disconnected) => None,
            };

            if !(self.sentinel.is_some() && self.sentinel == result) {
                // No sentinel reached, put the receiver back
                self.rx = Some(rx);
            }

            result
        } else {
            None
        }
    }
}

/// A synchronous input reader (blocking).
///
/// `SyncReader` implements the [`Iterator`](https://doc.rust-lang.org/std/iter/index.html#iterator)
/// trait. Documentation says:
///
/// > An iterator has a method, `next`, which when called, returns an `Option<Item>`. `next` will return
/// > `Some(Item)` as long as there are elements, and once they've all been exhausted, will return `None`
/// > to indicate that iteration is finished. Individual iterators may choose to resume iteration, and
/// > so calling `next` again may or may not eventually start returning `Some(Item)` again at some point.
///
/// `SyncReader` is an individual iterator and it doesn't use `None` to indicate that the iteration is
/// finished. You can expect additional `Some(InputEvent)` after calling `next` even if you have already
/// received `None`. Unfortunately, `None` means that an error occurred, but you're free to call `next`
/// again. This behavior will be changed in the future to avoid errors consumption.  
///
/// # Notes
///
/// * It requires enabled raw mode (see the
///   [`crossterm_screen`](https://docs.rs/crossterm_screen/) crate documentation to learn more).
/// * See the [`AsyncReader`](struct.AsyncReader.html) if you want a non blocking reader.
///
/// # Examples
///
/// ```no_run
/// use std::{thread, time::Duration};
///
/// use crossterm_input::{input, InputEvent, KeyEvent, RawScreen};
///
/// fn main() {
///     println!("Press 'ESC' to quit.");
///
///     // Enable raw mode and keep the `_raw` around otherwise the raw mode will be disabled
///     let _raw = RawScreen::into_raw_mode();
///
///     // Create an input from our screen
///     let input = input();
///
///     // Create a sync reader
///     let mut reader = input.read_sync();
///
///     loop {
///         if let Some(event) = reader.next() { // Blocking call
///             match event {
///                 InputEvent::Keyboard(KeyEvent::Esc) => {
///                     println!("Program closing ...");
///                     break;
///                  }
///                  InputEvent::Mouse(event) => { /* Mouse event */ }
///                  _ => { /* Other events */ }
///             }
///         }
///         thread::sleep(Duration::from_millis(50));
///     }
/// } // `_raw` dropped <- raw mode disabled
/// ```
pub struct SyncReader {
    rx: mpsc::Receiver<InputEvent>,
}

impl SyncReader {
    fn new() -> SyncReader {
        SyncReader {
            rx: event_receiver(),
        }
    }
}

impl Iterator for SyncReader {
    type Item = InputEvent;

    /// Tries to read the next input event (blocking).
    ///
    /// `None` doesn't mean that the iteration is finished. See the
    /// [`SyncReader`](struct.SyncReader.html) documentation for more information.
    fn next(&mut self) -> Option<Self::Item> {
        match self.rx.recv() {
            Ok(x) => Some(x),
            // TODO 1.0: We should propagate errors here?
            Err(mpsc::RecvError) => None,
        }
    }
}
