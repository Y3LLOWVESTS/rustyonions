// Error taxonomy (scaffold)
#[derive(Debug)]
pub enum KmError {
    NoSuchKey,
    SealedCorrupt,
    AlgUnavailable,
    Expired,
    Entropy,
    Backend,
}
pub type KmResult<T> = Result<T, KmError>;
