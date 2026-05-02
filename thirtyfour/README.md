[![Crates.io](https://img.shields.io/crates/v/thirtyfour.svg)](https://crates.io/crates/thirtyfour)
[![docs.rs](https://docs.rs/thirtyfour/badge.svg)](https://docs.rs/thirtyfour)
[![Build Status](https://img.shields.io/github/actions/workflow/status/stevepryde/thirtyfour/test.yml?branch=main)](https://github.com/stevepryde/thirtyfour/actions)
[![code coverage](https://codecov.io/github/stevepryde/thirtyfour/graph/badge.svg?token=Z3GDO1EXCX)](https://codecov.io/github/stevepryde/thirtyfour)

# thirtyfour

Thirtyfour is a Selenium / WebDriver library for Rust, for automated website UI testing.


## Features

- All W3C WebDriver and WebElement methods supported
- Create new browser session directly via WebDriver (e.g. chromedriver)
- Create new browser session via Selenium Standalone or Grid
- Find elements (via all common selectors e.g. Id, Class, CSS, Tag, XPath)
- Send keys to elements, including key-combinations
- Execute Javascript
- Action Chains
- Get and set cookies
- Switch to frame/window/element/alert
- Shadow DOM support
- Alert support
- Capture / Save screenshot of browser or individual element as PNG
- Chrome DevTools Protocol (CDP) — typed commands plus optional WebSocket-based event subscription via the `cdp-events` feature
- WebDriver BiDi (W3C bidirectional protocol) — typed commands and event subscription cross-browser, opt-in via the `bidi` feature
- Powerful query interface (the recommended way to find elements) with explicit waits and various predicates
- Component Wrappers (similar to `Page Object Model`)

## Feature Flags

- `rustls-tls`: (Default) Use rustls to provide TLS support (via reqwest).
- `native-tls`: Use native TLS (via reqwest).
- `component`: (Default) Enable the `Component` derive macro (via thirtyfour_macros).
- `cdp`: (Default) Typed Chrome DevTools Protocol commands.
- `cdp-events`: WebSocket-backed CDP event subscription.
- `bidi`: WebDriver BiDi (W3C) — typed commands and event subscription
  via `WebDriver::bidi()`. Opt in by calling `caps.enable_bidi()` before
  starting the session.

### Example (async):

To run this example:

    cargo run --example tokio_async

```rust
use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> WebDriverResult<()> {
     let caps = DesiredCapabilities::chrome();
     let driver = WebDriver::managed(caps).await?;

     // Navigate to https://wikipedia.org.
     driver.goto("https://wikipedia.org").await?;
     let elem_form = driver.find(By::Id("search-form")).await?;

     // Find element from element.
     let elem_text = elem_form.find(By::Id("searchInput")).await?;

     // Type in the search terms.
     elem_text.send_keys("selenium").await?;

     // Click the search button.
     let elem_button = elem_form.find(By::Css("button[type='submit']")).await?;
     elem_button.click().await?;

     // Look for header to implicitly wait for the page to load.
     driver.find(By::ClassName("firstHeading")).await?;
     assert_eq!(driver.title().await?, "Selenium - Wikipedia");

     // Always explicitly close the browser.
     driver.quit().await?;

     Ok(())
}
```

## Minimum Supported Rust Version

Rust 1.88.

## LICENSE

This work is dual-licensed under MIT or Apache 2.0.
You can choose either license if you use this work.

See the NOTICE file for more details.

`SPDX-License-Identifier: MIT OR Apache-2.0`
