mod event_iterator;
mod input_pool;
mod input_stream;
mod spmc;
mod input_source;

pub use self::{
    event_iterator::{EventIterator, IntoEventIterator},
    input_stream::InputStream,
    spmc::{InputEventChannel, InputEventConsumer},
    input_source::InputSource,
};

#[cfg(unix)]
pub use input_source::tty::TTYInputSource;
#[cfg(windows)]
pub use input_source::winapi::WinApiInputSource;
