//! Run as follows:
//!
//!     cargo run --example tokio_basic
//!
//! Uses `WebDriver::managed` (default `manager` feature), which auto-downloads
//! the matching `chromedriver` for your installed Chrome and starts it locally.

use thirtyfour::prelude::*;

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(run())
}

async fn run() -> anyhow::Result<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;
    // Navigate to https://wikipedia.org.
    driver.goto("https://wikipedia.org").await?;
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

    // Wait for the unique article heading before checking the title.
    driver.query(By::ClassName("firstHeading")).desc("Wikipedia article heading").single().await?;
    assert_eq!(driver.title().await?, "Selenium - Wikipedia");

    // Always explicitly close the browser. This prevents the executor from being blocked
    driver.quit().await?;

    Ok(())
}
