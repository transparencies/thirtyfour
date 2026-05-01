//! End-to-end tests for CDP event subscription via `CdpSession`.
//!
//! Requires both the `manager-tests` and `cdp-events` features:
//!
//! ```text
//! cargo test -p thirtyfour --features manager-tests,cdp-events --test cdp_events -- --test-threads=1
//! ```
//!
//! Each test opens a `CdpSession` over the WebSocket, navigates the
//! WebDriver session, and asserts that a typed event arrives within a
//! generous timeout.

#![cfg(all(feature = "manager-tests", feature = "cdp-events"))]

use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use thirtyfour::ChromeCapabilities;
use thirtyfour::cdp::CdpSession;
use thirtyfour::cdp::domains::{network, page};
use thirtyfour::prelude::*;

const TEST_TIMEOUT: Duration = Duration::from_secs(180);
const EVENT_TIMEOUT: Duration = Duration::from_secs(15);

async fn with_timeout<F, T>(f: F) -> WebDriverResult<T>
where
    F: Future<Output = WebDriverResult<T>>,
{
    tokio::time::timeout(TEST_TIMEOUT, f).await.unwrap_or_else(|_| {
        Err(WebDriverError::FatalError(format!("test exceeded {}s budget", TEST_TIMEOUT.as_secs())))
    })
}

fn chrome_caps() -> ChromeCapabilities {
    let mut caps = DesiredCapabilities::chrome();
    caps.set_headless().unwrap();
    caps.set_no_sandbox().unwrap();
    caps.set_disable_gpu().unwrap();
    caps.set_disable_dev_shm_usage().unwrap();
    caps.add_arg("--no-sandbox").unwrap();
    // Chromedriver only exposes the underlying browser's CDP socket via
    // `goog:chromeOptions.debuggerAddress` when remote debugging is on. With
    // chromedriver 115+ this is set automatically when a session is created,
    // but being explicit makes the test more robust across driver versions.
    caps.add_arg("--remote-debugging-port=0").unwrap();
    caps
}

/// Open a CdpSession against a managed driver. Returns both so the caller
/// keeps the driver alive (`CdpSession` doesn't pin the WebDriver).
async fn open_session() -> WebDriverResult<(WebDriver, CdpSession)> {
    let driver = WebDriver::managed(chrome_caps()).await?;
    let session = driver.cdp().connect().await?;
    Ok((driver, session))
}

/// Spawn a tiny HTTP server that returns a fixed body. Returns the bound
/// `http://127.0.0.1:<port>` URL. Used by the network-event test so we
/// don't depend on external network reachability from CI.
async fn spawn_local_http(body: &'static str) -> WebDriverResult<String> {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .map_err(|e| WebDriverError::FatalError(format!("bind localhost: {e}")))?;
    let addr = listener
        .local_addr()
        .map_err(|e| WebDriverError::FatalError(format!("local_addr: {e}")))?;
    let app = axum::Router::new().route("/", axum::routing::get(move || async move { body }));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(format!("http://127.0.0.1:{}/", addr.port()))
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_events_session_send_browser_get_version() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        // `Browser.getVersion` works at session scope on the page session
        // (commands are dispatched to the browser session by chromium when
        // they don't make sense at the page scope).
        let info = session
            .send(thirtyfour::cdp::domains::browser::GetVersion)
            .await
            .expect("Browser.getVersion via WS");
        assert!(info.user_agent.to_ascii_lowercase().contains("chrome"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_events_subscribe_request_will_be_sent() -> WebDriverResult<()> {
    with_timeout(async {
        let url = spawn_local_http("<html><body>cdp test</body></html>").await?;
        let (driver, session) = open_session().await?;
        session.send(network::Enable::default()).await.expect("Network.enable");
        let mut events = session.subscribe::<network::RequestWillBeSent>();

        // Subscribe first, then navigate — the navigation produces a
        // top-level request that fires `Network.requestWillBeSent`.
        driver.goto(&url).await?;

        let event = tokio::time::timeout(EVENT_TIMEOUT, events.next())
            .await
            .map_err(|_| {
                WebDriverError::Timeout(format!(
                    "no Network.requestWillBeSent within {}s",
                    EVENT_TIMEOUT.as_secs()
                ))
            })?
            .expect("event stream closed prematurely");
        assert!(!event.request_id.as_str().is_empty());
        assert!(!event.document_url.is_empty());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_events_subscribe_lifecycle() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(page::Enable).await.expect("Page.enable");
        session
            .send(page::SetLifecycleEventsEnabled {
                enabled: true,
            })
            .await
            .expect("Page.setLifecycleEventsEnabled");

        let mut events = session.subscribe::<page::LifecycleEvent>();

        // Trigger lifecycle events by navigating.
        driver.goto("data:text/html,<html><body><h1>hello</h1></body></html>").await?;

        // Collect a handful of lifecycle events; assert we see at least one
        // recognisable phase (`init`, `DOMContentLoaded`, or `load`).
        let mut saw_meaningful = false;
        let deadline = tokio::time::Instant::now() + EVENT_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(remaining, events.next()).await {
                Ok(Some(evt)) => {
                    if matches!(evt.name.as_str(), "init" | "DOMContentLoaded" | "load") {
                        saw_meaningful = true;
                        break;
                    }
                }
                _ => break,
            }
        }
        assert!(
            saw_meaningful,
            "no recognisable Page.lifecycleEvent within {}s",
            EVENT_TIMEOUT.as_secs()
        );
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_events_subscribe_all_yields_raw_events() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(page::Enable).await.expect("Page.enable");

        let mut events = session.subscribe_all();

        driver.goto("data:text/html,<html><body>x</body></html>").await?;

        let event = tokio::time::timeout(EVENT_TIMEOUT, events.next())
            .await
            .map_err(|_| {
                WebDriverError::Timeout(format!(
                    "no raw events within {}s",
                    EVENT_TIMEOUT.as_secs()
                ))
            })?
            .expect("event stream closed prematurely");
        assert!(!event.method.is_empty());
        // Should be scoped to the bound session.
        assert_eq!(event.session_id.as_ref(), session.session_id());
        driver.quit().await
    })
    .await
}
