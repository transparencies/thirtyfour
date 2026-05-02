//! End-to-end tests for CDP event subscription via `CdpSession`.
//!
//! Requires both the `manager-tests` and `cdp-events` features:
//!
//! ```text
//! cargo test -p thirtyfour --features manager-tests,cdp-events --test cdp_events -- --test-threads=1
//! ```
//!
//! Every typed event in [`thirtyfour::cdp::domains`] should have an
//! integration test here that proves chromium actually emits it with the
//! shape we expect.

#![cfg(all(feature = "manager-tests", feature = "cdp-events"))]

use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use thirtyfour::ChromeCapabilities;
use thirtyfour::cdp::CdpSession;
use thirtyfour::cdp::domains::{fetch, log, network, page, runtime, target};
use thirtyfour::prelude::*;

use crate::common::launch_managed_chrome;

mod common;

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
    caps.add_arg("--remote-debugging-port=0").unwrap();
    caps
}

/// Open a CdpSession against a managed driver. Returns both so the caller
/// keeps the driver alive (`CdpSession` doesn't pin the WebDriver).
async fn open_session() -> WebDriverResult<(WebDriver, CdpSession)> {
    let driver = launch_managed_chrome(chrome_caps()).await?;
    let session = driver.cdp().connect().await?;
    Ok((driver, session))
}

/// Spawn a tiny HTTP server that returns a fixed body. Returns the bound
/// `http://127.0.0.1:<port>` URL.
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

