//! End-to-end tests for the typed WebDriver BiDi API.
//!
//! BiDi is a W3C cross-browser protocol; this suite runs against either
//! chromedriver or geckodriver, selected by `THIRTYFOUR_BROWSER`
//! (default `chrome`). The driver is downloaded by the manager, so these
//! tests gate behind the `manager-tests` feature.
//!
//! ```text
//! cargo test -p thirtyfour --features manager-tests,bidi --test bidi_typed -- --test-threads=1
//! THIRTYFOUR_BROWSER=firefox cargo test -p thirtyfour --features manager-tests,bidi --test bidi_typed -- --test-threads=1
//! ```
//!
//! Every typed command in [`thirtyfour::bidi::modules`] should have at
//! least one test here. The point is to verify the wire format against a
//! real BiDi-capable driver — unit tests of the JSON shape are
//! guess-against-guess and don't tell you anything the driver doesn't.

#![cfg(all(feature = "manager-tests", feature = "bidi"))]

use std::future::Future;
use std::time::Duration;

use thirtyfour::bidi::modules::{browser, browsing_context, network, script, storage};
use thirtyfour::prelude::*;

use crate::common::launch_managed_bidi;

mod common;

const TEST_TIMEOUT: Duration = Duration::from_secs(180);

async fn with_timeout<F, T>(f: F) -> WebDriverResult<T>
where
    F: Future<Output = WebDriverResult<T>>,
{
    tokio::time::timeout(TEST_TIMEOUT, f).await.unwrap_or_else(|_| {
        Err(WebDriverError::FatalError(format!("test exceeded {}s budget", TEST_TIMEOUT.as_secs())))
    })
}

const BLANK_HTML: &str = "data:text/html,<html><head><title>BiDi%20test</title></head>\
<body><h1>x</h1><input id=t /></body></html>";

/// Convert a `BidiError` (returned by `BiDi::send`) into a `WebDriverError`
/// so tests can use `?` from inside `with_timeout`.
fn bidi_to_wd(e: thirtyfour::bidi::BidiError) -> WebDriverError {
    WebDriverError::FatalError(e.to_string())
}

