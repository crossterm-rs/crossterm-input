use std::io::Read;
use std::error::Error;
use crate::input::winapi::rea

struct WinApiInputSource {

}

impl Read for WinApiInputSource {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let events = read_input_events();
    }
}