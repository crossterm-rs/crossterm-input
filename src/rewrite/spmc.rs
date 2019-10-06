use crate::InputEvent;
use shrev::{self, EventChannel, ReaderId};
use std::sync::{Arc, LockResult, RwLock, RwLockWriteGuard};

/// Single producer multiple consumers channel (SPMC) for input sharing.
pub struct InputEventChannel {
    event_channel: Arc<RwLock<EventChannel<InputEvent>>>,
}

impl<'b> InputEventChannel {
    /// Constructs a new `InputEventChannel`.
    pub fn new(event_channel: EventChannel<InputEvent>) -> InputEventChannel {
        InputEventChannel {
            event_channel: Arc::new(RwLock::new(event_channel)),
        }
    }

    /// Constructs a new consumer for consuming input events.
    pub fn new_consumer(&self) -> InputEventConsumer {
        InputEventConsumer {
            read_id: self.event_channel.write().unwrap().register_reader(),
            event_channel: self.event_channel.clone(),
        }
    }

    /// Tries to acquire the producer that can sent input events to the consumers.
    pub fn producer<'a>(&mut self) -> ProducerLock<'_> {
        let a = self.event_channel.write();
        ProducerLock::from_lock_result(a)
    }
}

/// The consumer that consumers input events from the producer.
pub struct InputEventConsumer {
    // TODO: I could't find a way to store the Reader Lock here instead of the whole channel.
    event_channel: Arc<RwLock<EventChannel<InputEvent>>>,
    read_id: ReaderId<InputEvent>,
}

impl InputEventConsumer {
    /// Returns all available input events for this consumer.
    pub fn read_all(&mut self) -> Vec<InputEvent> {
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

pub struct ProducerLock<'a> {
    lock_result: LockResult<RwLockWriteGuard<'a, EventChannel<InputEvent>>>,
}

impl<'a> ProducerLock<'a> {
    pub fn from_lock_result(
        lock_result: LockResult<RwLockWriteGuard<'a, EventChannel<InputEvent>>>,
    ) -> ProducerLock<'a> {
        ProducerLock { lock_result }
    }

    pub fn produce_input_event(&mut self, input_event: InputEvent) {
        self.lock_result
            .as_mut()
            .expect("can not aquire write lock")
            .single_write(input_event);
    }
}
