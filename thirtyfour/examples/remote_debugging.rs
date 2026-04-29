//! Attach to an already-running Chrome that was launched with the
//! `--remote-debugging-port` flag.
//!
//! Launch Chrome first:
//!
//!     chrome --remote-debugging-port=9222 --user-data-dir="C:\Users\username\my-browser-profile\"
//!
//! Then run as follows:
//!
//!     cargo run --example remote_debugging
//!
//! Uses `WebDriver::managed` (default `manager` feature), which auto-downloads
//! a matching `chromedriver` and starts it locally; the capabilities below tell
//! `chromedriver` to attach to the Chrome listening on the debugger port
//! instead of launching a new one.

use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let mut caps = DesiredCapabilities::chrome();
    caps.set_debugger_address("localhost:9222")?;
    let driver = WebDriver::managed(caps).await?;
    driver.goto("https://www.baidu.com").await?;
    Ok(())
}
