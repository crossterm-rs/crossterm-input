use crate::rewrite::InputSource;
use crate::InputEvent;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;

pub struct FakeInputSource {
    input_receiver: Mutex<Receiver<InputEvent>>,
}

impl FakeInputSource {
    pub fn new(input_receiver: Receiver<InputEvent>) -> FakeInputSource {
        FakeInputSource {
            input_receiver: Mutex::new(input_receiver),
        }
    }
}

impl InputSource for FakeInputSource {
    fn input_event(&mut self) -> crossterm_utils::Result<Option<InputEvent>> {
        let input_receiver = self
            .input_receiver
            .lock()
            .expect("Can't acquire input receiver lock");

        Ok(Some(
            input_receiver
                .recv()
                .expect("Can't receive input from channel"),
        ))
    }
}
