![Lines of Code][s7] [![Latest Version][s1]][l1] [![MIT][s2]][l2] [![docs][s3]][l3]  [![Join us on Discord][s5]][l5]

# Crossterm Input

This crate allows you to read the user input cross-platform. 
It supports all UNIX and Windows terminals down to Windows 7 (not all terminals are tested
see [Tested Terminals](#tested-terminals) for more info)

`crossterm_input` is a sub-crate of the [crossterm](https://crates.io/crates/crossterm) crate. You can use it
directly, but it's **highly recommended** to use the [crossterm](https://crates.io/crates/crossterm) crate with
the `input` feature enabled (see [feature flags](https://crossterm-rs.github.io/crossterm/docs/feature_flags.html)
for more info).

## Future

> The `crossterm_input` crate code will be moved to the `crossterm` crate (it's reexported there now).
> Date is not set yet, but it doesn't make a lot of sense to start a new project with it. Please, use
> the `crossterm` crate with the `input` feature enabled.

Issues in this repository are disabled for the same reason. Please, report all issues in the
[crossterm-rs/crossterm](https://github.com/crossterm-rs/crossterm/issues) repository.

## Table of contents:

- [Getting started](#getting-started)
- [Useful links](#useful-links)
- [Features](#features)
- [Examples](#examples)
- [Tested Terminals](#tested-terminals)
- [Authors](#authors)
- [License](#license)

## Getting Started

All examples of how `crossterm_input` works can be found in the
[examples](https://github.com/crossterm-rs/examples) repository.

Add the `crossterm_input` package to your `Cargo.toml` file.

```
[dependencies]
crossterm_input = "0.4"
```

Import the `crossterm_input` modules you want to use.

```rust  
pub use crossterm_input::{input, AsyncReader, InputEvent, KeyEvent, MouseButton, MouseEvent, SyncReader, TerminalInput};
```

### Useful Links

- [Documentation](https://docs.rs/crossterm_input/)
- [Crates.io](https://crates.io/crates/crossterm_input)
- [Book](https://crossterm-rs.github.io/crossterm/docs/input.html)
- [Examples](https://github.com/crossterm-rs/examples)

## Features

These are the features of this crate:

- Cross-platform
- Multi-threaded (send, sync)
- Detailed Documentation
- Few Dependencies
- Input
    - Read character
    - Read line
    - Read key input events (async / sync)
    - Read mouse input events (press, release, position, button)
    - RawScreen (from `crossterm_screen`)
    
## Examples

The [examples](https://github.com/crossterm-rs/examples) repository has more complete and verbose examples.

_Simple Readings_
```rust 
let mut input = input();

 match input.read_char() {
    Ok(s) => println!("char typed: {}", s),
    Err(e) => println!("char error : {}", e),
 }
 
 match input.read_line() {
     Ok(s) => println!("string typed: {}", s),
     Err(e) => println!("error: {}", e),
 }
```

_Read input events synchronously or asynchronously._

```rust
// make sure to enable raw mode, this will make sure key events won't be handled by the terminal
// it's self and allows crossterm to read the input and pass it back to you.
let screen = RawScreen::into_raw_mode();
    
let mut input = input();

// either read the input synchronously 
let stdin = input.read_sync();
 
// or asynchronously
let stdin = input.read_async();

if let Some(key_event) = stdin.next() {
     match key_event {
         InputEvent::Keyboard(event: KeyEvent) => match event { /* check key event */ }
         InputEvent::Mouse(event: MouseEvent) => match event { /* check mouse event */ }
     }
 }
```

_Enable mouse input events._

```rust
let input = input();

// enable mouse events to be captured.
input.enable_mouse_mode().unwrap();

// disable mouse events to be captured.
input.disable_mouse_mode().unwrap();
```

## Tested terminals

- Windows Powershell
    - Windows 10 (pro)
- Windows CMD
    - Windows 10 (pro)
    - Windows 8.1 (N)
- Ubuntu Desktop Terminal
    - Ubuntu 17.10
- (Arch, Manjaro) KDE Konsole
- Linux Mint

This crate supports all Unix terminals and windows terminals down to Windows 7 but not all of them have been tested.
If you have used this library for a terminal other than the above list without issues feel free to add it to the above list, I really would appreciate it.

## Authors

* **Timon Post** - *Project Owner & creator*
* **Dave Ho** - *Contributor*

## License

This project is licensed under the MIT License - see the [LICENSE.md](./LICENSE) file for details

[s1]: https://img.shields.io/crates/v/crossterm_input.svg
[l1]: https://crates.io/crates/crossterm_input

[s2]: https://img.shields.io/badge/license-MIT-blue.svg
[l2]: ./LICENSE

[s3]: https://docs.rs/crossterm_input/badge.svg
[l3]: https://docs.rs/crossterm_input/

[s5]: https://img.shields.io/discord/560857607196377088.svg?logo=discord
[l5]: https://discord.gg/K4nyTDB

[s7]: https://travis-ci.org/crossterm-rs/crossterm.svg?branch=master
