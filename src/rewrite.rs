mod event_iterator;
mod input_pool;
mod input_source;
mod input_stream;
mod spmc;

pub use self::{
    event_iterator::{EventIterator, IntoEventIterator},
    input_source::InputSource,
    input_stream::InputStream,
    spmc::{InputEventChannel, InputEventConsumer},
};

#[cfg(unix)]
pub use input_source::tty::TTYInputSource;
#[cfg(windows)]
pub use input_source::winapi::WinApiInputSource;
