use crate::error::WebDriverResult;
use base64::{Engine, prelude::BASE64_STANDARD};
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;

/// Run `f`; if it panics, abort the process. Used to make the
/// [`GLOBAL_RT`] init and driver-thread paths fail loud rather than
/// silently poisoning the LazyLock (init) or hanging future
/// `block_on` calls (driver). The `Abort` drop-guard ensures the
/// abort runs even via unwind paths.
fn no_unwind<T>(f: impl FnOnce() -> T) -> T {
    let res = std::panic::catch_unwind(AssertUnwindSafe(f));

    res.unwrap_or_else(|_| {
        struct Abort;
        impl Drop for Abort {
            fn drop(&mut self) {
                eprintln!("unrecoverable error reached aborting...");
                std::process::abort()
            }
        }

        let _abort_on_unwind = Abort;
        unreachable!("thirtyfour global runtime panicked")
    })
}

/// Process-wide tokio runtime that backs [`block_on`]. We need a stable
/// runtime — one built per call would die at end of call, taking with it
/// the IO drivers of any reqwest::Client (or other tokio-bound resource)
/// constructed inside it. Since callers like the integration test suite
/// cache `WebDriverManager` (and its embedded reqwest client) across
/// `block_on` invocations, a fresh-per-call runtime breaks the second
/// caller. A long-lived runtime, kept driven by a background thread on a
/// `pending` future, lets resources survive between calls.
static GLOBAL_RT: LazyLock<tokio::runtime::Handle> = LazyLock::new(|| {
    no_unwind(|| {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let handle = rt.handle().clone();

        // Drive the runtime so all calls to `GLOBAL_RT.block_on()` work.
        thread::spawn(move || -> ! {
            async fn forever() -> ! {
                match std::future::pending::<Infallible>().await {}
            }

            no_unwind(move || rt.block_on(forever()))
        });
        handle
    })
});

/// Stack-overflow guard for `block_on`: futures larger than this get
/// `Box::pin`'d before being handed to tokio. Async state machines can
/// be surprisingly large (every local variable across every `.await` is
/// stored inline), and `block_on` is called from `SessionHandle::Drop`
/// — a context where the surrounding stack is already deep and a panic
/// triggers a second unwind. tokio added an internal version of this
/// guard in [PR #6826](https://github.com/tokio-rs/tokio/pull/6826),
/// shipped in tokio 1.40; since we accept tokio >= 1.30, we can't rely
/// on that and apply the guard at our own boundary. The 512-byte
/// threshold matches the tokio PR.
const BOX_FUTURE_THRESHOLD: usize = 512;

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
    // Box large futures to keep them off the stack. See `BOX_FUTURE_THRESHOLD`.
    if size_of::<F>() > BOX_FUTURE_THRESHOLD {
        block_on_inner(Box::pin(future))
    } else {
        block_on_inner(future)
    }
}

fn block_on_inner<F: Future>(future: F) -> F::Output {
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
