#![forbid(unsafe_code)]

pub mod hello;
pub mod storage;
pub mod mailbox;

pub use hello::handle_hello;
pub use storage::handle_storage_get;
pub use mailbox::handle_mailbox;