/// Wait for the next event of type `E` matching `predicate`, with a
/// `EVENT_TIMEOUT` budget. Returns the matched event.
async fn wait_for<E, F>(stream: &mut thirtyfour::cdp::EventStream<E>, mut predicate: F) -> E
where
    E: thirtyfour::cdp::CdpEvent,
    F: FnMut(&E) -> bool,
{
    let deadline = tokio::time::Instant::now() + EVENT_TIMEOUT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(evt)) if predicate(&evt) => return evt,
            Ok(Some(_)) => continue,
            _ => panic!(
                "no matching {} event within {}s",
                std::any::type_name::<E>(),
                EVENT_TIMEOUT.as_secs()
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Session-scoped command sending (verifies the WS transport's request path)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn session_send_browser_get_version() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
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
async fn subscribe_all_yields_raw_events_for_session() -> WebDriverResult<()> {
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
        assert_eq!(event.session_id.as_ref(), session.session_id());
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// Network events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn network_request_will_be_sent() -> WebDriverResult<()> {
    with_timeout(async {
        let url = spawn_local_http("<html><body>cdp test</body></html>").await?;
        let (driver, session) = open_session().await?;
        session.send(network::Enable::default()).await.expect("Network.enable");
        let mut events = session.subscribe::<network::RequestWillBeSent>();

        driver.goto(&url).await?;

        let event = wait_for(&mut events, |e| e.document_url == url).await;
        assert_eq!(event.document_url, url);
        assert!(!event.request_id.as_str().is_empty());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn network_response_received() -> WebDriverResult<()> {
    with_timeout(async {
        let url = spawn_local_http("<html><body>cdp resp</body></html>").await?;
        let (driver, session) = open_session().await?;
        session.send(network::Enable::default()).await.expect("Network.enable");
        let mut events = session.subscribe::<network::ResponseReceived>();

        driver.goto(&url).await?;

        let event =
            wait_for(&mut events, |e| matches!(e.r#type, network::ResourceType::Document)).await;
        assert!(matches!(event.r#type, network::ResourceType::Document));
        assert!(!event.request_id.as_str().is_empty());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn network_loading_finished() -> WebDriverResult<()> {
    with_timeout(async {
        let url = spawn_local_http("<html><body>cdp finish</body></html>").await?;
        let (driver, session) = open_session().await?;
        session.send(network::Enable::default()).await.expect("Network.enable");
        let mut events = session.subscribe::<network::LoadingFinished>();

        driver.goto(&url).await?;

        let event = wait_for(&mut events, |e| !e.request_id.as_str().is_empty()).await;
        assert!(event.encoded_data_length >= 0.0);
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn network_loading_failed_for_unreachable_url() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(network::Enable::default()).await.expect("Network.enable");
        let mut events = session.subscribe::<network::LoadingFailed>();

        // Inject a fetch to a port that's almost certainly closed. We
        // can't `goto` it because chromedriver waits for navigation; an
        // injected fetch fires the failed-loading event and resolves
        // quickly.
        driver.goto("data:text/html,<html><body>x</body></html>").await?;
        driver.execute("fetch('http://127.0.0.1:1/never').catch(()=>{});", Vec::new()).await?;

        let event = wait_for(&mut events, |_| true).await;
        assert!(!event.error_text.is_empty(), "expected error text on failed loading");
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// Page events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn page_lifecycle_event() -> WebDriverResult<()> {
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
        driver.goto("data:text/html,<html><body><h1>hello</h1></body></html>").await?;

        let event = wait_for(&mut events, |e| {
            matches!(e.name.as_str(), "init" | "DOMContentLoaded" | "load")
        })
        .await;
        assert!(matches!(event.name.as_str(), "init" | "DOMContentLoaded" | "load"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn page_frame_navigated() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(page::Enable).await.expect("Page.enable");
        let mut events = session.subscribe::<page::FrameNavigated>();

        driver.goto("data:text/html,<html><body>frame nav</body></html>").await?;

        let event = wait_for(&mut events, |_| true).await;
        // `frame` is a Value; check the structural shape that matters.
        assert!(event.frame.get("url").is_some());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn page_load_event_fired() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(page::Enable).await.expect("Page.enable");
        let mut events = session.subscribe::<page::LoadEventFired>();

        driver.goto("data:text/html,<html><body>load fired</body></html>").await?;

        let event = wait_for(&mut events, |e| e.timestamp > 0.0).await;
        assert!(event.timestamp > 0.0);
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// Fetch events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn fetch_request_paused_then_continue() -> WebDriverResult<()> {
    with_timeout(async {
        let url = spawn_local_http("<html><body>fetch intercept</body></html>").await?;
        let (driver, session) = open_session().await?;
        session.send(fetch::Enable::default()).await.expect("Fetch.enable");
        let mut events = session.subscribe::<fetch::RequestPaused>();

        // `Fetch` intercepts navigations; this event will fire.
        let nav = {
            let url = url.clone();
            tokio::spawn(async move { driver.goto(&url).await })
        };

        let event =
            wait_for(&mut events, |e| matches!(e.resource_type, network::ResourceType::Document))
                .await;
        assert!(!event.request_id.as_str().is_empty());

        // Let the request through so the navigation completes.
        session
            .send(fetch::ContinueRequest {
                request_id: event.request_id,
                url: None,
                method: None,
                post_data: None,
                headers: None,
            })
            .await
            .expect("Fetch.continueRequest");

        // Reap the navigation task. We don't unwrap because chromedriver's
        // navigation may still be in-flight when the test ends; what matters
        // is that we received the paused event.
        let driver = nav.await.expect("nav join");
        let _ = driver;
        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Log events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn log_entry_added_for_console_error() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(log::Enable).await.expect("Log.enable");
        let mut events = session.subscribe::<log::EntryAdded>();

        // Navigate first, then trigger a deprecation/intervention-style
        // log message reliably. Calling `console.error` doesn't always go
        // through `Log.entryAdded`; loading a malformed inline resource
        // does because Chrome reports it at the network layer.
        driver
            .goto(
                "data:text/html,<html><head><link rel='stylesheet' href='http://127.0.0.1:1/none.css'></head><body>log</body></html>",
            )
            .await?;

        let event = wait_for(&mut events, |_| true).await;
        // Just assert we got something with a non-empty text.
        assert!(!event.entry.text.is_empty());
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// Runtime events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn runtime_console_api_called_for_console_log() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(runtime::Enable).await.expect("Runtime.enable");
        let mut events = session.subscribe::<runtime::ConsoleApiCalled>();

        driver
            .goto(
                "data:text/html,<html><body><script>console.log('cdp-console-marker', 42);</script></body></html>",
            )
            .await?;

        let event = wait_for(&mut events, |e| {
            matches!(e.r#type, runtime::ConsoleApiType::Log)
                && e.args.iter().any(|a| {
                    a.value
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .map(|s| s == "cdp-console-marker")
                        .unwrap_or(false)
                })
        })
        .await;
        assert_eq!(event.r#type, runtime::ConsoleApiType::Log);
        assert!(
            event.args.iter().any(|a| a.r#type == "number"),
            "expected a number arg in {:?}",
            event.args.iter().map(|a| &a.r#type).collect::<Vec<_>>()
        );
        assert!(event.timestamp > 0.0);

        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn runtime_console_api_called_warning_type() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(runtime::Enable).await.expect("Runtime.enable");
        let mut events = session.subscribe::<runtime::ConsoleApiCalled>();

        // CDP reports `console.warn` as `"warning"` (not `"warn"`); make
        // sure the typed enum matches the wire value.
        driver
            .goto(
                "data:text/html,<html><body><script>console.warn('cdp-warn-marker');</script></body></html>",
            )
            .await?;

        let event = wait_for(&mut events, |e| {
            e.args.iter().any(|a| {
                a.value
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .map(|s| s == "cdp-warn-marker")
                    .unwrap_or(false)
            })
        })
        .await;
        assert_eq!(event.r#type, runtime::ConsoleApiType::Warning);

        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn runtime_exception_thrown_for_uncaught_error() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;
        session.send(runtime::Enable).await.expect("Runtime.enable");
        let mut events = session.subscribe::<runtime::ExceptionThrown>();

        driver
            .goto(
                "data:text/html,<html><body><script>setTimeout(()=>{throw new Error('cdp-exception-marker');},0);</script></body></html>",
            )
            .await?;

        let event = wait_for(&mut events, |e| {
            e.exception_details.text.contains("cdp-exception-marker")
                || e.exception_details
                    .exception
                    .as_ref()
                    .and_then(|o| o.description.as_ref())
                    .map(|d| d.contains("cdp-exception-marker"))
                    .unwrap_or(false)
        })
        .await;
        assert!(event.timestamp > 0.0);

        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// Target events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn target_attached_to_target_via_explicit_attach() -> WebDriverResult<()> {
    with_timeout(async {
        let (driver, session) = open_session().await?;

        // Create a sibling tab and explicitly attach to it. The
        // `Target.attachedToTarget` event fires on the session that
        // initiated the attachment.
        let created = session
            .send(target::CreateTarget {
                url: "about:blank".to_string(),
                width: None,
                height: None,
                browser_context_id: None,
                background: None,
            })
            .await
            .expect("Target.createTarget");

        let mut events = session.subscribe::<target::AttachedToTarget>();

        let _ = session
            .send(target::AttachToTarget::flat(created.target_id.clone()))
            .await
            .expect("Target.attachToTarget");

        let event = wait_for(&mut events, |e| e.target_info.target_id == created.target_id).await;
        assert_eq!(event.target_info.target_id, created.target_id);
        assert!(!event.session_id.as_str().is_empty());

        // Detach + close to keep things clean.
        let _ = session
            .send(target::DetachFromTarget {
                session_id: Some(event.session_id),
            })
            .await;
        let _ = session
            .send(target::CloseTarget {
                target_id: created.target_id,
            })
            .await;
        driver.quit().await
    })
    .await
}
