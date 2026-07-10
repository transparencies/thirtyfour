//! Run as follows:
//!
//!     cargo run --example minimal_async
//!
//! This example uses `WebDriver::managed` (default `manager` feature), which
//! auto-downloads the matching `chromedriver` for your installed Chrome,
//! starts it locally, and shuts it down when the `WebDriver` is dropped.

use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;
    // Navigate to https://wikipedia.org.
    driver.goto("https://wikipedia.org").await?;

    // Query the unique search form.
    let elem_form =
        driver.query(By::Id("search-form")).desc("Wikipedia search form").single().await?;

    // Querying from an element scopes the search to its subtree.
    let elem_text =
        elem_form.query(By::Id("searchInput")).desc("Wikipedia search input").single().await?;

    // Type in the search terms.
    elem_text.send_keys("selenium").await?;

    // Click the search button.
    let elem_button = elem_form
        .query(By::Css("button[type='submit']"))
        .desc("Wikipedia search button")
        .single()
        .await?;
    elem_button.click().await?;

    // Always explicitly close the browser. This prevents the executor from being blocked
    driver.quit().await?;

    Ok(())
}
