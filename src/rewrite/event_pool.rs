use std::sync::{LockResult, Mutex, MutexGuard};

use crossterm_utils::Result;
use lazy_static::lazy_static;

use crate::rewrite::event_stream::EventStream;
use crate::rewrite::spmc::EventChannel;
use crate::rewrite::EventSource;
#[cfg(unix)]
use crate::rewrite::TTYEventSource;
#[cfg(windows)]
use crate::rewrite::WinApiEventSource;

lazy_static! {
    /// Static input pool that can be used to read input events.
    pub static ref INPUT: Mutex<EventPool> = { Mutex::new(EventPool::new()) };
}

/// An input pool is a pool that takes care of polling for new input.
/// Before you are able to use the input pool, you have to acquire a lock for it.
/// That prevents race conditions while reading input from certain sources.
pub struct EventPool {
    event_channel: EventChannel,
    event_source: Box<dyn EventSource>,
}

impl EventPool {
    pub(crate) fn new() -> EventPool {
        #[cfg(windows)]
        let input = WinApiEventSource::new();
        #[cfg(unix)]
        let input = TTYEventSource::new();

        EventPool {
            event_source: Box::new(input) as Box<dyn EventSource + Sync + Send>,
            event_channel: EventChannel::channel(shrev::EventChannel::new()),
        }
    }

    /// Acquires the `InputPool`, this can be used when you want mutable access to this pool.
    pub fn lock() -> LockResult<MutexGuard<'static, EventPool>> {
        INPUT.lock()
    }

    /// Changes the default input source to the given input source.
    pub fn set_event_source(&mut self, event_source: Box<dyn EventSource>) {
        self.event_source = event_source;
    }

    /// Returns a input stream that can be used to read input events with.
    pub fn acquire_stream(&self) -> EventStream {
        EventStream::new(self.event_channel.new_consumer())
    }

    /// Polls for input from the underlying input source.
    ///
    /// An input event will be replicated to all consumers aka streams if an input event has occurred.
    /// This poll function will block read for a single key press.
    pub fn poll(&mut self) -> Result<()> {
        // poll for occurred input events
        let event = self.event_source.read_event()?.unwrap();

        // produce the input event for the consumers
        self.event_channel.producer().produce_input_event(event);

        Ok(())
    }

    pub fn enable_mouse_events() {}

    pub fn disable_mouse_events() {}
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use crate::rewrite::event_pool::EventPool;
    use crate::rewrite::event_source::fake::FakeEventSource;
    use crate::{InputEvent, KeyEvent, MouseEvent};

    #[test]
    pub fn test_read_input_multiple_consumers() {
        let mut input_pool = EventPool::lock().unwrap();

        // sender can be used to send fake data, receiver is used to provide the fake input source with input events.
        let (input_sender, input_receiver) = channel();

        // set input source, and sent fake input
        input_pool.set_event_source(Box::new(FakeEventSource::new(input_receiver)));
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
