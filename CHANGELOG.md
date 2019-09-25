# Version 0.4.1

- Maintenance release only
- Moved to a [separate repository](https://github.com/crossterm-rs/crossterm-input)

# Version 0.4.0

- `TerminalInput::read_line` returns `crossterm::Result` instead of `io::Result`
- `TerminalInput::read_char` returns `crossterm::Result` instead of `io::Result`  
- `Command::get_anis_code()` to `ansi_code()`
- Added KeyEvent::Enter and KeyEvent::Tab: [added-key-event-enter], [added-key-event-tab] 
- `ExecutableCommand::queue` returns `crossterm::Result`
- `QueueableCommand::queue` returns `crossterm::Result`
- Added derives: Serialize/Deserialize for key events [serde]
- Command API takes mutable self instead of self

[added-key-event-tab]: https://github.com/crossterm-rs/crossterm/pull/239
[added-key-event-enter]: https://github.com/crossterm-rs/crossterm/pull/236
[serde]: https://github.com/crossterm-rs/crossterm/pull/190

# Version 0.3.3

- Removed println from `SyncReader`

# Version 0.3.2

- Fixed some special key combination detections for UNIX systems
- Windows mouse input event position was 0-based and should be 1-based

# Version 0.3.1

- Updated crossterm_utils 

# Version 0.3.0

- Removed `TerminalInput::from_output()` 

# Version 0.2.2

- Fixed SyncReade bug.

# Version 0.2.1

- Introduced SyncReader

# Version 0.2.0

- Introduced KeyEvents
- Introduced MouseEvents

# Version 0.1.0

- Moved out of `crossterm` 5.4 crate.
