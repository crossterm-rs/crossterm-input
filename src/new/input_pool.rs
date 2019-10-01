use std::sync::mpsc::Receiver;
use crate::InputEvent;
use crate::new::input_stream::InputStream;

struct InputPool {
    input_receiver: Receiver<InputEvent>
}

impl InputPool {
    pub fn acquire_stream() -> InputStream {

    }

    pub fn enable_mouse_events() { }

    pub fn disable_mouse_events() { }
}