//! End-to-end tests for the WebDriver manager.
//!
//! These tests download a real driver binary and spawn a real subprocess, so
//! they're gated behind the `manager-tests` feature and run in their own CI
//! job that does *not* pre-start chromedriver / geckodriver. Run locally with:
//!
//! ```text
//! cargo test -p thirtyfour --features manager-tests --test managed -- --test-threads=1
//! ```

#![cfg(feature = "manager-tests")]

use std::future::Future;
use std::time::Duration;

use thirtyfour::manager::WebDriverManager;
use thirtyfour::prelude::*;
use thirtyfour::{ChromeCapabilities, EdgeCapabilities, FirefoxCapabilities};

/// Per-test wall-clock budget. Real driver downloads on slow CI runners are
/// usually well under a minute; this is a defensive ceiling so a hung test
/// fails-fast rather than burning the runner's full 6h limit.
const TEST_TIMEOUT: Duration = Duration::from_secs(180);

/// Run `f` with a hard timeout. Returns an error rather than hanging.
async fn with_timeout<F, T>(f: F) -> WebDriverResult<T>
where
    F: Future<Output = WebDriverResult<T>>,
{
    tokio::time::timeout(TEST_TIMEOUT, f).await.unwrap_or_else(|_| {
        Err(WebDriverError::FatalError(format!("test exceeded {}s budget", TEST_TIMEOUT.as_secs())))
    })
}

/// Chromium-based browsers (Chrome, Edge) running headless on Windows runners
/// have known reliability issues with `goto`, even for `about:blank` — see
/// `managed_edge_smoke`'s comment on the same bug for Linux Edge. The
/// manager's job is done by the time we'd call `goto`; navigation is the
/// browser engine's responsibility, not ours, so we skip it on the affected
/// platform.
fn skip_navigation() -> bool {
    cfg!(target_os = "windows")
}

fn chrome_caps() -> ChromeCapabilities {
    let mut caps = DesiredCapabilities::chrome();
    caps.set_headless().unwrap();
    caps.set_no_sandbox().unwrap();
    caps.set_disable_gpu().unwrap();
    caps.set_disable_dev_shm_usage().unwrap();
    caps.add_arg("--no-sandbox").unwrap();
    caps
}

fn firefox_caps() -> FirefoxCapabilities {
    let mut caps = DesiredCapabilities::firefox();
    caps.set_headless().unwrap();
    caps
}

fn edge_caps() -> EdgeCapabilities {
    let mut caps = DesiredCapabilities::edge();
    // EdgeCapabilities is Chromium-based and accepts the same args via
    // `ms:edgeOptions.args`. Headless is `--headless=new` for modern Edge.
    caps.add_arg("--headless=new").unwrap();
    caps.add_arg("--no-sandbox").unwrap();
    caps.add_arg("--disable-gpu").unwrap();
    caps.add_arg("--disable-dev-shm-usage").unwrap();
    caps
}

#[tokio::test(flavor = "multi_thread")]
async fn managed_chrome_smoke() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        if !skip_navigation() {
            driver.goto("about:blank").await?;
        }
        driver.quit().await?;
        Ok(())
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn managed_firefox_smoke() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(firefox_caps()).await?;
        driver.goto("about:blank").await?;
        driver.quit().await?;
        Ok(())
    })
    .await
}

/// Edge headless on Ubuntu has a known issue where the renderer times out
/// even on `about:blank`. That's a quirk of Edge-for-Linux's headless mode,
/// not the manager — by the time we reach `goto` the manager has already done
/// its full job (download, spawn, readiness poll, session creation). So we
/// stop short of `goto` here: creating + quitting a session is the
/// manager-level smoke. The full session round-trip is exercised by the
/// existing chrome/firefox smokes.
#[tokio::test(flavor = "multi_thread")]
async fn managed_edge_smoke() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(edge_caps()).await?;
        driver.quit().await?;
        Ok(())
    })
    .await
}

