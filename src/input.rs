//! A module that contains all the actions related to reading input from the terminal.
//! Like reading a line, reading a character and reading asynchronously.

use crossterm_utils::Result;

#[cfg(unix)]
pub use self::unix::{AsyncReader, SyncReader};
#[cfg(windows)]
pub use self::windows::{AsyncReader, SyncReader};

#[cfg(unix)]
pub(crate) mod unix;
#[cfg(windows)]
pub(crate) mod windows;

/// This trait defines the actions that can be performed with the terminal input.
/// This trait can be implemented so that a concrete implementation of the ITerminalInput can fulfill
/// the wishes to work on a specific platform.
///
/// ## For example:
///
/// This trait is implemented for Windows and UNIX systems.
/// Unix is using the 'TTY' and windows is using 'libc' C functions to read the input.
pub(crate) trait Input {
    /// Read one character from the user input
    fn read_char(&self) -> Result<char>;
    /// Read the input asynchronously from the user.
    fn read_async(&self) -> AsyncReader;
    ///  Read the input asynchronously until a certain character is hit.
    fn read_until_async(&self, delimiter: u8) -> AsyncReader;
    /// Read the input synchronously from the user.
    fn read_sync(&self) -> SyncReader;
    /// Start monitoring mouse events.
    fn enable_mouse_mode(&self) -> Result<()>;
    /// Stop monitoring mouse events.
    fn disable_mouse_mode(&self) -> Result<()>;
}
