use crossterm_input::KeyEvent;
use crossterm_input::{EventSource, InputEvent, RawScreen, TTYEventSource};

fn main() {
    let s = RawScreen::into_raw_mode();

    let mut source = TTYEventSource::new();

    while true {
        let event = source.read_event();
        println!("event: {:?}", event);

        if let Ok(Some(InputEvent::Keyboard(KeyEvent::Char('q')))) = event {
            break;
        }
    }
}
