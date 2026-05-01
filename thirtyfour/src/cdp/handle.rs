//! The [`Cdp`] handle — the request/response entry point for CDP.
//!
//! Built on top of the WebDriver vendor endpoint `goog/cdp/execute`, so it
//! works for any session with a Chromium-based driver (chromedriver,
//! msedgedriver, brave-driver, etc.) and shares its HTTP client with the
//! parent [`crate::WebDriver`]. For event subscription, upgrade to a
//! [`CdpSession`] via [`Cdp::connect`] (feature `cdp-events`).

use std::sync::Arc;

use serde_json::Value;

use super::command::CdpCommand;
use super::domains;
use super::transport::Transport;
use super::transport::http::HttpTransport;
use crate::error::WebDriverResult;
use crate::session::handle::SessionHandle;

/// Chrome DevTools Protocol handle.
///
/// Cheap to clone; just a wrapped `Arc<SessionHandle>`.
///
/// # Example
/// ```no_run
/// # use thirtyfour::prelude::*;
/// # async fn run() -> WebDriverResult<()> {
/// let driver = WebDriver::new("http://localhost:4444", DesiredCapabilities::chrome()).await?;
///
/// // Untyped escape hatch.
/// let info = driver.cdp().send_raw("Browser.getVersion", serde_json::json!({})).await?;
/// println!("user agent: {}", info["userAgent"]);
///
/// // Typed via a domain facade.
/// driver.cdp().network().clear_browser_cache().await?;
/// driver.quit().await
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Cdp {
    transport: HttpTransport,
}

impl Cdp {
    /// Construct a CDP handle from a session handle. Most users should call
    /// [`crate::WebDriver::cdp`] instead.
    pub fn new(handle: Arc<SessionHandle>) -> Self {
        Self {
            transport: HttpTransport::new(handle),
        }
    }

    /// The underlying session handle. Useful when you need to interleave
    /// CDP and regular WebDriver operations.
    pub fn handle(&self) -> &Arc<SessionHandle> {
        self.transport.handle()
    }

    /// Send a typed CDP command and decode its response.
    ///
    /// # Example
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # async fn run(driver: WebDriver) -> WebDriverResult<()> {
    /// use thirtyfour::cdp::domains::browser::GetVersion;
    /// let v = driver.cdp().send(GetVersion::default()).await?;
    /// println!("{}", v.user_agent);
    /// # Ok(()) }
    /// ```
    pub async fn send<C: CdpCommand>(&self, params: C) -> WebDriverResult<C::Returns> {
        let raw = self.send_raw(C::METHOD, serde_json::to_value(params)?).await?;
        Ok(serde_json::from_value(raw)?)
    }

    /// Send a CDP command by name with raw JSON params, returning the raw
    /// `result` value. Useful for one-off commands that aren't in the
    /// curated set under [`crate::cdp::domains`].
    pub async fn send_raw(&self, method: &str, params: Value) -> WebDriverResult<Value> {
        self.transport.send_raw(method, params).await
    }

    /// Open a WebSocket-backed CDP session for this driver session.
    ///
    /// Discovers the underlying CDP WebSocket from the session's
    /// capabilities (`se:cdp` or `goog:chromeOptions.debuggerAddress`),
    /// connects, and attaches to the active page target in flat mode.
    /// Required for event subscription.
    #[cfg(feature = "cdp-events")]
    pub async fn connect(&self) -> WebDriverResult<super::CdpSession> {
        super::CdpSession::connect(self.handle().clone()).await
    }

    /// `Browser.*` domain.
    pub fn browser(&self) -> domains::browser::BrowserDomain<'_> {
        domains::browser::BrowserDomain::new(self)
    }

    /// `Page.*` domain.
    pub fn page(&self) -> domains::page::PageDomain<'_> {
        domains::page::PageDomain::new(self)
    }

    /// `Network.*` domain.
    pub fn network(&self) -> domains::network::NetworkDomain<'_> {
        domains::network::NetworkDomain::new(self)
    }

    /// `Fetch.*` domain.
    pub fn fetch(&self) -> domains::fetch::FetchDomain<'_> {
        domains::fetch::FetchDomain::new(self)
    }

    /// `Runtime.*` domain.
    pub fn runtime(&self) -> domains::runtime::RuntimeDomain<'_> {
        domains::runtime::RuntimeDomain::new(self)
    }

    /// `DOM.*` domain.
    pub fn dom(&self) -> domains::dom::DomDomain<'_> {
        domains::dom::DomDomain::new(self)
    }

    /// `Emulation.*` domain.
    pub fn emulation(&self) -> domains::emulation::EmulationDomain<'_> {
        domains::emulation::EmulationDomain::new(self)
    }

    /// `Input.*` domain.
    pub fn input(&self) -> domains::input::InputDomain<'_> {
        domains::input::InputDomain::new(self)
    }

    /// `Target.*` domain.
    pub fn target(&self) -> domains::target::TargetDomain<'_> {
        domains::target::TargetDomain::new(self)
    }

    /// `Storage.*` domain.
    pub fn storage(&self) -> domains::storage::StorageDomain<'_> {
        domains::storage::StorageDomain::new(self)
    }

    /// `Log.*` domain.
    pub fn log(&self) -> domains::log::LogDomain<'_> {
        domains::log::LogDomain::new(self)
    }

    /// `Performance.*` domain.
    pub fn performance(&self) -> domains::performance::PerformanceDomain<'_> {
        domains::performance::PerformanceDomain::new(self)
    }
}
