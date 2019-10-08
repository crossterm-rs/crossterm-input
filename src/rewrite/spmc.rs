use crate::InputEvent;
use shrev::{self, EventChannel, ReaderId};
use std::sync::{Arc, LockResult, RwLock, RwLockWriteGuard};

/// Single producer multiple consumers channel (SPMC) for input sharing.
pub(crate) struct InputEventChannel {
    event_channel: Arc<RwLock<EventChannel<InputEvent>>>,
}

impl<'b> InputEventChannel {
    /// Constructs a new spmc `InputEventChannel`.
    pub(crate) fn channel(event_channel: EventChannel<InputEvent>) -> InputEventChannel {
        InputEventChannel {
            event_channel: Arc::new(RwLock::new(event_channel)),
        }
    }

    /// Constructs a new consumer for consuming input events.
    pub(crate) fn new_consumer(&self) -> InputEventConsumer {
        InputEventConsumer::new(self.event_channel.clone())
    }

    /// Tries to acquire the producer that can sent input events to the consumers.
    pub(crate) fn producer<'a>(&mut self) -> ProducerLock<'_> {
        let a = self.event_channel.write();
        ProducerLock::from_lock_result(a)
    }
}

/// The consumer that consumers input events from the producer.
pub(crate) struct InputEventConsumer {
    // TODO: I could't find a way to store the Reader Lock here instead of the whole channel.
    event_channel: Arc<RwLock<EventChannel<InputEvent>>>,
    read_id: ReaderId<InputEvent>,
}

impl InputEventConsumer {
    pub(crate) fn new(event_channel: Arc<RwLock<EventChannel<InputEvent>>>) -> InputEventConsumer {
        InputEventConsumer {
            read_id: event_channel.write().unwrap().register_reader(),
            event_channel: event_channel.clone(),
        }
    }

    /// Returns all available input events for this consumer.
    pub(crate) fn read_all(&mut self) -> Vec<InputEvent> {
        let lock = self
            .event_channel
            .read()
            .expect("Can not acquire read lock");

        lock.read(&mut self.read_id)
            .into_iter()
            .map(|x| x.clone())
            .collect::<Vec<InputEvent>>()
    }
}

/// An acquired write lock to the event channel producer.
pub(crate) struct ProducerLock<'a> {
    lock_result: LockResult<RwLockWriteGuard<'a, EventChannel<InputEvent>>>,
}

impl<'a> ProducerLock<'a> {
    pub(crate) fn from_lock_result(
        lock_result: LockResult<RwLockWriteGuard<'a, EventChannel<InputEvent>>>,
    ) -> ProducerLock<'a> {
        ProducerLock { lock_result }
    }

    pub(crate) fn produce_input_event(&mut self, input_event: InputEvent) {
        self.lock_result
            .as_mut()
            .expect("can not aquire write lock")
            .single_write(input_event);
    }
}

#[cfg(test)]
mod tests {
    use crate::rewrite::input_stream::InputStream;
    use crate::rewrite::spmc::InputEventConsumer;
    use crate::{InputEvent, KeyEvent, MouseEvent};
    use shrev::EventChannel;
    use std::sync::{Arc, RwLock};

    #[test]
    pub fn test_read_all_events() {
        let (channel, mut consumer) = event_consumer();

        let input_events = vec![
            InputEvent::Unsupported(vec![]),
            InputEvent::Unknown,
            InputEvent::Mouse(MouseEvent::Unknown),
            InputEvent::Keyboard(KeyEvent::Up),
        ];

        for event in input_events.iter() {
            channel.write().unwrap().single_write(event.clone());
        }

        assert_eq!(consumer.read_all(), input_events);
    }

    fn event_consumer() -> (Arc<RwLock<EventChannel<InputEvent>>>, InputEventConsumer) {
        let mut channel = Arc::new(RwLock::new(EventChannel::new()));
        (channel.clone(), InputEventConsumer::new(channel))
    }
}
