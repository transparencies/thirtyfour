//! Run as follows:
//!
//!     cargo run --example custom_poller
//!
//! Uses `WebDriver::managed` (default `manager` feature), which auto-downloads
//! the matching `chromedriver` for your installed Chrome and starts it locally.

use std::sync::Arc;
use std::time::Duration;

use thirtyfour::common::config::WebDriverConfig;
use thirtyfour::extensions::query::ElementPollerWithTimeout;
use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    // Navigate to https://wikipedia.org.
    driver.goto("https://wikipedia.org").await?;

    // Override default query timeout on a per-query basis.
    let elem_form = driver
        .query(By::Id("search-form"))
        .wait(Duration::from_secs(60), Duration::from_secs(1))
        .single()
        .await?;

    // Override default wait timeout on a per-query basis.
    elem_form
        .wait_until()
        .wait(Duration::from_secs(60), Duration::from_secs(1))
        .displayed()
        .await?;

    // Use a custom poller instance.
    let my_poller =
        Arc::new(ElementPollerWithTimeout::new(Duration::from_secs(120), Duration::from_secs(1)));
    let new_config = WebDriverConfig::builder().poller(my_poller).build()?;
    let my_driver = driver.clone_with_config(new_config);

    // Perform query using custom poller.
    let elem_form = my_driver.query(By::Id("search-form")).single().await?;

    // Perform element wait using custom poller. Elements always inherit the `WebDriverConfig` from
    // the `WebDriver` instance that found them.
    elem_form.wait_until().displayed().await?;

    // Always explicitly close the browser. This prevents the executor from being blocked
    driver.quit().await?;

    Ok(())
}
