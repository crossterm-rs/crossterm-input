use crossterm_utils::Result;
use crossterm_winapi::{Console, Handle, InputEventType, KeyEventRecord, MouseEvent};

use crate::input::windows::read_single_event;
use crate::rewrite::event_source::EventSource;
use crate::InputEvent;

pub struct WinApiEventSource;

impl WinApiEventSource {
    pub fn new() -> WinApiEventSource {
        WinApiEventSource
    }
}

impl EventSource for WinApiEventSource {
    fn read_event(&mut self) -> Result<Option<InputEvent>> {
        read_single_event()
    }
}

impl WinApiEventSource {
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
