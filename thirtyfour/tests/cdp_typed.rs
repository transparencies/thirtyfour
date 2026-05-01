//! End-to-end tests for the typed CDP API.
//!
//! These run against a real chromedriver downloaded by the manager, so they
//! gate behind the `manager-tests` feature and live in their own file (the
//! `manager-test.yml` CI job runs `cargo test ... --test cdp_typed` without
//! pre-starting a driver).
//!
//! ```text
//! cargo test -p thirtyfour --features manager-tests --test cdp_typed -- --test-threads=1
//! ```
//!
//! Tests intentionally use `data:` URLs and inline JS where possible so they
//! don't depend on the file-server fixture in `common.rs`. The HTTP path
//! (`Cdp` via `goog/cdp/execute`) covers everything except event
//! subscription, which is exercised separately in `cdp_events.rs`.

#![cfg(all(feature = "manager-tests", feature = "cdp"))]

use std::future::Future;
use std::time::Duration;

use thirtyfour::ChromeCapabilities;
use thirtyfour::cdp::domains::{browser, network, runtime};
use thirtyfour::prelude::*;

const TEST_TIMEOUT: Duration = Duration::from_secs(180);

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
    caps
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_browser_get_version_typed() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        let info = driver.cdp().browser().get_version().await?;
        assert!(info.user_agent.contains("Chrome"), "user_agent: {}", info.user_agent);
        assert!(info.product.starts_with("Chrome/") || info.product.starts_with("HeadlessChrome/"));
        assert!(!info.protocol_version.is_empty());
        assert!(!info.js_version.is_empty());
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_browser_get_version_via_command_struct() -> WebDriverResult<()> {
    // Same call but using the `Cdp::send` typed entry directly instead of
    // the domain facade — verifies the trait-based path.
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        let info = driver.cdp().send(browser::GetVersion).await?;
        assert!(info.user_agent.to_ascii_lowercase().contains("chrome"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_send_raw_escape_hatch() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        let v = driver.cdp().send_raw("Browser.getVersion", serde_json::json!({})).await?;
        assert!(v["userAgent"].as_str().unwrap().to_ascii_lowercase().contains("chrome"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_runtime_evaluate_value_returns_typed_json() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        let v = driver.cdp().runtime().evaluate_value("1 + 41").await?;
        assert_eq!(v, serde_json::json!(42));
        let s = driver.cdp().runtime().evaluate_value("'foo' + 'bar'").await?;
        assert_eq!(s, serde_json::json!("foobar"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_emulation_device_metrics_changes_inner_width() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        // Force a specific viewport via Emulation, then read it back via JS.
        driver.cdp().emulation().set_device_metrics_override(444, 333, 1.0, false).await?;
        // A page must be loaded for `window.innerWidth` to be meaningful.
        driver.goto("data:text/html,<html><body>hi</body></html>").await?;
        let w = driver.cdp().runtime().evaluate_value("window.innerWidth").await?;
        let h = driver.cdp().runtime().evaluate_value("window.innerHeight").await?;
        assert_eq!(w, serde_json::json!(444), "innerWidth after override");
        assert_eq!(h, serde_json::json!(333), "innerHeight after override");
        driver.cdp().emulation().clear_device_metrics_override().await?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_network_clear_browser_cache_returns_unit() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        // Just confirms the unit-returning command path round-trips.
        driver.cdp().network().clear_browser_cache().await?;
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_network_user_agent_override_via_typed_command() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        driver
            .cdp()
            .send(network::SetUserAgentOverride {
                user_agent: "thirtyfour-cdp-test/1.0".to_string(),
                accept_language: None,
                platform: None,
            })
            .await?;
        // Override is reported back via navigator.userAgent on the next document.
        driver.goto("data:text/html,<html><body>x</body></html>").await?;
        let ua = driver.cdp().runtime().evaluate_value("navigator.userAgent").await?;
        assert_eq!(ua, serde_json::json!("thirtyfour-cdp-test/1.0"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_element_backend_node_id_resolves_and_describes() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        driver
            .goto("data:text/html,<html><body><button id='b' class='c'>hi</button></body></html>")
            .await?;
        let elem = driver.find(By::Id("b")).await?;

        // The whole bridge: WebElement -> RemoteObjectId -> BackendNodeId.
        let backend_id = elem.cdp_backend_node_id().await?;

        // DescribeNode against the BackendNodeId should report a BUTTON node.
        let described = driver
            .cdp()
            .send(thirtyfour::cdp::domains::dom::DescribeNode {
                backend_node_id: Some(backend_id),
                ..Default::default()
            })
            .await?;
        assert_eq!(described.node.node_name, "BUTTON");
        // attributes is `[name, value, name, value, ...]`. Look for id="b".
        let attrs = described.node.attributes.unwrap_or_default();
        let pairs: Vec<_> = attrs.chunks_exact(2).collect();
        assert!(pairs.iter().any(|p| p[0] == "id" && p[1] == "b"), "attrs: {attrs:?}");

        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_legacy_chrome_devtools_still_works() -> WebDriverResult<()> {
    // Backward compat: the deprecated `ChromeDevTools` helper must still
    // work for the lifetime of the deprecation period.
    #[allow(deprecated)]
    use thirtyfour::extensions::cdp::ChromeDevTools;

    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        #[allow(deprecated)]
        let dev = ChromeDevTools::new(driver.handle.clone());
        #[allow(deprecated)]
        let v = dev.execute_cdp("Browser.getVersion").await?;
        assert!(v["userAgent"].as_str().unwrap().to_ascii_lowercase().contains("chrome"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_emulation_set_user_agent_via_emulation_domain() -> WebDriverResult<()> {
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        driver.cdp().emulation().set_user_agent_override("EmuUA/1.0").await?;
        driver.goto("data:text/html,<html><body>x</body></html>").await?;
        let ua = driver.cdp().runtime().evaluate_value("navigator.userAgent").await?;
        assert_eq!(ua, serde_json::json!("EmuUA/1.0"));
        driver.quit().await
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn cdp_evaluate_remote_object_for_complex_value() -> WebDriverResult<()> {
    // `Runtime.evaluate` without `returnByValue` should return a RemoteObject
    // handle with a non-empty objectId for an object literal.
    with_timeout(async {
        let driver = WebDriver::managed(chrome_caps()).await?;
        let result = driver.cdp().send(runtime::Evaluate::new("({a: 1, b: [2, 3]})")).await?;
        assert_eq!(result.result.r#type, "object");
        assert!(result.result.object_id.is_some(), "expected an object handle");
        // Release the handle to keep the test hygienic.
        if let Some(id) = result.result.object_id {
            driver.cdp().runtime().release_object(id).await?;
        }
        driver.quit().await
    })
    .await
}
