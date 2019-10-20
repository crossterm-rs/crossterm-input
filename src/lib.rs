//#![deny(unused_imports, unused_must_use)]

//! # Input
//!
//! The `crossterm_input` crate provides a functionality to read the input events.
//!
//! This documentation does not contain a lot of examples. The reason is that it's fairly
//! obvious how to use this crate. Although, we do provide
//! [examples](https://github.com/crossterm-rs/examples) repository
//! to demonstrate the capabilities.
//!
//! ## Synchronous vs Asynchronous
//!
//! ### Synchronous Reading
//!
//! Read the input synchronously from the user, the reads performed will be blocking calls.
//! Using synchronous over asynchronous reading has the benefit that it is using fewer resources than
//! the asynchronous because background thread and queues are left away.
//!
//! See the [`SyncReader`](struct.SyncReader.html) documentation for more details.
//!
//! ### Asynchronous Reading
//!
//! Read the input asynchronously, input events are gathered in the background and queued for you to read.
//! Using asynchronous reading has the benefit that input events are queued until you read them. You can poll
//! for occurred events, and the reads won't block your program.
//!
//! See the [`AsyncReader`](struct.AsyncReader.html) documentation for more details.
//!
//! ### Technical details
//!
//! On UNIX systems crossterm reads from the TTY, on Windows, it uses `ReadConsoleInputW`.
//! For asynchronous reading, a background thread will be fired up to read input events,
//! occurred events will be queued on an MPSC-channel, and the user can iterate over those events.
//!
//! The terminal has to be in the raw mode, raw mode prevents the input of the user to be displayed
//! on the terminal screen. See the
//! [`crossterm_screen`](https://docs.rs/crossterm_screen/) crate documentation to learn more.

#[doc(no_inline)]
pub use crossterm_screen::{IntoRawMode, RawScreen};
#[doc(no_inline)]
pub use crossterm_utils::Result;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(unix)]
use event_source::tty::TTYEventSource;
#[cfg(windows)]
use event_source::winapi::WinApiEventSource;

pub use self::{
    event_iterator::{EventIterator, IntoEventIterator},
    event_source::EventSource,
    event_stream::EventStream,
    event_pool::EventPool
};

mod sys;

mod event_iterator;
mod event_pool;
mod event_source;
mod event_stream;
mod spmc;



/// Represents an input event.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialOrd, PartialEq, Hash, Clone)]
pub enum InputEvent {
    /// A single key or a combination of keys.
    Keyboard(KeyEvent),
    /// A mouse event.
    Mouse(MouseEvent),
    /// An unsupported event.
    ///
    /// You can ignore this type of event, because it isn't used.
    Unsupported(Vec<u8>), // TODO Not used, should be removed.
    /// An unknown event.
    Unknown,
    /// Internal cursor position event. Don't use it, it will be removed in the
    /// `crossterm` 1.0.
    #[doc(hidden)]
    #[cfg(unix)]
    CursorPosition(u16, u16), // TODO 1.0: Remove
}

/// Represents a mouse event.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialOrd, PartialEq, Hash, Clone, Copy)]
pub enum MouseEvent {
    /// Pressed mouse button at the location (column, row).
    Press(MouseButton, u16, u16),
    /// Released mouse button at the location (column, row).
    Release(u16, u16),
    /// Mouse moved with a pressed left button to the new location (column, row).
    Hold(u16, u16),
    /// An unknown mouse event.
    Unknown,
}

/// Represents a mouse button/wheel.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialOrd, PartialEq, Hash, Clone, Copy)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
    /// Wheel scrolled up.
    WheelUp,
    /// Wheel scrolled down.
    WheelDown,
}

/// Represents a key or a combination of keys.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum KeyEvent {
    /// Backspace key.
    Backspace,
    /// Enter key.
    Enter,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page up key.
    PageUp,
    /// Page dow key.
    PageDown,
    /// Tab key.
    Tab,
    /// Shift + Tab key.
    BackTab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// F key.
    ///
    /// `KeyEvent::F(1)` represents F1 key, etc.
    F(u8),
    /// A character.
    ///
    /// `KeyEvent::Char('c')` represents `c` character, etc.
    Char(char),
    /// Alt key + character.
    ///
    /// `KeyEvent::Alt('c')` represents `Alt + c`, etc.
    Alt(char),
    /// Ctrl key + character.
    ///
    /// `KeyEvent::Ctrl('c') ` represents `Ctrl + c`, etc.
    Ctrl(char),
    /// Null.
    Null,
    /// Escape key.
    Esc,
    /// Ctrl + up arrow key.
    CtrlUp,
    /// Ctrl + down arrow key.
    CtrlDown,
    /// Ctrl + right arrow key.
    CtrlRight,
    /// Ctrl + left arrow key.
    CtrlLeft,
    /// Shift + up arrow key.
    ShiftUp,
    /// Shift + down arrow key.
    ShiftDown,
    /// Shift + right arrow key.
    ShiftRight,
    /// Shift + left arrow key.
    ShiftLeft,
}

/// An internal event.
///
/// Encapsulates publicly available `InputEvent` with additional internal
/// events that shouldn't be publicly available to the crate users.
#[cfg(unix)]
#[derive(Debug, PartialOrd, PartialEq, Hash, Clone)]
pub enum InternalEvent {
    /// An input event.
    Input(InputEvent),
    /// A cursor position (`x`, `y`).
    CursorPosition(u16, u16),
}

/// Converts an `InternalEvent` into a possible `InputEvent`.
#[cfg(unix)]
impl From<InternalEvent> for Option<InputEvent> {
    fn from(ie: InternalEvent) -> Self {
        match ie {
            InternalEvent::Input(input_event) => Some(input_event),
            // TODO 1.0: Swallow `CursorPosition` and return `None`.
            // `cursor::pos_raw()` will be able to use this module `internal_event_receiver()`
            InternalEvent::CursorPosition(x, y) => Some(InputEvent::CursorPosition(x, y)),
        }
    }
}
