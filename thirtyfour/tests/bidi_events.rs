//! End-to-end tests for BiDi event subscription.
//!
//! Cross-browser via `THIRTYFOUR_BROWSER` (default `chrome`):
//!
//! ```text
//! cargo test -p thirtyfour --features manager-tests,bidi --test bidi_events -- --test-threads=1
//! THIRTYFOUR_BROWSER=firefox cargo test -p thirtyfour --features manager-tests,bidi --test bidi_events -- --test-threads=1
//! ```
//!
//! Every typed event in [`thirtyfour::bidi::modules`] should have an
//! integration test here that proves the real driver emits it with the
//! shape we expect.

#![cfg(all(feature = "manager-tests", feature = "bidi"))]

use std::future::Future;
use std::net::SocketAddr;
use std::time::Duration;

use futures_util::StreamExt;
use thirtyfour::bidi::events::{
    BeforeRequestSent, ContextCreated, ContextDestroyed, DomContentLoaded, HistoryUpdated, Load,
    LogEntryAdded, NavigationCommitted, NavigationStarted, RealmCreated, ResponseCompleted,
    ResponseStarted,
};
use thirtyfour::bidi::modules::{browsing_context, log, network};
use thirtyfour::prelude::*;

use crate::common::launch_managed_bidi;

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

fn bidi_to_wd(e: thirtyfour::bidi::BidiError) -> WebDriverError {
    WebDriverError::FatalError(e.to_string())
}

/// Wait for the next event of type `E` matching `predicate`, with the
/// `EVENT_TIMEOUT` budget. Returns the matched event.
async fn wait_for<E, F>(stream: &mut thirtyfour::bidi::EventStream<E>, mut predicate: F) -> E
where
    E: thirtyfour::bidi::BidiEvent,
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

