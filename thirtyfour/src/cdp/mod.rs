//! Chrome DevTools Protocol (CDP) support.
//!
//! Two entry points:
//!
//! - [`Cdp`](crate::cdp::Cdp) — request/response over the WebDriver vendor
//!   endpoint `goog/cdp/execute`. Available on any Chromium-based session
//!   (chromedriver, msedgedriver, Brave, Opera, etc.). Get one with
//!   [`crate::WebDriver::cdp`].
//! - [`CdpSession`](crate::cdp::CdpSession) (feature `cdp-events`) — a
//!   WebSocket-backed flat-mode CDP session with **event subscription**.
//!   Open one with [`Cdp::connect`](crate::cdp::Cdp::connect) when you need
//!   to listen for events like `Network.requestWillBeSent` or
//!   `Page.lifecycleEvent`.
//!
//! ## Typed vs untyped
//!
//! Every command in [`domains`](crate::cdp::domains) is a Rust struct that
//! implements [`CdpCommand`](crate::cdp::CdpCommand), pairing the request
//! type with its response type and the wire method name. Calls flow through
//! one entry point:
//!
//! ```no_run
//! # use thirtyfour::prelude::*;
//! # async fn run(driver: WebDriver) -> WebDriverResult<()> {
//! use thirtyfour::cdp::domains::network::NetworkConditions;
//! driver.cdp().network().clear_browser_cache().await?;
//! driver.cdp().network().emulate_network_conditions(NetworkConditions {
//!     offline: false,
//!     latency: 200,
//!     download_throughput: 256 * 1024,
//!     upload_throughput: 64 * 1024,
//!     connection_type: None,
//! }).await?;
//! # Ok(()) }
//! ```
//!
//! For commands not in the curated set under [`domains`](crate::cdp::domains),
//! use the untyped escape hatch [`Cdp::send_raw`](crate::cdp::Cdp::send_raw)
//! or implement [`CdpCommand`](crate::cdp::CdpCommand) yourself.

pub mod domains;

mod command;
mod error;
mod handle;
mod ids;
mod transport;

#[cfg(feature = "cdp-events")]
mod capabilities;
#[cfg(feature = "cdp-events")]
mod events;
#[cfg(feature = "cdp-events")]
mod session;

pub use command::{CdpCommand, CdpEvent, Empty, RawEvent};
pub use error::CdpError;
pub use handle::Cdp;
pub use ids::{
    BackendNodeId, BrowserContextId, ExecutionContextId, FetchRequestId, FrameId, InterceptionId,
    LoaderId, NodeId, RemoteObjectId, RequestId, ScriptId, SessionId, TargetId, Timestamp,
};

#[cfg(feature = "cdp-events")]
pub use events::EventStream;
#[cfg(feature = "cdp-events")]
pub use session::CdpSession;
