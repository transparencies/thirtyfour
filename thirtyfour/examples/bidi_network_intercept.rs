//! WebDriver BiDi network interception.
//!
//! Run with: `cargo run --example bidi_network_intercept --features manager,bidi`
//!
//! Demonstrates the request-interception loop:
//!
//! 1. Subscribe to `network.beforeRequestSent`.
//! 2. Register an intercept for the request phase.
//! 3. Navigate; the request is paused on the wire.
//! 4. Continue (or fail / synthesize) the paused request.
//!
//! The pattern is the same for response-phase interception — swap the
//! phase enum and continue with `network.continueResponse`.

use futures_util::StreamExt;
use thirtyfour::bidi::modules::browsing_context::ReadinessState;
use thirtyfour::bidi::modules::network;
use thirtyfour::bidi::modules::network::events::BeforeRequestSent;
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

    // Subscribe BEFORE adding the intercept so we don't miss the event.
    bidi.session().subscribe("network.beforeRequestSent").await?;
    let mut events = bidi.subscribe::<BeforeRequestSent>();

    let intercept = bidi
        .network()
        .add_intercept(vec![network::InterceptPhase::BeforeRequestSent], None)
        .await?;
    println!("intercept registered: {}", intercept.intercept);

    let tree = bidi.browsing_context().get_tree(None).await?;
    let context = tree.contexts[0].context.clone();

    // Kick off the navigation in the background — it won't return until
    // we continue the paused request.
    let nav = {
        let bidi = bidi.clone();
        let context = context.clone();
        tokio::spawn(async move {
            bidi.browsing_context()
                .navigate(context, "https://example.com/", Some(ReadinessState::Complete))
                .await
        })
    };

    // Wait for the paused request and let it through unmodified.
    while let Some(event) = events.next().await {
        if event.is_blocked && event.request.url.starts_with("https://example.com/") {
            println!("continuing paused {} {}", event.request.method, event.request.url);
            bidi.network().continue_request(event.request.request).await?;
            break;
        }
    }

    nav.await.map_err(|e| WebDriverError::FatalError(format!("nav join: {e}")))??;

    bidi.network().remove_intercept(intercept.intercept).await?;
    println!("page loaded; intercept removed");

    driver.quit().await
}
