use crossterm_utils::Result;
use crate::event_source::EventSource;
use crate::InputEvent;
use crate::sys::winapi::read_single_event;

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
