use crate::error::WebDriverResult;
use base64::{Engine, prelude::BASE64_STANDARD};
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::path::Path;
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

/// Process-wide tokio runtime that backs [`block_on`]. We need a stable
/// runtime — one built per call would die at end of call, taking with it
/// the IO drivers of any reqwest::Client (or other tokio-bound resource)
/// constructed inside it. Since callers like the integration test suite
/// cache `WebDriverManager` (and its embedded reqwest client) across
/// `block_on` invocations, a fresh-per-call runtime breaks the second
/// caller. A long-lived runtime, kept driven by a background thread on a
/// `pending` future, lets resources survive between calls.
static GLOBAL_RT: LazyLock<tokio::runtime::Handle> = LazyLock::new(|| {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build block_on runtime");
    let handle = rt.handle().clone();
    thread::spawn(move || {
        rt.block_on(async {
            std::future::pending::<Infallible>().await;
        });
    });
    handle
});

/// Run an async future to completion from synchronous code, returning its
/// output.
///
/// Hidden from rustdoc — kept `pub` only so the crate's own doc tests and
/// integration tests can drive async examples from synchronous bodies.
/// External callers should build their own runtime; this is a thin
/// wrapper over a process-wide `current_thread` tokio runtime.
///
/// # Panics
///
/// Panics if called from a thread that is already inside a tokio runtime,
/// because tokio refuses to nest `block_on` inside another runtime.
#[doc(hidden)]
pub fn block_on<F>(future: F) -> F::Output
where
    F: Future + Send,
    F::Output: Send,
{
    GLOBAL_RT.block_on(future)
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
