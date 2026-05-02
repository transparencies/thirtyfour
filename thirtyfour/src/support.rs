use crate::error::WebDriverResult;
use base64::{Engine, prelude::BASE64_STANDARD};
use std::future::Future;
use std::io;
use std::path::Path;
use std::time::Duration;

/// Run an async future to completion from synchronous code, returning its
/// output.
///
/// Hidden from rustdoc — kept `pub` only so the crate's own doc tests and
/// integration tests can drive async examples from synchronous bodies.
/// External callers should build their own runtime; this is a thin
/// wrapper over `tokio::runtime::Builder::new_current_thread`.
///
/// # Panics
///
/// Panics if called from a thread that is already inside a tokio runtime,
/// because tokio refuses to construct a nested runtime.
#[doc(hidden)]
pub fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build block_on runtime")
        .block_on(future)
}

pub(crate) async fn write_file(
    path: impl AsRef<Path>,
    bytes: impl Into<Vec<u8>>,
) -> io::Result<()> {
    async fn inner(path: &Path, bytes: Vec<u8>) -> io::Result<()> {
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || std::fs::write(path, bytes)).await?
    }

    inner(path.as_ref(), bytes.into()).await
}

/// Helper to sleep asynchronously for the specified duration.
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await
}

/// Convenience wrapper for base64 encoding.
pub fn base64_encode(data: &[u8]) -> String {
    BASE64_STANDARD.encode(data)
}

/// Convenience wrapper for base64 decoding.
pub fn base64_decode(data: &str) -> WebDriverResult<Vec<u8>> {
    let value = BASE64_STANDARD.decode(data)?;
    Ok(value)
}
