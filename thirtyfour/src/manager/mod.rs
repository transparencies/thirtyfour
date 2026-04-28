//! Auto-download and lifetime-managed local WebDriver process management.
//!
//! Enabled via the `manager` feature (default-on). When enabled, you can use
//! [`WebDriver::managed`] to launch a session with no external driver server:
//!
//! ```no_run
//! # use thirtyfour::prelude::*;
//! # async fn run() -> WebDriverResult<()> {
//! let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;
//! driver.goto("https://www.rust-lang.org/").await?;
//! driver.quit().await?;
//! # Ok(()) }
//! ```
//!
//! For more control (multi-browser, custom cache dir, offline mode, etc.) construct a
//! [`WebDriverManager`] explicitly:
//!
//! ```no_run
//! # use thirtyfour::prelude::*;
//! # use thirtyfour::manager::WebDriverManager;
//! # async fn run() -> WebDriverResult<()> {
//! let mgr = WebDriverManager::builder().latest().build();
//! let chrome  = mgr.launch(DesiredCapabilities::chrome()).await?;
//! let firefox = mgr.launch(DesiredCapabilities::firefox()).await?;
//! # Ok(()) }
//! ```
//!
//! [`WebDriver::managed`]: crate::WebDriver::managed

mod browser;
mod download;
mod error;
#[allow(clippy::module_inception)]
mod manager;
mod process;
mod version;

#[cfg(test)]
mod tests;

pub use browser::BrowserKind;
pub use error::ManagerError;
pub use manager::{WebDriverManager, WebDriverManagerBuilder};
pub use process::StdioMode;
pub use version::DriverVersion;
