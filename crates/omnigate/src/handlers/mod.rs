#![forbid(unsafe_code)]

pub mod hello;
pub mod mailbox;
pub mod storage;

pub use hello::handle_hello;
pub use mailbox::handle_mailbox;
pub use storage::handle_storage_get;
