// TEMPORARY IN THIS PR - IT WONT BE PART OF THE FINAL PR

#![allow(dead_code)]
use std::io::{stdout, Write};

use crossterm_input::{InputEvent, KeyEvent, MouseEvent, RawScreen, Result, TerminalInput};

//
// This is sample `crossterm_cursor::pos_raw` implementation we will have to use.
//
// Once the crates will be merged, `pos_raw` will gain access to `internal_event_receiver()`
// function and thus we can drop `InputEvent::CursorPosition` and use `InternalEvent::CursorPosition`
// to hide this implementation detail from the user.
//
fn pos_raw() -> Result<(u16, u16)> {
    let input = TerminalInput::new();
    let mut reader = input.read_sync();

    // Write command
    let mut stdout = stdout();
    stdout.write_all(b"\x1B[6n")?;
    stdout.flush()?;

    loop {
        if let Some(InputEvent::CursorPosition(x, y)) = reader.next() {
            return Ok((x, y));
        }
    }
}

fn async_test() -> Result<()> {
    let input = TerminalInput::new();
    let _raw = RawScreen::into_raw_mode()?;

    let mut reader = input.read_async();

    input.enable_mouse_mode()?;

    loop {
        if let Some(event) = reader.next() {
            match event {
                InputEvent::Keyboard(KeyEvent::Esc) => break,
                InputEvent::Keyboard(KeyEvent::Char('c')) => println!("Cursor: {:?}", pos_raw()),
                InputEvent::Mouse(mouse) => {
                    match mouse {
                        MouseEvent::Press(_, x, y) => println!("Press: {}x{}", x, y),
                        MouseEvent::Hold(x, y) => println!("Move: {}x{}", x, y),
                        MouseEvent::Release(x, y) => println!("Release: {}x{}", x, y),
                        _ => {}
                    };
                }
                InputEvent::CursorPosition(_, _) => {}
                e => {
                    println!("Event: {:?}", e);
                }
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        println!(".");
    }

    input.disable_mouse_mode()?;
    Ok(())
}

fn sync_test() -> Result<()> {
    let input = TerminalInput::new();
    let _raw = RawScreen::into_raw_mode()?;
    input.enable_mouse_mode()?;

    let mut reader = input.read_sync();
    loop {
        if let Some(event) = reader.next() {
            match event {
                InputEvent::Keyboard(KeyEvent::Esc) => break,
                InputEvent::Keyboard(KeyEvent::Char('c')) => println!("Cursor: {:?}", pos_raw()),
                InputEvent::Mouse(mouse) => {
                    match mouse {
                        MouseEvent::Press(_, x, y) => println!("Press: {}x{}", x, y),
                        MouseEvent::Hold(x, y) => println!("Move: {}x{}", x, y),
                        MouseEvent::Release(x, y) => println!("Release: {}x{}", x, y),
                        _ => {}
                    };
                }
                InputEvent::CursorPosition(_, _) => {}
                e => {
                    println!("Event: {:?}", e);
                }
            };
        }
        println!(".");
    }
    input.disable_mouse_mode()?;
    Ok(())
}

fn main() -> Result<()> {
    // async_test()
    sync_test()
}