/// Safari is macOS-only and uses the system `safaridriver`. The CI runner must
/// have run `safaridriver --enable` (and have Allow Remote Automation toggled).
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
async fn managed_safari_smoke() -> WebDriverResult<()> {
    with_timeout(async {
        let mut caps = DesiredCapabilities::safari();
        let _ = &mut caps; // safari has no headless / no sandbox toggles
        let driver = WebDriver::managed(caps).await?;
        driver.goto("about:blank").await?;
        driver.quit().await?;
        Ok(())
    })
    .await
}

/// Exercises a few non-default options on Chrome:
///   - a custom cache dir
///   - a custom ready timeout
///   - two `launch()` calls share a single chromedriver process (refcount + dedup)
///
/// We deliberately do NOT exercise `DriverVersion::Latest` here. ChromeDriver's
/// major version must match Chrome's, and CI runners typically have a Chrome a
/// few days behind the absolute latest chromedriver release — so a
/// `Latest`-driven E2E test would be perpetually flaky. The `Latest` resolver
/// itself is covered by the wiremock-based unit tests.
#[tokio::test(flavor = "multi_thread")]
async fn managed_chrome_options_and_dedup() -> WebDriverResult<()> {
    with_timeout(async {
        let cache = tempfile::tempdir().expect("tempdir");
        let mgr = WebDriverManager::builder()
            .match_local() // default; explicit for clarity
            .cache_dir(cache.path().to_path_buf())
            .ready_timeout(Duration::from_secs(60))
            .build();

        let d1 = mgr.launch(chrome_caps()).await?;
        let d2 = mgr.launch(chrome_caps()).await?;

        // Two `launch()` calls keyed on the same (BrowserKind, version, host)
        // should reuse a single chromedriver subprocess — both `WebDriver`
        // instances should point at the same server URL.
        assert_eq!(
            d1.server_url(),
            d2.server_url(),
            "two managed sessions for the same browser must share a chromedriver process"
        );

        if !skip_navigation() {
            d1.goto("about:blank").await?;
            d2.goto("about:blank").await?;
        }

        d1.quit().await?;
        d2.quit().await?;
        Ok(())
    })
    .await
}

