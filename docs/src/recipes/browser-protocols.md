# CDP And BiDi

Keep protocol-specific code behind a small boundary. CDP is Chromium-only and
enabled by `thirtyfour`'s default features. BiDi is the W3C cross-browser path,
but it requires both a Cargo feature and a session capability. The currently
documented and tested implementations are Chromium with chromedriver 115 or
newer and Firefox with geckodriver 0.31 or newer.

## Clear Chromium's Cache With CDP

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn clears_the_browser_cache() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/catalogue").await?;
        driver.cdp().network().clear_browser_cache().await?;

        let version = driver.cdp().browser().get_version().await?;
        println!("cleared cache in {}", version.product);
        Ok(())
    }
    .await;

    let quit_result = driver.quit().await;
    test_result?;
    quit_result
}
```

This recipe needs the `cdp` feature, which is enabled by default, and a
Chromium-backed driver. Typed commands use the existing WebDriver connection;
no event WebSocket is required. See the
[CDP guide](../cdp/overview.md) for supported domains and the untyped escape
hatch.

## Subscribe To A BiDi Load Event

Enable the optional `bidi` feature and add `futures-util` for `StreamExt`:

```toml
[dependencies]
futures-util = "0.3"
thirtyfour = { version = "THIRTYFOUR_CRATE_VERSION", features = ["bidi"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
```

```rust,no_run
use std::time::Duration;
use futures_util::StreamExt;
use thirtyfour::bidi::events::Load;
use thirtyfour::prelude::*;

#[tokio::test]
async fn observes_a_page_load() -> WebDriverResult<()> {
    let mut caps = DesiredCapabilities::chrome();
    caps.enable_bidi()?;
    let driver = WebDriver::managed(caps).await?;

    let test_result: WebDriverResult<()> = async {
        let bidi = driver.bidi().await?;
        let mut loads = bidi.subscribe::<Load>().await?;
        let context = bidi.browsing_context().top_level().await?;

        bidi.browsing_context()
            .navigate(context, "https://example.test/dashboard", None)
            .await?;
        let load = tokio::time::timeout(Duration::from_secs(10), loads.next())
            .await
            .map_err(|_| WebDriverError::Timeout("BiDi load event timed out".into()))?
            .ok_or_else(|| WebDriverError::FatalError("BiDi event stream closed".into()))?;
        println!("loaded: {}", load.url);

        // Dropping the last typed stream schedules a best-effort unsubscribe.
        drop(loads);
        Ok(())
    }
    .await;

    let quit_result = driver.quit().await;
    test_result?;
    quit_result
}
```

`enable_bidi()` requests the WebSocket URL during session creation, and
`subscribe::<Load>()` subscribes while the typed stream is alive. Dropping the
last stream schedules a best-effort background unsubscribe; the following
`driver.quit()` closes the WebDriver session regardless. The
[BiDi overview](../bidi/overview.md) covers browser support, and the
[BiDi Events guide](../bidi/events.md) covers typed streams, scoping, and
interception.