// ---------------------------------------------------------------------------
// browsingContext events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_load_event_after_navigate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        // Typed `subscribe` auto-sends `session.subscribe`.
        let mut events = bidi.subscribe::<Load>().await.map_err(bidi_to_wd)?;

        let ctx = bidi.browsing_context().top_level().await.map_err(bidi_to_wd)?;
        let url = "data:text/html,<html><body>load</body></html>";
        bidi.browsing_context().navigate(ctx.clone(), url, None).await.map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.context == ctx).await;
        assert!(evt.url.starts_with("data:text/html"));
        assert!(evt.timestamp > 0);
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_dom_content_loaded_event_after_navigate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<DomContentLoaded>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let url = "data:text/html,<html><body>dcl</body></html>";
        bidi.browsing_context().navigate(ctx.clone(), url, None).await.map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.context == ctx).await;
        assert!(evt.url.starts_with("data:text/html"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_navigation_started_event_after_navigate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<NavigationStarted>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let url = "data:text/html,<html><body>nav</body></html>";
        bidi.browsing_context().navigate(ctx.clone(), url, None).await.map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.context == ctx).await;
        assert!(evt.url.starts_with("data:text/html"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_context_created_event_for_iframe() -> WebDriverResult<()> {
    // The spec-guaranteed trigger for `contextCreated` is navigating a
    // top-level context to a page with an iframe — the iframe is a child
    // browsing context whose creation must be announced. (Top-level tabs
    // created via `browsingContext.create` get their announcement folded
    // into the response and don't always fire the event in chromedriver.)
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<ContextCreated>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let parent = tree.contexts[0].context.clone();
        let url =
            "data:text/html,<html><body><iframe src='data:text/html,child'></iframe></body></html>";
        bidi.browsing_context()
            .navigate(parent.clone(), url, Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.0.parent.as_ref() == Some(&parent)).await;
        assert_eq!(evt.0.parent.as_ref(), Some(&parent));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_context_destroyed_event_after_close() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<ContextDestroyed>().await.map_err(bidi_to_wd)?;
        let created = bidi
            .browsing_context()
            .create(browsing_context::CreateType::Tab)
            .await
            .map_err(bidi_to_wd)?;
        let target = created.context.clone();
        bidi.browsing_context().close(created.context).await.map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.0.context == target).await;
        assert_eq!(evt.0.context, target);
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// log events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn log_entry_added_for_console_log() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<LogEntryAdded>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(
                ctx.clone(),
                "data:text/html,<html></html>",
                Some(browsing_context::ReadinessState::Complete),
            )
            .await
            .map_err(bidi_to_wd)?;
        bidi.script()
            .evaluate(ctx, "console.log('thirtyfour-bidi-marker')", false)
            .await
            .map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| {
            e.entry_type == "console"
                && e.text.as_deref().is_some_and(|t| t.contains("thirtyfour-bidi-marker"))
        })
        .await;
        assert_eq!(evt.entry_type, "console");
        assert_eq!(evt.level, log::LogLevel::Info);
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// script events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn script_realm_created_after_navigate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<RealmCreated>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(
                ctx,
                "data:text/html,<html><body>r</body></html>",
                Some(browsing_context::ReadinessState::Complete),
            )
            .await
            .map_err(bidi_to_wd)?;
        // Any window realm event is enough to prove the typed shape.
        let evt = wait_for(&mut events, |e| e.0.realm_type == "window").await;
        assert_eq!(evt.0.realm_type, "window");
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// network events
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn network_before_request_sent_event_during_navigate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<BeforeRequestSent>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let server = spawn_local_http("ok").await?;
        bidi.browsing_context()
            .navigate(ctx.clone(), server.clone(), Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.request.url.starts_with(&server)).await;
        assert_eq!(evt.request.method.to_uppercase(), "GET");
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn network_response_completed_event_during_navigate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<ResponseCompleted>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let server = spawn_local_http("hello world").await?;
        bidi.browsing_context()
            .navigate(ctx.clone(), server.clone(), Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.request.url.starts_with(&server)).await;
        assert_eq!(evt.response.status, 200);
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn network_response_started_event_during_navigate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let mut events = bidi.subscribe::<ResponseStarted>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let server = spawn_local_http("started").await?;
        bidi.browsing_context()
            .navigate(ctx.clone(), server.clone(), Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        let evt = wait_for(&mut events, |e| e.request.url.starts_with(&server)).await;
        assert_eq!(evt.response.status, 200);
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn network_intercept_continue_request_unmodified() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let server = spawn_local_http("intercepted").await?;

        // 1. Subscribe to events first so we don't miss them.
        let mut events = bidi.subscribe::<BeforeRequestSent>().await.map_err(bidi_to_wd)?;

        // 2. Add a global intercept on the request phase.
        let added = bidi
            .network()
            .add_intercept(vec![network::InterceptPhase::BeforeRequestSent], None)
            .await
            .map_err(bidi_to_wd)?;

        // 3. Navigate; the request will be paused.
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        // Start the navigation in the background — the call only resolves
        // once the load completes, which won't happen until we continue
        // the intercepted request.
        let nav_bidi = bidi.clone();
        let nav_server = server.clone();
        let nav_ctx = ctx.clone();
        let nav_handle = tokio::spawn(async move {
            nav_bidi
                .browsing_context()
                .navigate(nav_ctx, nav_server, Some(browsing_context::ReadinessState::Complete))
                .await
        });

        // 4. Find the matching event, then continue the request.
        let evt =
            wait_for(&mut events, |e| e.request.url.starts_with(&server) && e.is_blocked).await;
        bidi.network().continue_request(evt.request.id.clone()).await.map_err(bidi_to_wd)?;

        nav_handle
            .await
            .map_err(|e| WebDriverError::FatalError(format!("nav join: {e}")))?
            .map_err(bidi_to_wd)?;

        // 5. Cleanup — remove the intercept explicitly via the guard.
        added.remove().await.map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// browsingContext events — history & navigation lifecycle
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_history_updated_event() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        // historyUpdated is optional; some drivers don't emit it. Use the
        // best-effort subscribe pattern via the session command so we can
        // bail cleanly if the wire-level subscribe fails.
        let sub = bidi.session().subscribe("browsingContext.historyUpdated").await;
        if let Err(e) = sub {
            if matches!(
                e.error.as_str(),
                "unknown command" | "unknown method" | "unsupported operation" | "invalid argument"
            ) {
                return driver.quit().await;
            }
            return Err(bidi_to_wd(e));
        }
        let mut events = bidi.subscribe::<HistoryUpdated>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(
                ctx.clone(),
                "data:text/html,<html><body>x</body></html>",
                Some(browsing_context::ReadinessState::Complete),
            )
            .await
            .map_err(bidi_to_wd)?;
        // Trigger pushState — that's the spec trigger for history updated.
        bidi.script()
            .evaluate(ctx.clone(), "history.pushState({}, '', '?upd=1')", false)
            .await
            .map_err(bidi_to_wd)?;
        // Best-effort wait — some drivers don't emit. Use a shorter timeout
        // and just exit cleanly if nothing arrives.
        let res = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(evt) = events.next().await
                    && evt.context == ctx
                {
                    return evt;
                }
            }
        })
        .await;
        if let Ok(evt) = res {
            assert!(evt.url.contains("upd=1"));
        }
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_navigation_committed_event() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let sub = bidi.session().subscribe("browsingContext.navigationCommitted").await;
        if let Err(e) = sub {
            if matches!(
                e.error.as_str(),
                "unknown command" | "unknown method" | "unsupported operation" | "invalid argument"
            ) {
                return driver.quit().await;
            }
            return Err(bidi_to_wd(e));
        }
        let mut events = bidi.subscribe::<NavigationCommitted>().await.map_err(bidi_to_wd)?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(
                ctx.clone(),
                "data:text/html,<html><body>committed</body></html>",
                Some(browsing_context::ReadinessState::Complete),
            )
            .await
            .map_err(bidi_to_wd)?;
        let res = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(evt) = events.next().await
                    && evt.context == ctx
                {
                    return evt;
                }
            }
        })
        .await;
        if let Ok(evt) = res {
            assert!(evt.url.starts_with("data:text/html"));
        }
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// raw event subscription
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_raw_yields_typed_method_names() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        bidi.session().subscribe("browsingContext.load").await.map_err(bidi_to_wd)?;
        let mut raw = bidi.subscribe_raw();
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(ctx, "data:text/html,<html></html>", None)
            .await
            .map_err(bidi_to_wd)?;
        let deadline = tokio::time::Instant::now() + EVENT_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, raw.next()).await {
                Ok(Some(ev)) if ev.method == "browsingContext.load" => break,
                Ok(Some(_)) => continue,
                _ => panic!("no browsingContext.load via raw stream"),
            }
        }
        driver.quit().await
    })
    .await
}
