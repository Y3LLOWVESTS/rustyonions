/// Fetch raw bytes from overlay given an address + relative path.
///
/// # Errors
///
/// As a stub this never errors and returns an empty vec. When implemented,
/// it will return I/O or protocol errors from the overlay client.
pub fn get_bytes(_addr: &str, _rel: &str) -> anyhow::Result<Vec<u8>> {
    Ok(Vec::new())
}
