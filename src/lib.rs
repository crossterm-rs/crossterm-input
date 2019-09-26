//! Crossterm provides a way to work with the terminal input. We will not cover the basic usage but instead asynchronous and synchronous reading of input.
//! Please check out these [examples](https://github.com/crossterm-rs/crossterm/blob/master/examples/input.rs) for reading a line or a character from the user.
//!
//! ## Differences Synchronous and Asynchronous
//! Crossterm provides two ways to read user input, synchronous and asynchronous.
//!
//! ### Synchronous reading
//!
//! Read the input synchronously from the user, the reads performed will be blocking calls.
//! Using synchronous over asynchronous reading has the benefit that it is using fewer resources than the asynchronous because background thread and queues are left away.
//!
//! You can get asynchronous event reader by calling: `TerminalInput::read_sync`.
//!
//! ### Asynchronous reading
//!
//! Read the input asynchronously, input events are gathered in the background and will be queued for you to read.
//! Using asynchronous reading has the benefit that input events are queued until you read them. You can poll for occurred events, and the reads won't block your program.
//!
//! You can get a synchronous event reader by calling: `TerminalInput::read_async`, `TerminalInput::read_async_until`.
//!
//! ### Technical details
//! On UNIX systems crossterm reads from the TTY, on Windows, it uses `ReadConsoleInputW`.
//! For asynchronous reading, a background thread will be fired up to read input events,
//! occurred events will be queued on an MPSC-channel, and the user can iterate over those events.
//!
//! The terminal has to be in raw mode, raw mode prevents the input of the user to be displayed on the terminal screen, see [screen](./screen.md) for more info.
//!
//! # Example
//! In the following example, we will create a small program that will listen for mouse and keyboard input.
//! On the press of the 'escape' key, the program will be stopped.
//!
//! So let's start by setting up the basics.
//!
//! ```no_run
//! use std::{thread, time::Duration};
//! use crossterm_input::{input, InputEvent, KeyEvent};
//!
//! fn main() {
//!     println!("Press 'ESC' to quit.");
//!
//!     /* next code here */
//! }
//! ```
//!
//! Next, we need to put the terminal into raw mode. We do this because we don't want the user input to be printed to the terminal screen.
//!
//! ```ignore
//! // enable raw mode
//! let screen = RawScreen::into_raw_mode();
//!
//! // create a input from our screen
//! let input = input();
//!
//! /* next code here */
//! ```
//!
//! Now that we constructed a `TerminalInput` instance we can go ahead an start the reading.
//! Do this by calling `input.read_async()`, which returns an [AsyncReader](https://docs.rs/crossterm/0.8.0/crossterm/struct.AsyncReader.html).
//! This is an iterator over the input events that you could as any other iterator.
//!
//! ```ignore
//! let mut async_stdin = input.read_async();
//!
//! loop {
//!     if let Some(key_event) = async_stdin.next() {
//!         /* next code here */
//!     }
//!     thread::sleep(Duration::from_millis(50));
//! }
//! ```
//!
//! The [AsyncReader](https://docs.rs/crossterm/0.8.0/crossterm/struct.AsyncReader.html) iterator will return `None` when nothing is there to read, `Some(InputEvent)` if there are events to read.
//! I use a thread delay to prevent spamming the iterator.
//!
//! Next up we can start pattern matching to see if there are input events we'd like to catch.
//! In our case, we want to catch the `Escape Key`.
//!
//! ```ignore
//! match key_event {
//!     InputEvent::Keyboard(event) => match event {
//!         KeyEvent::Esc => {
//!             println!("Program closing ...");
//!             break;
//!         }
//!          _ => println!("Key {:?} was pressed!", event)
//!         }
//!     InputEvent::Mouse(event) => { /* Mouse Event */ }
//!     _ => { }
//! }
//! ```
//!
//! As you see, we check if the `KeyEvent::Esc` was pressed, if that's true we stop the program by breaking out of the loop.
//!
//! _final code_
//! ```no_run
//! use std::{thread, time::Duration};
//! use crossterm_input::{input, InputEvent, KeyEvent, RawScreen};
//!
//! fn main() {
//!     println!("Press 'ESC' to quit.");
//!
//!     // enable raw mode
//!     let screen = RawScreen::into_raw_mode();
//!
//!     // create a input from our screen.
//!     let input = input();
//!
//!     // create async reader
//!     let mut async_stdin = input.read_async();
//!
//!     loop {
//!       // try to get the next input event.
//!       if let Some(key_event) = async_stdin.next() {
//!           match key_event {
//!               InputEvent::Keyboard(event) => match event {
//!                   KeyEvent::Esc => {
//!                       println!("Program closing ...");
//!                       break;
//!                   }
//!                   _ => println!("Key {:?} was pressed!", event)
//!                }
//!                InputEvent::Mouse(event) => { /* Mouse Event */ }
//!                _ => { }
//!           }
//!       }
//!       thread::sleep(Duration::from_millis(50));
//!   }
//! } // <=== background reader will be disposed when dropped.s
//! ```
//! ---------------------------------------------------------------------------------------------------------------------------------------------
//! More robust and complete examples on all input aspects like mouse, keys could be found [here](https://github.com/crossterm-rs/crossterm/tree/master/examples/).

