use crate::rewrite::spmc::InputEventConsumer;
use crate::rewrite::{EventIterator, IntoEventIterator};
use crate::{InputEvent, KeyEvent, MouseEvent};

/// An input stream that can be used to read occurred key events.
pub struct InputStream {
    channel_reader: InputEventConsumer,
    input_cache: Vec<InputEvent>,
}

impl<'a> InputStream {
    /// Constructs a new `InputStream` by passing in the consumer responsible for receiving input events.
    pub(crate) fn new(channel_reader: InputEventConsumer) -> InputStream {
        InputStream {
            channel_reader,
            input_cache: Vec::new(),
        }
    }

    /// Returns an iterator over the pressed `KeyEvent`s.
    pub fn key_events(&mut self) -> EventIterator<KeyEvent> {
        self.update_local_cache();

        self.drain_input_events(|e| match e {
            InputEvent::Keyboard(event) => Some(event.to_owned()),
            _ => None,
        })
        .into_event_iterator()
    }

    /// Returns an iterator over the pressed `MouseEvent`s.
    pub fn mouse_events(&mut self) -> EventIterator<MouseEvent> {
        self.update_local_cache();
        self.drain_input_events(|e| match e {
            InputEvent::Mouse(event) => Some(event.to_owned()),
            _ => None,
        })
        .into_event_iterator()
    }

    /// Returns an iterator over the pressed `InputEvent`s.
    pub fn events(&mut self) -> EventIterator<InputEvent> {
        self.update_local_cache();
        self.drain_input_events(|e| Some(e.to_owned()))
            .into_event_iterator()
    }

    /// Drains input events from the local cache based on the given criteria.
    fn drain_input_events<T>(
        &mut self,
        mut filter: impl FnMut(&InputEvent) -> Option<T>,
    ) -> Vec<T> {
        // TODO: nightly: `Vec::drain_filter`
        let mut drained = Vec::with_capacity(self.input_cache.len());
        let mut i = 0;
        while i != self.input_cache.len() {
            if let Some(event) = filter(&self.input_cache[i]) {
                self.input_cache.remove(i);
                drained.push(event);
            } else {
                i += 1;
            }
        }
        drained
    }

    /// Receives input events from receiver and write them to the local cache.
    fn update_local_cache(&mut self) {
        self.input_cache.extend(self.channel_reader.read_all());
    }
}

#[cfg(test)]
mod tests {
    use crate::rewrite::input_stream::InputStream;
    use crate::{InputEvent, KeyEvent, MouseEvent};

    #[test]
    pub fn test_receive_key_events() {
        let (tx, rx) = unbounded();

        let mut input_stream = InputStream::new(rx);

        tx.send(InputEvent::Keyboard(KeyEvent::Tab));

        assert_eq!(input_stream.key_events().next(), Some(KeyEvent::Tab));
    }

    #[test]
    pub fn test_receive_mouse_events() {
        let (tx, rx) = unbounded();

        let mut input_stream = InputStream::new(rx);

        tx.send(InputEvent::Mouse(MouseEvent::Unknown));

        assert_eq!(
            input_stream.mouse_events().next(),
            Some(MouseEvent::Unknown)
        );
        assert_eq!(input_stream.key_events().next(), None);
        assert_eq!(input_stream.events().next(), None);
    }
}
