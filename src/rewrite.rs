#[cfg(unix)]
pub use event_source::tty::TTYEventSource;
#[cfg(windows)]
pub use event_source::winapi::WinApiEventSource;

pub use self::{
    event_iterator::{EventIterator, IntoEventIterator},
    event_source::EventSource,
    event_stream::EventStream,
};

mod event_iterator;
mod event_pool;
mod event_source;
mod event_stream;
mod spmc;
