# Forms And Page Content

The recipes on this page use app-owned `data-testid` hooks. The important
pattern is the same for any app: scope queries to the smallest meaningful
container, wait for an interactable control, perform the action, and query for
the user-visible result.

## Submit A Login Form

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn logs_in() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/login").await?;
        let form = driver
            .query(By::Testid("login-form"))
            .desc("login form")
            .single()
            .await?;

        form.query(By::Testid("email"))
            .desc("email input")
            .single()
            .await?
            .send_keys("ada@example.test")
            .await?;
        form.query(By::Testid("password"))
            .desc("password input")
            .single()
            .await?
            .send_keys("correct horse battery staple")
            .await?;

        let submit = form
            .query(By::Testid("login-submit"))
            .desc("login button")
            .single()
            .await?;
        submit.wait_until().clickable().await?;
        submit.click().await?;

        driver
            .query(By::Testid("account-heading"))
            .with_text("Welcome, Ada")
            .and_displayed()
            .desc("signed-in account heading")
            .single()
            .await?;
        Ok(())
    }
    .await;

    let quit_result = driver.quit().await;
    test_result?;
    quit_result
}
```

The final query is the assertion. It waits for the application to expose the
signed-in state instead of assuming the click or navigation has finished.

## Exercise A Search Flow

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn searches_the_catalogue() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/catalogue").await?;
        let form = driver
            .query(By::Testid("catalogue-search"))
            .desc("catalogue search form")
            .single()
            .await?;

        form.query(By::Testid("search-input"))
            .desc("catalogue search input")
            .single()
            .await?
            .send_keys("webdriver")
            .await?;
        let submit = form
            .query(By::Css("button[type='submit']"))
            .desc("catalogue search button")
            .single()
            .await?;
        submit.wait_until().clickable().await?;
        submit.click().await?;

        let results = driver
            .query(By::Testid("search-results"))
            .desc("catalogue search results")
            .single()
            .await?;
        results
            .query(By::Testid("result-title"))
            .with_text("Thirtyfour browser automation")
            .and_displayed()
            .desc("matching catalogue result title")
            .first()
            .await?;
        Ok(())
    }
    .await;

    let quit_result = driver.quit().await;
    test_result?;
    quit_result
}
```

`first()` is intentional here: the result list may contain several matching
titles, and the test needs one visible result with the expected text. Use
`single()` instead if duplicate matching results would violate the page
contract.

## Interact With An HTML Modal

This recipe targets an in-page HTML modal, not a browser-native JavaScript
alert. Native alerts use `get_alert_text()`, `accept_alert()`, and
`dismiss_alert()`.

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn confirms_account_deletion() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/account").await?;
        let open = driver
            .query(By::Testid("delete-account"))
            .desc("delete account button")
            .single()
            .await?;
        open.wait_until().clickable().await?;
        open.click().await?;

        let modal = driver
            .query(By::Testid("delete-account-modal"))
            .and_displayed()
            .desc("delete account confirmation modal")
            .single()
            .await?;
        modal
            .query(By::Testid("modal-title"))
            .with_text("Delete account?")
            .desc("delete account modal title")
            .single()
            .await?;

        let cancel = modal
            .query(By::Testid("cancel-delete"))
            .desc("cancel account deletion button")
            .single()
            .await?;
        cancel.wait_until().clickable().await?;
        cancel.click().await?;
        modal.wait_until().stale().await?;
        Ok(())
    }
    .await;

    let quit_result = driver.quit().await;
    test_result?;
    quit_result
}
```

This page contract removes the modal from the DOM when it closes, so `stale()`
is the correct completion condition. If your app keeps the modal mounted and
hides it, use `not_displayed()` instead.

## Assert A Table Or List Row

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn shows_the_shipped_order() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/orders").await?;
        let table = driver
            .query(By::Testid("orders-table"))
            .desc("orders table")
            .single()
            .await?;

        let order = table
            .query(By::Css("tbody tr[data-order-id='1234']"))
            .desc("row for order 1234")
            .single()
            .await?;
        order
            .query(By::Testid("order-status"))
            .with_text("Shipped")
            .and_displayed()
            .desc("shipped status for order 1234")
            .single()
            .await?;
        Ok(())
    }
    .await;

    let quit_result = driver.quit().await;
    test_result?;
    quit_result
}
```

The first query scopes the assertion to the table. The stable app-owned
`data-order-id` attribute enforces that exactly one row represents the order,
and the nested status query checks the user-visible state without depending on
column position.
