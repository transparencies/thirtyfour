//! WebDriver BiDi `log.entryAdded` event subscription.
//!
//! Run with: `cargo run --example bidi_log_events --features manager,bidi`
//!
//! Subscribes to `log.entryAdded`, navigates to a page that calls
//! `console.log` / `console.warn` / `console.error`, and prints each
//! entry with its severity, source realm, and text.

use futures_util::StreamExt;
use thirtyfour::bidi::modules::log::LogLevel;
use thirtyfour::bidi::modules::log::events::EntryAdded;
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

    bidi.session().subscribe("log.entryAdded").await?;
    let mut events = bidi.subscribe::<EntryAdded>();

    let tree = bidi.browsing_context().get_tree(None).await?;
    let context = tree.contexts[0].context.clone();

    let page = "data:text/html,<html><body><script>\
        console.log('hello from log');\
        console.warn('a warning');\
        console.error('an error');\
        </script></body></html>";
    bidi.browsing_context()
        .navigate(
            context,
            page,
            Some(thirtyfour::bidi::modules::browsing_context::ReadinessState::Complete),
        )
        .await?;

    // Drain three log entries — one per console call above.
    let mut seen = 0;
    while let Some(event) = events.next().await {
        let level = match event.level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Unknown(ref s) => s.as_str(),
        };
        let text = event.text.as_deref().unwrap_or("(no text)");
        println!("[{level}] {} :: {text}", event.entry_type);
        seen += 1;
        if seen >= 3 {
            break;
        }
    }

    driver.quit().await
}