#![deny(unused_imports)]

use std::io;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use crossterm_screen::{IntoRawMode, RawScreen};
pub use crossterm_utils::Result;
pub use self::input::{AsyncReader, SyncReader};
use self::input::ITerminalInput;
#[cfg(unix)]
pub use self::input::UnixInput;
#[cfg(windows)]
pub use self::input::WindowsInput;

mod input;
mod sys;

/// Enum to specify which input event has occurred.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialOrd, PartialEq, Hash, Clone)]
pub enum InputEvent {
    /// A single key or a combination is pressed.
    Keyboard(KeyEvent),
    /// A mouse event occurred.
    Mouse(MouseEvent),
    /// A unsupported event has occurred.
    Unsupported(Vec<u8>),
    /// An unknown event has occurred.
    Unknown,
}

/// Enum to specify which mouse event has occurred.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialOrd, PartialEq, Hash, Clone, Copy)]
pub enum MouseEvent {
    /// A mouse press has occurred, this contains the pressed button and the position of the press.
    Press(MouseButton, u16, u16),
    /// A mouse button was released.
    Release(u16, u16),
    /// A mouse button was hold.
    Hold(u16, u16),
    /// An unknown mouse event has occurred.
    Unknown,
}

/// Enum to define mouse buttons.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialOrd, PartialEq, Hash, Clone, Copy)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button
    Middle,
    /// Scroll up
    WheelUp,
    /// Scroll down
    WheelDown,
}

/// Enum with different key or key combinations.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum KeyEvent {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    F(u8),
    Char(char),
    Alt(char),
    Ctrl(char),
    Null,
    Esc,
    CtrlUp,
    CtrlDown,
    CtrlRight,
    CtrlLeft,
    ShiftUp,
    ShiftDown,
    ShiftRight,
    ShiftLeft,
}

/// Allows you to read user input.
///
/// Check [examples](https://github.com/crossterm-rs/examples) in the library for more specific examples.
///
/// ## Examples
///
/// Basic usage:
///
/// ```no_run
/// // You can replace the following line with `use crossterm::TerminalColor;`
/// // if you're using the `crossterm` crate with the `style` feature enabled.
/// use crossterm_input::{Result, TerminalInput, RawScreen};
///
/// fn main() -> Result<()> {
/// let color = TerminalInput::new();
///     // read a single char
///     let char = color.read_char()?;
///     // read a single line
///     let line = color.read_line()?;
///
///     // make sure to enable raw screen when reading input events.
///     let screen = RawScreen::into_raw_mode();
///
///     // create async reader
///     let mut async_stdin = color.read_async();
///
///     // create async reader
///     let mut sync_stdin = color.read_sync();
///
///     // enable mouse input events
///     color.enable_mouse_mode()?;
///     // disable mouse input events
///     color.disable_mouse_mode()
/// }
/// ```
pub struct TerminalInput {
    #[cfg(windows)]
    input: WindowsInput,
    #[cfg(unix)]
    input: UnixInput,
}

impl TerminalInput {
    /// Create a new instance of `TerminalInput` whereon input related actions could be performed.
    pub fn new() -> TerminalInput {
        #[cfg(windows)]
        let input = WindowsInput::new();

        #[cfg(unix)]
        let input = UnixInput::new();

        TerminalInput { input }
    }

