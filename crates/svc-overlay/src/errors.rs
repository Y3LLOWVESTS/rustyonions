//! RO:WHAT — Error taxonomy
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("protocol: {0}")]
    Protocol(String),
}
