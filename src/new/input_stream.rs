use std::sync::mpsc::Receiver;
use crate::new::event_iterator::{KeyEventIterator, MouseEventIterator, EventIterator};
use crate::InputEvent;

pub struct InputStream {
    input_receiver: Receiver<InputEvent>
}

impl InputStream {
    pub fn read_key_events() -> KeyEventIterator {

    }

    pub fn read_mouse_events() -> MouseEventIterator {

    }

    pub fn read_events() -> EventIterator {

    }
}