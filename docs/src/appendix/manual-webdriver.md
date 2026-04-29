# Manual WebDriver Setup

The default [Managed WebDriver](../features/manager.md) feature handles
downloading and running the appropriate webdriver subprocess for you. This
appendix is for the cases where you want to do it yourself:

- You have disabled the `manager` feature.
- You're connecting to an externally-managed driver server, like a remote
  [Selenium grid](../tools/selenium.md) or a driver running in a container.
- You want a long-lived driver process you can reuse across many runs of
  your program.

In any of these scenarios you'll need to download the webdriver binary,
start it on a known port, and pass that URL to `WebDriver::new(...)`.

## Downloading A WebDriver Binary

Pick the binary that matches your browser:

* For Chrome, download [chromedriver](https://developer.chrome.com/docs/chromedriver/downloads)
* For Firefox, download [geckodriver](https://github.com/mozilla/geckodriver/releases)
* For Microsoft Edge, download [msedgedriver](https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/)
* For Safari (macOS), `safaridriver` ships with the OS — run
  `safaridriver --enable` once to allow remote automation.

The webdriver may be zipped. Unzip it and place the binary somewhere in
your `PATH`. Make sure it is executable and that you have permission to
run it.

> If the webdriver is not the right version for your browser, it will
> show an error message when you try to start a new session using
> `thirtyfour`. Browser auto-updates are a common cause; the
> [Managed WebDriver](../features/manager.md) feature exists in part to
> sidestep this problem.

## Starting The WebDriver

Open a terminal and run the binary directly:

    chromedriver        # listens on port 9515 by default
    geckodriver         # listens on port 4444 by default
    msedgedriver        # listens on port 9515 by default

Leave it running in that terminal — it's the server that `thirtyfour`
will talk to.

## Connecting From Your Code

Pass the driver's URL to `WebDriver::new(...)`:

```rust
use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> WebDriverResult<()> {
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:9515", caps).await?;
    driver.goto("https://www.rust-lang.org/").await?;
    driver.quit().await?;
    Ok(())
}
```

For Firefox, use `DesiredCapabilities::firefox()` and the geckodriver URL
(`http://localhost:4444`).
