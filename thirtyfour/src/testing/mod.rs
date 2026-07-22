//! Helpers for running reliable browser tests.
//!
//! [`run_browser_test`](crate::testing::run_browser_test) owns the
//! browser-session lifecycle around an async test
//! body. It always attempts [`WebDriver::quit`] after the body returns or
//! unwinds, so early `?` returns and panicking assertions do not accidentally
//! leave cleanup to the blocking `Drop` fallback.
//!
//! The session input is any future that produces a [`WebDriver`]. This keeps
//! the same helper useful for a local managed browser and for an external
//! Selenium or Grid session:
//!
//! ```no_run
//! use thirtyfour::{
//!     prelude::*,
//!     testing::{BrowserTestError, run_browser_test},
//! };
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), BrowserTestError> {
//! let mut caps = DesiredCapabilities::chrome();
//! caps.set_headless()?;
//!
//! run_browser_test(WebDriver::managed(caps), |driver| async move {
//!     driver.goto("https://example.com").await?;
//!     driver.query(By::Css("h1")).single().await?;
//!     Ok(())
//! })
//! .await
//! # }
//! ```
//!
//! [`WebDriver`]: crate::WebDriver
//! [`WebDriver::quit`]: crate::WebDriver::quit

mod runner;

pub use runner::{BrowserTestError, run_browser_test};
