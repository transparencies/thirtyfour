## Writing Your First Browser Automation Code

Before we begin, you'll need to install Rust. You can do so by using the [rustup](https://rustup.rs/) tool.

Let's start a new project. Open your terminal application and navigate to the directory 
where you usually put your source code. Then run these commands:

    cargo new --bin my-automation-project
    cd my-automation-project

You will see a `Cargo.toml` file and a `src/` directory there already.

First, let's edit the `Cargo.toml` file in your editor (e.g. Visual Studio Code) and add some dependencies:

    [dependencies]
    thirtyfour = "THIRTYFOUR_CRATE_VERSION"
    tokio = { version = "1", features = ["full"] }

Great! Now let's open `src/main.rs` and  add the following code.

> **NOTE:** Make sure you remove any existing code from `main.rs`.

Don't worry, we'll go through what it does soon.

> */src/main.rs*
```rust
use std::error::Error;

use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

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
    driver.query(By::ClassName("firstHeading")).first().await?;
    assert_eq!(driver.title().await?, "Selenium – Wikipedia");

    // Always explicitly close the browser.
    driver.quit().await?;

    Ok(())
}
```

Make sure Chrome is installed, then run:

    cargo run

If everything worked correctly you should have seen a Chrome browser window open up,
navigate to the "Selenium" article on Wikipedia, and then close again.

The first run will take a few seconds longer than subsequent runs — `thirtyfour`
downloads a matching `chromedriver` into your system cache directory the first time,
then reuses it on every later run. See [WebDriver Manager](../features/manager.md)
for the version-pinning, offline-mode, and observability options that the manager
provides.

## Running on Firefox

To run the code using Firefox instead, change the capabilities in `main`:

```rust
    let driver = WebDriver::managed(DesiredCapabilities::firefox()).await?;
```

Make sure Firefox is installed, and re-run:

    cargo run

If everything worked correctly, you should have seen the Wikipedia page open up on Firefox this time.

Congratulations! You successfully automated a web browser.