    /// Read one line from the user input.
    ///
    /// # Remark
    /// This function is not work when raw screen is turned on.
    /// When you do want to read a line in raw mode please, checkout `read_async`, `read_async_until` or `read_sync`.
    /// Not sure what 'raw mode' is, checkout the 'crossterm_screen' crate.
    ///
    /// # Example
    /// ```no_run
    /// let input = crossterm_input::input();
    /// match input.read_line() {
    ///     Ok(s) => println!("string typed: {}", s),
    ///     Err(e) => println!("error: {}", e),
    /// }
    /// ```
    pub fn read_line(&self) -> Result<String> {
        let mut rv = String::new();
        io::stdin().read_line(&mut rv)?;
        let len = rv.trim_end_matches(&['\r', '\n'][..]).len();
        rv.truncate(len);
        Ok(rv)
    }

    /// Read one character from the user input
    ///
    /// ```no_run
    /// let input = crossterm_input::input();
    /// match input.read_char() {
    ///     Ok(c) => println!("character pressed: {}", c),
    ///     Err(e) => println!("error: {}", e),
    /// }
    /// ```
    pub fn read_char(&self) -> Result<char> {
        self.input.read_char()
    }

    /// Read the input asynchronously, which means that input events are gathered on the background and will be queued for you to read.
    ///
    /// If you want a blocking, or less resource consuming read to happen use `read_sync()`, this will leave a way all the thread and queueing and will be a blocking read.
    ///
    /// This is the same as `read_async()` but stops reading when a certain character is hit.
    ///
    /// # Remarks
    /// - Readings won't be blocking calls.
    ///   A thread will be fired to read input, on unix systems from TTY and on windows WinApi
    ///   `ReadConsoleW` will be used.
    /// - Input events read from the user will be queued on a MPSC-channel.
    /// - The reading thread will be cleaned up when it drops.
    /// - Requires 'raw screen to be enabled'.
    ///   Not sure what this is? Please checkout the 'crossterm_screen' crate.
    ///
    /// # Examples
    /// Please checkout the example folder in the repository.
    pub fn read_async(&self) -> AsyncReader {
        self.input.read_async()
    }

    /// Read the input asynchronously until a certain delimiter (character as byte) is hit, which means that input events are gathered on the background and will be queued for you to read.
    ///
    /// If you want a blocking or less resource consuming read to happen, use `read_sync()`. This will leave alone the background thread and queues and will be a blocking read.
    ///
    /// This is the same as `read_async()` but stops reading when a certain character is hit.
    ///
    /// # Remarks
    /// - Readings won't be blocking calls.
    ///   A thread will be fired to read input, on unix systems from TTY and on windows WinApi
    ///   `ReadConsoleW` will be used.
    /// - Input events read from the user will be queued on a MPSC-channel.
    /// - The reading thread will be cleaned up when it drops.
    /// - Requires 'raw screen to be enabled'.
    ///   Not sure what this is? Please checkout the 'crossterm_screen' crate.
    ///
    /// # Examples
    /// Please checkout the example folder in the repository.
    pub fn read_until_async(&self, delimiter: u8) -> AsyncReader {
        self.input.read_until_async(delimiter)
    }

    /// Read the input synchronously from the user, which means that reading calls will block.
    /// It also uses less resources than the `AsyncReader` because the background thread and queues are left alone.
    ///
    /// Consider using `read_async` if you don't want the reading call to block your program.
    ///
    /// # Remark
    /// - Readings will be blocking calls.
    ///
    /// # Examples
    /// Please checkout the example folder in the repository.
    pub fn read_sync(&self) -> SyncReader {
        self.input.read_sync()
    }

    /// Enable mouse events to be captured.
    ///
    /// When enabling mouse input, you will be able to capture mouse movements, pressed buttons, and locations.
    ///
    /// # Remark
    /// - Mouse events will be send over the reader created with `read_async`, `read_async_until`, `read_sync`.
    pub fn enable_mouse_mode(&self) -> Result<()> {
        self.input.enable_mouse_mode()
    }

    /// Disable mouse events to be captured.
    ///
    /// When disabling mouse input, you won't be able to capture mouse movements, pressed buttons, and locations anymore.
    pub fn disable_mouse_mode(&self) -> Result<()> {
        self.input.disable_mouse_mode()
    }
}

/// Get a `TerminalInput` instance whereon input related actions can be performed.
pub fn input() -> TerminalInput {
    TerminalInput::new()
}