// ---------------------------------------------------------------------------
// session
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn session_status_round_trip() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let status = bidi.session().status().await.map_err(bidi_to_wd)?;
        // Once we already have an active session the driver reports
        // `ready: false`, but `message` is always present.
        assert!(!status.message.is_empty());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn session_subscribe_unsubscribe_round_trip() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        bidi.session().subscribe("browsingContext.load").await.map_err(bidi_to_wd)?;
        bidi.session()
            .unsubscribe(["browsingContext.load".to_string()])
            .await
            .map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// browsingContext
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_get_tree() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        assert!(!tree.contexts.is_empty(), "expected at least one top-level context");
        let root = &tree.contexts[0];
        // `parent` is null for top-level contexts. Top-level URL is
        // `about:blank` immediately after a fresh session.
        assert!(root.parent.is_none());
        assert!(!root.context.as_str().is_empty());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_navigate_complete() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let context = tree.contexts[0].context.clone();
        let nav = bidi
            .browsing_context()
            .navigate(context.clone(), BLANK_HTML, Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        assert!(nav.url.starts_with("data:text/html"));
        // `wait: complete` always assigns a navigation id.
        assert!(nav.navigation.is_some());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_create_and_close() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let created = bidi
            .browsing_context()
            .create(browsing_context::CreateType::Tab)
            .await
            .map_err(bidi_to_wd)?;
        assert!(!created.context.as_str().is_empty());
        // Confirm it shows up in the tree.
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        assert!(tree.contexts.iter().any(|c| c.context == created.context));
        bidi.browsing_context().close(created.context.clone()).await.map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_capture_screenshot_returns_png_b64() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(ctx.clone(), BLANK_HTML, Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        let shot = bidi.browsing_context().capture_screenshot(ctx).await.map_err(bidi_to_wd)?;
        assert!(!shot.data.is_empty());
        // PNG base64 always starts with `iVBOR…`.
        assert!(
            shot.data.starts_with("iVBOR"),
            "unexpected screenshot prefix: {}",
            &shot.data[..16]
        );
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_set_viewport_then_capture() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(ctx.clone(), BLANK_HTML, Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        bidi.browsing_context()
            .set_viewport(
                ctx.clone(),
                Some(browsing_context::Viewport {
                    width: 320,
                    height: 240,
                }),
            )
            .await
            .map_err(bidi_to_wd)?;
        // Verify the viewport actually changed via window.innerWidth.
        let result =
            bidi.script().evaluate(ctx, "window.innerWidth", false).await.map_err(bidi_to_wd)?;
        let v = result.ok_value().expect("expected success result");
        assert_eq!(v["value"].as_i64(), Some(320));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_activate() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        // Smoke test — activate doesn't error on a single visible tab.
        bidi.browsing_context().activate(ctx).await.map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_traverse_history() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(
                ctx.clone(),
                "data:text/html,<html><body>a</body></html>",
                Some(browsing_context::ReadinessState::Complete),
            )
            .await
            .map_err(bidi_to_wd)?;
        bidi.browsing_context()
            .navigate(
                ctx.clone(),
                "data:text/html,<html><body>b</body></html>",
                Some(browsing_context::ReadinessState::Complete),
            )
            .await
            .map_err(bidi_to_wd)?;
        // Step back. Driver returns Empty on success.
        bidi.browsing_context().traverse_history(ctx, -1).await.map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_reload() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(ctx.clone(), BLANK_HTML, Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        // geckodriver 0.36 doesn't yet implement the `ignoreCache` param
        // ("unsupported operation"). Send the bare reload — wait condition
        // only — so the test exercises the command shape on both browsers.
        bidi.send(browsing_context::Reload {
            context: ctx,
            ignore_cache: None,
            wait: Some(browsing_context::ReadinessState::Complete),
        })
        .await
        .map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn browsing_context_handle_user_prompt_dismiss() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(
                ctx.clone(),
                "data:text/html,<html><body><script>setTimeout(() => alert('hi'), 0);</script></body></html>",
                Some(browsing_context::ReadinessState::Interactive),
            )
            .await
            .map_err(bidi_to_wd)?;
        // The alert may or may not fire before this call depending on
        // timing. Try once; if there's no prompt the driver returns
        // "no such alert" — accept either outcome to keep the test
        // non-flaky.
        let outcome = bidi
            .browsing_context()
            .handle_user_prompt(ctx, Some(false), None)
            .await;
        match outcome {
            Ok(_) => {}
            Err(e) if e.error == "no such alert" => {}
            Err(e) => return Err(bidi_to_wd(e)),
        }
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// script
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn script_evaluate_simple_expression() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let r = bidi.script().evaluate(ctx, "1 + 2", false).await.map_err(bidi_to_wd)?;
        let v = r.ok_value().expect("success");
        assert_eq!(v["type"], "number");
        assert_eq!(v["value"].as_i64(), Some(3));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn script_evaluate_await_promise() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let r = bidi
            .script()
            .evaluate(
                ctx,
                "new Promise(res => setTimeout(() => res(42), 1))",
                /* await_promise */ true,
            )
            .await
            .map_err(bidi_to_wd)?;
        let v = r.ok_value().expect("success");
        assert_eq!(v["value"].as_i64(), Some(42));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn script_evaluate_returns_exception_on_throw() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let r = bidi
            .script()
            .evaluate(ctx, "throw new Error('boom')", false)
            .await
            .map_err(bidi_to_wd)?;
        assert!(r.is_exception());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn script_call_function_returns_value() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        let r = bidi
            .script()
            .call_function(ctx, "() => 'thirtyfour'", false)
            .await
            .map_err(bidi_to_wd)?;
        let v = r.ok_value().expect("success");
        assert_eq!(v["value"].as_str(), Some("thirtyfour"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn script_get_realms_lists_window_realm() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let r = bidi.script().get_realms().await.map_err(bidi_to_wd)?;
        // At least the top-level window realm.
        assert!(r.realms.iter().any(|info| info.realm_type == "window"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn script_add_remove_preload_script() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let added = bidi
            .script()
            .add_preload_script("() => { window.__thirtyfour_preload = 1; }")
            .await
            .map_err(bidi_to_wd)?;
        assert!(!added.script.as_str().is_empty());
        // Navigate so the preload runs.
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(ctx.clone(), BLANK_HTML, Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        let r = bidi
            .script()
            .evaluate(ctx, "window.__thirtyfour_preload", false)
            .await
            .map_err(bidi_to_wd)?;
        let v = r.ok_value().expect("success");
        assert_eq!(v["value"].as_i64(), Some(1));
        bidi.script().remove_preload_script(added.script).await.map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn script_evaluate_in_realm_target() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let realms = bidi.script().get_realms().await.map_err(bidi_to_wd)?;
        let realm =
            realms.realms.into_iter().find(|r| r.realm_type == "window").expect("window realm");
        let r = bidi
            .send(script::Evaluate {
                expression: "navigator.userAgent".to_string(),
                target: script::Target::Realm {
                    realm: realm.realm,
                },
                await_promise: false,
                result_ownership: None,
                user_activation: None,
            })
            .await
            .map_err(bidi_to_wd)?;
        let v = r.ok_value().expect("success");
        let ua = v["value"].as_str().unwrap_or("");
        // Just assert the realm-targeted call returned a non-empty
        // user-agent string — the exact contents are browser-specific.
        assert!(!ua.is_empty(), "expected a non-empty user agent string");
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// network — non-event commands.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn network_add_remove_intercept() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let added = bidi
            .network()
            .add_intercept(vec![network::InterceptPhase::BeforeRequestSent], None)
            .await
            .map_err(bidi_to_wd)?;
        assert!(!added.intercept.as_str().is_empty());
        bidi.network().remove_intercept(added.intercept).await.map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn network_set_cache_behavior_global() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        bidi.network()
            .set_cache_behavior(network::CacheBehavior::Bypass)
            .await
            .map_err(bidi_to_wd)?;
        bidi.network()
            .set_cache_behavior(network::CacheBehavior::Default)
            .await
            .map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// storage
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn storage_set_get_delete_cookie_round_trip() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        // Cookies need a real (non-data:) origin. Navigate to about:blank
        // is rejected by some drivers so use a tiny localhost server.
        let server = spawn_local_http("ok").await?;
        bidi.browsing_context()
            .navigate(ctx, server.clone(), Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;

        let host = url::Url::parse(&server)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .expect("host");
        bidi.storage()
            .set_cookie(storage::PartialCookie {
                name: "thirtyfour".to_string(),
                value: storage::bytes_string("hello"),
                domain: host.clone(),
                path: Some("/".to_string()),
                http_only: Some(false),
                secure: Some(false),
                same_site: Some(storage::SameSite::Lax),
                expiry: None,
            })
            .await
            .map_err(bidi_to_wd)?;
        let got = bidi.storage().get_cookies_by_name("thirtyfour").await.map_err(bidi_to_wd)?;
        assert_eq!(got.cookies.len(), 1);
        assert_eq!(got.cookies[0].name, "thirtyfour");
        bidi.storage().delete_cookies_by_name("thirtyfour").await.map_err(bidi_to_wd)?;
        let after = bidi.storage().get_cookies_by_name("thirtyfour").await.map_err(bidi_to_wd)?;
        assert!(after.cookies.is_empty());
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// input
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn input_perform_actions_keystrokes_into_input() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.browsing_context()
            .navigate(ctx.clone(), BLANK_HTML, Some(browsing_context::ReadinessState::Complete))
            .await
            .map_err(bidi_to_wd)?;
        // Focus the input element first via JS.
        bidi.script()
            .evaluate(ctx.clone(), "document.getElementById('t').focus()", false)
            .await
            .map_err(bidi_to_wd)?;
        // Send "hi" through input.performActions.
        let actions = vec![serde_json::json!({
            "id": "kbd",
            "type": "key",
            "actions": [
                {"type": "keyDown", "value": "h"},
                {"type": "keyUp",   "value": "h"},
                {"type": "keyDown", "value": "i"},
                {"type": "keyUp",   "value": "i"},
            ]
        })];
        bidi.input().perform_actions(ctx.clone(), actions).await.map_err(bidi_to_wd)?;
        let r = bidi
            .script()
            .evaluate(ctx, "document.getElementById('t').value", false)
            .await
            .map_err(bidi_to_wd)?;
        let v = r.ok_value().expect("success");
        assert_eq!(v["value"].as_str(), Some("hi"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn input_release_actions_no_op_after_perform() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let tree = bidi.browsing_context().get_tree(None).await.map_err(bidi_to_wd)?;
        let ctx = tree.contexts[0].context.clone();
        bidi.input().release_actions(ctx).await.map_err(bidi_to_wd)?;
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// browser
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn browser_user_context_create_list_remove() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let created = bidi.browser().create_user_context().await.map_err(bidi_to_wd)?;
        let list = bidi.browser().get_user_contexts().await.map_err(bidi_to_wd)?;
        assert!(list.user_contexts.iter().any(|c| c.user_context == created.user_context));
        bidi.browser()
            .remove_user_context(created.user_context.clone())
            .await
            .map_err(bidi_to_wd)?;
        let after = bidi.browser().get_user_contexts().await.map_err(bidi_to_wd)?;
        assert!(after.user_contexts.iter().all(|c| c.user_context != created.user_context));
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// permissions
// ---------------------------------------------------------------------------
//
// `permissions.setPermission` is optional in the BiDi spec and not
// implemented by every driver. Test it best-effort: skip on
// `unknown command` / `unknown method` / `unsupported operation`.
#[tokio::test(flavor = "multi_thread")]
async fn permissions_set_permission_best_effort() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let res = bidi
            .permissions()
            .set_permission(
                serde_json::json!({"name": "geolocation"}),
                thirtyfour::bidi::modules::permissions::PermissionState::Granted,
                "https://example.com",
            )
            .await;
        match res {
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.error.as_str(),
                    "unknown command" | "unknown method" | "unsupported operation"
                ) => {}
            Err(e) => return Err(bidi_to_wd(e)),
        }
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// untyped escape hatch
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn send_raw_unknown_method_returns_typed_error() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = launch_managed_bidi().await?;
        let bidi = driver.bidi().await?;
        let err = bidi
            .send_raw("nope.notACommand", serde_json::json!({}))
            .await
            .expect_err("expected an error");
        assert!(matches!(err.error.as_str(), "unknown command" | "unknown method"));
        driver.quit().await
    })
    .await
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

async fn spawn_local_http(body: &'static str) -> WebDriverResult<String> {
    let listener = tokio::net::TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
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

// Touch unused-imports the test file declares so `use` lines stay tidy
// even if a test gets removed.
#[allow(dead_code)]
fn _force_imports(_: browser::CreateUserContext) {}
