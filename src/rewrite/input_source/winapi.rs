use crate::input::windows::read_single_event;
use crate::rewrite::input_source::InputSource;
use crate::InputEvent;
use crossterm_utils::Result;

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
