//! Run as follows:
//!
//!     cargo run --example chrome_devtools
//!
//! Demonstrates the typed CDP API on [`WebDriver`]. For event subscription
//! see `cargo run --example chrome_devtools --features cdp-events`.

use thirtyfour::cdp::domains::network::{ConnectionType, NetworkConditions};
use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    // Typed CDP — domain-grouped methods on `driver.cdp()`.
    let info = driver.cdp().browser().get_version().await?;
    println!("Chrome: {}", info.product);
    println!("User agent: {}", info.user_agent);

    // `Network.emulateNetworkConditions` — works on Chrome, Edge, Brave, Opera.
    driver
        .cdp()
        .network()
        .emulate_network_conditions(NetworkConditions {
            offline: false,
            latency: 200,
            download_throughput: 256 * 1024,
            upload_throughput: 64 * 1024,
            connection_type: Some(ConnectionType::Cellular3G),
        })
        .await?;

    // `Network.clearBrowserCache`.
    driver.cdp().network().clear_browser_cache().await?;

    // Untyped escape hatch for one-off commands not in the curated set.
    let raw = driver.cdp().send_raw("Browser.getVersion", serde_json::json!({})).await?;
    println!("Raw response: {}", raw["userAgent"]);

    driver.quit().await?;
    Ok(())
}
