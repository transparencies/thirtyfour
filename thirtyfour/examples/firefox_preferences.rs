//! Run as follows:
//!
//!     cargo run --example firefox_preferences
//!
//! Uses `WebDriver::managed` (default `manager` feature), which auto-downloads
//! a matching `geckodriver` for your installed Firefox and starts it locally.

use thirtyfour::common::capabilities::firefox::FirefoxPreferences;
use thirtyfour::{FirefoxCapabilities, WebDriver};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let user_agent = "Custom";

    // Set user agent via Firefox preferences.
    let mut prefs = FirefoxPreferences::new();
    prefs.set_user_agent(user_agent.to_string())?;

    let mut caps = FirefoxCapabilities::new();
    caps.set_preferences(prefs)?;

    let driver = WebDriver::managed(caps).await?;
    driver.goto("https://www.google.com").await?;

    // Get the user agent and verify.
    let js_user_agent: String =
        driver.execute(r#"return navigator.userAgent;"#, Vec::new()).await?.convert()?;
    assert_eq!(&js_user_agent, user_agent);

    // Always explicitly close the browser. This prevents the executor from being blocked
    driver.quit().await?;
    Ok(())
}
