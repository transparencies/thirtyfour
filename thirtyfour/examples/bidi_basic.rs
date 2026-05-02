//! WebDriver BiDi quick start.
//!
//! Run with: `cargo run --example bidi_basic --features manager,bidi`
//!
//! Auto-downloads chromedriver, launches a headless Chrome, opts in to BiDi
//! via the `webSocketUrl: true` capability, and exercises a few common
//! commands and one event subscription.

use futures_util::StreamExt;
use thirtyfour::bidi::modules::browsing_context::events::Load;
use thirtyfour::prelude::*;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> WebDriverResult<()> {
    let mut caps = DesiredCapabilities::chrome();
    caps.set_headless()?;
    caps.set_no_sandbox()?;
    caps.set_disable_gpu()?;
    caps.enable_bidi()?;

    let driver = WebDriver::managed(caps).await?;
    let bidi = driver.bidi().await?;

    // Status round-trip — verifies the WS is connected and ready.
    let status = bidi.session().status().await?;
    println!("driver ready: {} ({})", status.ready, status.message);

    // Pick the active browsing context.
    let tree = bidi.browsing_context().get_tree(None).await?;
    let context = tree.contexts[0].context.clone();

    // Subscribe to load events, then navigate.
    bidi.session().subscribe("browsingContext.load").await?;
    let mut load_events = bidi.subscribe::<Load>();

    let url = "data:text/html,<html><title>BiDi%20demo</title></html>";
    bidi.browsing_context().navigate(context.clone(), url, None).await?;

    if let Some(load) = load_events.next().await {
        println!("loaded: {} (context={})", load.url, load.context);
    }

    // Read the title via script.evaluate.
    let result = bidi.script().evaluate(context.clone(), "document.title", false).await?;
    if let Some(value) = result.ok_value() {
        println!("title: {}", value);
    }

    driver.quit().await
}
