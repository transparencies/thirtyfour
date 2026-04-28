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

use std::time::Duration;

use thirtyfour::manager::{DriverVersion, WebDriverManager};
use thirtyfour::prelude::*;
use thirtyfour::{ChromeCapabilities, EdgeCapabilities, FirefoxCapabilities};

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
    let driver = WebDriver::managed(chrome_caps()).await?;
    driver.goto("about:blank").await?;
    driver.quit().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn managed_firefox_smoke() -> WebDriverResult<()> {
    let driver = WebDriver::managed(firefox_caps()).await?;
    driver.goto("about:blank").await?;
    driver.quit().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn managed_edge_smoke() -> WebDriverResult<()> {
    let driver = WebDriver::managed(edge_caps()).await?;
    driver.goto("about:blank").await?;
    driver.quit().await?;
    Ok(())
}

/// Safari is macOS-only and uses the system `safaridriver`. The CI runner must
/// have run `safaridriver --enable` (and have Allow Remote Automation toggled).
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
async fn managed_safari_smoke() -> WebDriverResult<()> {
    let mut caps = DesiredCapabilities::safari();
    let _ = &mut caps; // safari has no headless / no sandbox toggles
    let driver = WebDriver::managed(caps).await?;
    driver.goto("about:blank").await?;
    driver.quit().await?;
    Ok(())
}

/// Exercises a few non-default options on Chrome:
///   - explicit `WebDriverManager::builder().latest()` instead of `MatchLocalBrowser`
///   - a custom cache dir
///   - two `launch()` calls share a single chromedriver process (refcount + dedup)
///   - a custom ready timeout
#[tokio::test(flavor = "multi_thread")]
async fn managed_chrome_options_and_dedup() -> WebDriverResult<()> {
    let cache = tempfile::tempdir().expect("tempdir");
    let mgr = WebDriverManager::builder()
        .version(DriverVersion::Latest)
        .cache_dir(cache.path().to_path_buf())
        .ready_timeout(Duration::from_secs(60))
        .build();

    let d1 = mgr.launch(chrome_caps()).await?;
    let d2 = mgr.launch(chrome_caps()).await?;

    // Two `launch()` calls keyed on the same (BrowserKind, version, host)
    // should reuse a single chromedriver subprocess — both `WebDriver`
    // instances should point at the same server URL.
    assert_eq!(
        d1.handle.server_url(),
        d2.handle.server_url(),
        "two managed sessions for the same browser must share a chromedriver process"
    );

    d1.goto("about:blank").await?;
    d2.goto("about:blank").await?;

    d1.quit().await?;
    d2.quit().await?;
    Ok(())
}
