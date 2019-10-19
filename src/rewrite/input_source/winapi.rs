use crate::input::windows::read_single_event;
use crate::rewrite::input_source::InputSource;
use crate::InputEvent;
use crossterm_utils::Result;
use crossterm_winapi::{Console, Handle, InputEventType, KeyEventRecord, MouseEvent};

pub struct WinApiInputSource;

impl WinApiInputSource {
    pub fn new() -> WinApiInputSource {
        WinApiInputSource
    }
}

impl InputSource for WinApiInputSource {
    fn input_event(&mut self) -> Result<Option<InputEvent>> {
        read_single_event()
    }
}

impl WinApiInputSource {
    pub fn read_single_event() -> Result<Option<InputEvent>> {
        let console = Console::from(Handle::current_in_handle()?);

        let input = console.read_single_input_event()?;

        match input.event_type {
            InputEventType::KeyEvent => {
                handle_key_event(unsafe { KeyEventRecord::from(*input.event.KeyEvent()) })
            }
            InputEventType::MouseEvent => {
                handle_mouse_event(unsafe { MouseEvent::from(*input.event.MouseEvent()) })
            }
            // NOTE (@imdaveho): ignore below
            InputEventType::WindowBufferSizeEvent => return Ok(None), // TODO implement terminal resize event
            InputEventType::FocusEvent => Ok(None),
            InputEventType::MenuEvent => Ok(None),
        }
    }
}
