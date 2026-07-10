//! Requires selenium running on port 4444:
//!
//!     java -jar selenium-server-standalone-3.141.59.jar
//!
//! Run as follows:
//!
//!     cargo run --example selenium_example

use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let caps = DesiredCapabilities::chrome();
    // NOTE: For selenium 3.x, use "http://localhost:4444/wd/hub/session".
    let driver = WebDriver::new("http://localhost:4444", caps).await?;

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
