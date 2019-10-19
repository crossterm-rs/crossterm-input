![Lines of Code][s7] [![Latest Version][s1]][l1] [![MIT][s2]][l2] [![docs][s3]][l3]  [![Join us on Discord][s5]][l5]

# Crossterm Input

**The `crossterm_input` crate is deprecated and no longer maintained. The GitHub repository will
be archived soon. All the code is being moved to the `crossterm`
[crate](https://github.com/crossterm-rs/crossterm). You can learn more in the
[Merge sub-crates to the crossterm crate](https://github.com/crossterm-rs/crossterm/issues/265)
issue.**

This crate allows you to read the user input cross-platform. 
It supports all UNIX and Windows terminals down to Windows 7 (not all terminals are tested
see [Tested Terminals](https://github.com/crossterm-rs/crossterm/blob/master/README.md#tested-terminals) for more info).

`crossterm_input` is a sub-crate of the [crossterm](https://crates.io/crates/crossterm) crate. You can use it
directly, but it's **highly recommended** to use the [crossterm](https://crates.io/crates/crossterm) crate with
the `input` feature enabled.

## Features

- Cross-platform
- Multi-threaded (send, sync)
- Detailed documentation
- Few dependencies
- Input
  - Read character
  - Read line
  - Read key input events (async / sync)
  - Read mouse input events (press, release, position, button)
  - Raw screen

## Getting Started

<details>
<summary>
Click to show Cargo.toml.
</summary>

```toml
[dependencies]
# All crossterm features are enabled by default.
crossterm = "0.11"
```

</details>
<p></p>

```rust
use crossterm::{input, InputEvent, KeyEvent, MouseButton, MouseEvent, RawScreen, Result};

fn main() -> Result<()> {
    // Keep _raw around, raw mode will be disabled on the _raw is dropped
    let _raw = RawScreen::into_raw_mode()?;

    let input = input();
    input.enable_mouse_mode()?;

    let mut sync_stdin = input.read_sync();

    loop {
        if let Some(event) = sync_stdin.next() {
            match event {
                InputEvent::Keyboard(KeyEvent::Esc) => break,
                InputEvent::Keyboard(KeyEvent::Left) => println!("Left arrow"),
                InputEvent::Mouse(MouseEvent::Press(MouseButton::Left, col, row)) => {
                    println!("Left mouse button pressed at {}x{}", col, row);
                }
                _ => println!("Other event {:?}", event),
            }
        }
    }

    input.disable_mouse_mode()
} // <- _raw dropped = raw mode disabled
```

## Other Resources

- [API documentation](https://docs.rs/crossterm_input/) (with other examples)
- [Examples repository](https://github.com/crossterm-rs/examples)

## Authors

* **Timon Post** - *Project Owner & creator*
* **Dave Ho** - *Contributor*

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details

[s1]: https://img.shields.io/crates/v/crossterm_input.svg
[l1]: https://crates.io/crates/crossterm_input

[s2]: https://img.shields.io/badge/license-MIT-blue.svg
[l2]: ./LICENSE

[s3]: https://docs.rs/crossterm_input/badge.svg
[l3]: https://docs.rs/crossterm_input/

[s5]: https://img.shields.io/discord/560857607196377088.svg?logo=discord
[l5]: https://discord.gg/K4nyTDB

[s7]: https://travis-ci.org/crossterm-rs/crossterm.svg?branch=master
