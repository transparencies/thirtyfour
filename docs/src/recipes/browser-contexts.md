# Frames, Shadow DOM, And Files

Frames and shadow roots create new query scopes. File inputs cross from the
browser into the machine running the driver. These recipes make each boundary
explicit.

## Work Inside An iframe

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn submits_embedded_checkout() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/checkout").await?;
        driver
            .query(By::Testid("payment-frame"))
            .desc("payment iframe")
            .single()
            .await?
            .enter_frame()
            .await?;

        let frame_result: WebDriverResult<()> = async {
            let pay = driver
                .query(By::Testid("pay-now"))
                .desc("pay now button inside payment iframe")
                .single()
                .await?;
            pay.wait_until().clickable().await?;
            pay.click().await?;
            Ok(())
        }
        .await;

        // Frame switching has no guard, so restore the outer context even if
        // the interaction inside the frame fails.
        let restore_result = driver.enter_default_frame().await;
        frame_result?;
        restore_result?;
        driver
            .query(By::Testid("payment-status"))
            .with_text("Payment complete")
            .and_displayed()
            .desc("completed payment status")
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

After `enter_frame()`, driver-level queries target that frame. There is no
frame guard, so retain the inner result and attempt `enter_default_frame()`
before propagating it or querying outer content.

## Query A Shadow Root

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn opens_the_shadow_dom_menu() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/dashboard").await?;
        let host = driver
            .query(By::Testid("account-menu-host"))
            .desc("account menu shadow host")
            .single()
            .await?;
        let root = host.get_shadow_root().await?;

        let open = root
            .query(By::Testid("open-account-menu"))
            .desc("open account menu button")
            .single()
            .await?;
        open.wait_until().clickable().await?;
        open.click().await?;
        root.query(By::Testid("account-menu"))
            .and_displayed()
            .desc("opened account menu")
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

`get_shadow_root()` returns a queryable element handle. Scope selectors through
that root; a normal driver query does not pierce a shadow boundary.

## Upload A File

The path must exist on the machine running the browser driver. When Selenium
Grid runs the driver remotely, use the Grid's file-transfer setup rather than
assuming a path on the test client is visible to the remote node.

```rust,no_run
use thirtyfour::prelude::*;

#[tokio::test]
async fn uploads_an_avatar() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    let test_result: WebDriverResult<()> = async {
        driver.goto("https://example.test/profile").await?;
        let input = driver
            .query(By::Testid("avatar-file"))
            .desc("avatar file input")
            .single()
            .await?;

        input.send_keys("/absolute/path/to/avatar.png").await?;
        let upload = driver
            .query(By::Testid("upload-avatar"))
            .desc("upload avatar button")
            .single()
            .await?;
        upload.wait_until().clickable().await?;
        upload.click().await?;

        driver
            .query(By::Testid("avatar-status"))
            .with_text("Upload complete")
            .and_displayed()
            .desc("completed avatar upload status")
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

Send the absolute file path directly to the `<input type="file">`; do not
automate the operating system's file-picker window.
