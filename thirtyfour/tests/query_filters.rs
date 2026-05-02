//! Integration tests for `ElementQuery` filter predicates against a real
//! browser via `WebDriver::managed`.
//!
//! Gated behind `manager-tests` like the other manager-driven E2E tests:
//!
//! ```text
//! cargo test -p thirtyfour --features manager-tests --test query_filters -- --test-threads=1
//! ```

#![cfg(feature = "manager-tests")]

use std::time::Duration;

use thirtyfour::ChromeCapabilities;
use thirtyfour::prelude::*;

use crate::common::launch_managed_chrome;

mod common;

const TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn chrome_caps() -> ChromeCapabilities {
    let mut caps = DesiredCapabilities::chrome();
    caps.set_headless().unwrap();
    caps.set_no_sandbox().unwrap();
    caps.set_disable_gpu().unwrap();
    caps.set_disable_dev_shm_usage().unwrap();
    caps.add_arg("--no-sandbox").unwrap();
    caps
}

/// Reproduces issue #259: `with_class("foo")` against an element whose `class`
/// attribute is `"a b foo c"` should match. The user expects CSS-class
/// semantics — match a single token within the space-separated class list.
#[tokio::test(flavor = "multi_thread")]
async fn with_class_matches_one_class_in_multi_class_attribute() -> WebDriverResult<()> {
    tokio::time::timeout(TEST_TIMEOUT, async {
        let driver = launch_managed_chrome(chrome_caps()).await?;

        // Two `<li>`s share a generic class list; only the second carries the
        // class we filter on. Mirrors the markup from the original bug report.
        driver
            .goto(
                "data:text/html,<html><body><ul>\
                 <li class='MuiListItem-root MuiListItem-gutters practice-type-list-item css-120b5pf' data-testid='type-1'>type</li>\
                 <li class='MuiListItem-root MuiListItem-gutters practice-exercises-list-item css-j4r5fq' data-testid='exercises-1'>e1</li>\
                 <li class='MuiListItem-root MuiListItem-gutters practice-exercises-list-item css-j4r5fq' data-testid='exercises-2'>e2</li>\
                 </ul></body></html>",
            )
            .await?;

        let elems = driver
            .query(By::Tag("li"))
            .with_class("practice-exercises-list-item")
            .nowait()
            .any()
            .await?;

        assert_eq!(
            elems.len(),
            2,
            "with_class should match elements where the class attribute contains the class as a token"
        );

        driver.quit().await?;
        Ok(())
    })
    .await
    .unwrap_or_else(|_| {
        Err(WebDriverError::FatalError(format!(
            "test exceeded {}s budget",
            TEST_TIMEOUT.as_secs()
        )))
    })
}

/// `with_class("foo")` must NOT match an element whose class is `"foobar"` —
/// CSS class semantics are whole-token, not substring.
#[tokio::test(flavor = "multi_thread")]
async fn with_class_does_not_match_class_substring() -> WebDriverResult<()> {
    tokio::time::timeout(TEST_TIMEOUT, async {
        let driver = launch_managed_chrome(chrome_caps()).await?;

        driver
            .goto(
                "data:text/html,<html><body>\
                 <div class='foobar' data-testid='foobar'>x</div>\
                 <div class='foo' data-testid='foo'>y</div>\
                 </body></html>",
            )
            .await?;

        let elems = driver.query(By::Tag("div")).with_class("foo").nowait().any().await?;

        assert_eq!(elems.len(), 1, "with_class('foo') should not match class='foobar'");
        assert_eq!(elems[0].attr("data-testid").await?.as_deref(), Some("foo"));

        driver.quit().await?;
        Ok(())
    })
    .await
    .unwrap_or_else(|_| {
        Err(WebDriverError::FatalError(format!("test exceeded {}s budget", TEST_TIMEOUT.as_secs())))
    })
}

/// `without_class("foo")` is the inverse: the element with class `"foo"`
/// must be excluded; `"foobar"` and `"bar"` must be included.
#[tokio::test(flavor = "multi_thread")]
async fn without_class_excludes_only_whole_token_match() -> WebDriverResult<()> {
    tokio::time::timeout(TEST_TIMEOUT, async {
        let driver = launch_managed_chrome(chrome_caps()).await?;

        driver
            .goto(
                "data:text/html,<html><body>\
                 <div class='foobar' data-testid='foobar'>a</div>\
                 <div class='foo bar' data-testid='foo-bar'>b</div>\
                 <div class='bar' data-testid='bar'>c</div>\
                 </body></html>",
            )
            .await?;

        let elems = driver.query(By::Tag("div")).without_class("foo").nowait().any().await?;

        let mut testids: Vec<String> = Vec::new();
        for e in &elems {
            if let Some(id) = e.attr("data-testid").await? {
                testids.push(id);
            }
        }
        testids.sort();

        assert_eq!(testids, vec!["bar".to_string(), "foobar".to_string()]);

        driver.quit().await?;
        Ok(())
    })
    .await
    .unwrap_or_else(|_| {
        Err(WebDriverError::FatalError(format!("test exceeded {}s budget", TEST_TIMEOUT.as_secs())))
    })
}
