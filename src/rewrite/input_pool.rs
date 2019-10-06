use crate::rewrite::input_stream::InputStream;
use crate::rewrite::{InputEventChannel, InputSource};
use crossterm_utils::Result;
use lazy_static::lazy_static;

#[cfg(unix)]
use crate::rewrite::TTYInputSource;
#[cfg(windows)]
use crate::rewrite::WinApiInputSource;

use shrev::EventChannel;
use std::sync::{LockResult, Mutex, MutexGuard};

lazy_static! {
    /// Static input pool that can be used to read input events.
    pub static ref INPUT: Mutex<InputPool> = { Mutex::new(InputPool::new()) };
}

/// An input pool is a pool that takes care of polling for new input.
/// Before you are able to use the input pool, you have to acquire a lock for it.
/// That prevents race conditions while reading input from certain sources.
pub struct InputPool {
    event_channel: InputEventChannel,
    input_source: Box<dyn InputSource>,
}

impl InputPool {
    fn new() -> InputPool {
        #[cfg(windows)]
        let input = WinApiInputSource::new();
        #[cfg(unix)]
        let input = TTYInputSource::new();

        InputPool {
            input_source: Box::new(input) as Box<dyn InputSource + Sync + Send>,
            event_channel: InputEventChannel::new(EventChannel::new()),
        }
    }

    /// Acquires the `InputPool`, this can be used when you want mutable access to this pool.
    pub fn lock() -> LockResult<MutexGuard<'static, InputPool>> {
        INPUT.lock()
    }

    /// Changes the default input source to the given input source.
    pub fn set_input_source(&mut self, input_source: Box<InputSource>) {
        self.input_source = input_source;
    }

    /// Returns a input stream that can be used to read input events with.
    pub fn acquire_stream(&self) -> InputStream {
        InputStream::new(self.event_channel.new_consumer())
    }

    /// Polls for input from the underlying input source.
    ///
    /// An input event will be replicated to all consumers aka streams if an input event has occurred.
    /// This poll function will block read for a single key press.
    pub fn poll(&mut self) -> Result<()> {
        // poll for occurred input events
        let input_event = self.input_source.input_event()?.unwrap();

        // produce the input event for the consumers
        self.event_channel
            .producer()
            .produce_input_event(input_event);

        Ok(())
    }

    pub fn enable_mouse_events() {}

    pub fn disable_mouse_events() {}
}

#[cfg(test)]
mod tests {
    use crate::rewrite::input_pool::InputPool;
    use crate::rewrite::input_source::fake::FakeInputSource;
    use crate::{InputEvent, KeyEvent, MouseEvent};
    use std::sync::mpsc::channel;

    #[test]
    pub fn test_input_pool() {
        let mut input_pool = InputPool::lock().unwrap();

        // sender can be used to send fake data, receiver is used to provide the fake input source with input events.
        let (input_sender, input_receiver) = channel();

        // set input source, and sent fake input
        input_pool.set_input_source(Box::new(FakeInputSource::new(input_receiver)));
        input_sender.send(InputEvent::Unknown);

        // acquire consumers
        let mut stream1 = input_pool.acquire_stream();
        let mut stream2 = input_pool.acquire_stream();

        // poll for input
        input_pool.poll().unwrap();

        assert_eq!(stream1.events().next(), Some(InputEvent::Unknown));
        assert_eq!(stream2.events().next(), Some(InputEvent::Unknown));
    }
}
