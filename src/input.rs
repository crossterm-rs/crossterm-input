//! A module that contains all the actions related to reading input from the terminal.
//! Like reading a line, reading a character and reading asynchronously.

use crossterm_utils::Result;

#[cfg(unix)]
pub use self::unix_input::{AsyncReader, SyncReader};
#[cfg(windows)]
pub use self::windows_input::{AsyncReader, SyncReader};

#[cfg(unix)]
pub(crate) mod unix_input;
#[cfg(windows)]
pub(crate) mod windows_input;

/// This trait defines the actions that can be performed with the terminal input.
/// This trait can be implemented so that a concrete implementation of the ITerminalInput can fulfill
/// the wishes to work on a specific platform.
///
/// ## For example:
///
/// This trait is implemented for Windows and UNIX systems.
/// Unix is using the 'TTY' and windows is using 'libc' C functions to read the input.
pub(crate) trait ITerminalInput {
    /// Read one character from the user input
    fn read_char(&self) -> Result<char>;
    /// Read the input asynchronously from the user.
    fn read_async(&self) -> AsyncReader;
    ///  Read the input asynchronously until a certain character is hit.
    fn read_until_async(&self, delimiter: u8) -> AsyncReader;
    /// Read the input synchronously from the user.
    fn read_sync(&self) -> SyncReader;
    fn enable_mouse_mode(&self) -> Result<()>;
    fn disable_mouse_mode(&self) -> Result<()>;
}