/// True if a TCP connect to the driver's `host:port` succeeds, i.e. the
/// chromedriver subprocess is still listening.
async fn driver_is_listening(server_url: &url::Url) -> bool {
    let host = server_url.host_str().unwrap_or("127.0.0.1");
    let port = server_url.port_or_known_default().unwrap_or(0);
    tokio::time::timeout(Duration::from_secs(1), tokio::net::TcpStream::connect((host, port)))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

/// Issue #281: dropping a `WebDriver` without calling `quit()` should still
/// close the session and (for managed drivers) kill the chromedriver
/// subprocess. `SessionHandle::Drop` spawns a dedicated OS thread that
/// runs `quit()` via `support::block_on` (process-wide `GLOBAL_RT`),
/// joining synchronously so Drop blocks until the DELETE /session call
/// completes. `ManagedDriverProcess::Drop` then SIGKILLs the subprocess.
///
/// Tested under both runtime flavors because the cleanup path must not
/// rely on the caller's runtime — the OS thread + GLOBAL_RT trick has to
/// work whether the caller is on `multi_thread` or `current_thread`.
#[tokio::test(flavor = "multi_thread")]
async fn dropping_managed_driver_kills_subprocess_multi_thread() -> WebDriverResult<()> {
    drop_kills_subprocess_inner().await
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_managed_driver_kills_subprocess_current_thread() -> WebDriverResult<()> {
    drop_kills_subprocess_inner().await
}

async fn drop_kills_subprocess_inner() -> WebDriverResult<()> {
    with_timeout(async {
        let server_url = {
            let driver = WebDriver::managed(chrome_caps()).await?;
            let url = driver.server_url().clone();
            assert!(driver_is_listening(&url).await, "driver should be listening while alive");
            if !skip_navigation() {
                driver.goto("about:blank").await?;
            }
            url
            // No explicit `driver.quit().await` — drop fires here, which must
            // run quit() and then SIGKILL the subprocess.
        };

        // Give SIGKILL a moment to actually reap the process.
        tokio::time::sleep(Duration::from_millis(500)).await;

        assert!(
            !driver_is_listening(&server_url).await,
            "chromedriver at {server_url} should be gone after WebDriver was dropped without quit"
        );
        Ok(())
    })
    .await
}

/// Issue #281 specifically: panic unwind drops the in-scope `WebDriver`,
/// which must run the same `quit + kill subprocess` path as a clean drop.
/// Users should never need a custom `Drop` guard with `block_on` — that's
/// what `SessionHandle::Drop` already does internally.
#[tokio::test(flavor = "multi_thread")]
async fn panic_unwind_kills_managed_driver_subprocess() -> WebDriverResult<()> {
    use std::sync::{Arc, Mutex};

    with_timeout(async {
        let captured: Arc<Mutex<Option<url::Url>>> = Arc::new(Mutex::new(None));
        let captured_clone = Arc::clone(&captured);

        // Run the panicking code on a dedicated thread so the unwind happens
        // there and we can `join()` to observe it. The `WebDriver` is held
        // when `panic!` fires; the unwind must drop it cleanly.
        let join = std::thread::spawn(move || {
            thirtyfour::support::block_on(async move {
                let driver = WebDriver::managed(chrome_caps()).await.expect("managed launch");
                *captured_clone.lock().unwrap() = Some(driver.server_url().clone());
                if !skip_navigation() {
                    driver.goto("about:blank").await.expect("goto");
                }
                panic!("simulated user panic with WebDriver still in scope");
            });
        })
        .join();

        assert!(join.is_err(), "thread should have panicked");

        let url = captured
            .lock()
            .unwrap()
            .take()
            .expect("server URL should have been captured before panic");

        tokio::time::sleep(Duration::from_millis(500)).await;

        assert!(
            !driver_is_listening(&url).await,
            "chromedriver at {url} should be gone after panic unwind dropped the WebDriver"
        );
        Ok(())
    })
    .await
}

/// Several `WebDriver`s dropped at the same instant must all clean up.
///
/// Each `SessionHandle::Drop` spawns its own OS thread that calls
/// `support::block_on` (i.e. `Handle::block_on` on the process-wide
/// `GLOBAL_RT`). This test fires N drops concurrently so multiple OS
/// threads all park on `GLOBAL_RT` simultaneously — guards against any
/// regression where the cleanup path serialises through a shared
/// resource and deadlocks when more than one drop is in flight.
///
/// Each `WebDriver::managed` call builds a fresh `WebDriverManager`,
/// so the N sessions back onto N independent chromedriver subprocesses
/// — we can verify each one is gone independently after its drop.
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_drops_kill_all_subprocesses() -> WebDriverResult<()> {
    const N: usize = 3;

    with_timeout(async {
        // Spin up N drivers in parallel. Wrap each builder's `IntoFuture` in
        // an explicit async block so `try_join_all` sees a concrete `Future`.
        let drivers: Vec<WebDriver> = futures_util::future::try_join_all(
            (0..N).map(|_| async { WebDriver::managed(chrome_caps()).await }),
        )
        .await?;

        let urls: Vec<url::Url> = drivers.iter().map(|d| d.server_url().clone()).collect();
        for url in &urls {
            assert!(driver_is_listening(url).await, "driver at {url} should be listening");
        }

        // Drop each driver on its own OS thread so the Drops run concurrently
        // (vs. `drop(Vec)` which would run them sequentially). All cleanup
        // threads call `Handle::block_on` on the shared GLOBAL_RT at once.
        let drop_threads: Vec<_> =
            drivers.into_iter().map(|d| std::thread::spawn(move || drop(d))).collect();
        for t in drop_threads {
            t.join().expect("drop thread panicked");
        }

        // Give SIGKILL a moment to actually reap the processes.
        tokio::time::sleep(Duration::from_millis(500)).await;

        for url in &urls {
            assert!(
                !driver_is_listening(url).await,
                "chromedriver at {url} should be gone after concurrent drop"
            );
        }
        Ok(())
    })
    .await
}
