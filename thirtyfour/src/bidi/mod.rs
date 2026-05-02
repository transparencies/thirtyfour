//! WebDriver BiDi (W3C bidirectional protocol) support.
//!
//! WebDriver BiDi is the W3C-standard cross-browser successor to parts of
//! Chrome DevTools Protocol. It runs over a WebSocket negotiated by the
//! `webSocketUrl: true` capability on `New Session`; the driver responds
//! with the actual `ws://...` URL on the session's capabilities.
//!
//! # Quick start
//!
//! ```no_run
//! # use thirtyfour::prelude::*;
//! # async fn run() -> WebDriverResult<()> {
//! let mut caps = DesiredCapabilities::chrome();
//! caps.enable_bidi()?;
//! let driver = WebDriver::new("http://localhost:4444", caps).await?;
//!
//! // Lazy-connect the BiDi WebSocket on first use; cached afterwards.
//! let bidi = driver.bidi().await?;
//! let status = bidi.session().status().await?;
//! assert!(status.ready || !status.message.is_empty());
//! # driver.quit().await }
//! ```
//!
//! # Modules
//!
//! Curated typed bindings live under [`modules`](crate::bidi::modules):
//!
//! - [`session`](crate::bidi::modules::session) — protocol handshake,
//!   subscription control, status.
//! - [`browser`](crate::bidi::modules::browser) — top-level browser (close,
//!   user contexts).
//! - [`browsing_context`](crate::bidi::modules::browsing_context) — tabs,
//!   frames, navigation, screenshots.
//! - [`script`](crate::bidi::modules::script) — `evaluate`, `callFunction`,
//!   preload scripts, realms.
//! - [`network`](crate::bidi::modules::network) — interception,
//!   modify-request/response, auth.
//! - [`storage`](crate::bidi::modules::storage) — cookies and partition
//!   lookup.
//! - [`log`](crate::bidi::modules::log) — log entry events.
//! - [`input`](crate::bidi::modules::input) — `performActions`,
//!   `releaseActions`, `setFiles`.
//! - [`permissions`](crate::bidi::modules::permissions) — `setPermission`.
//!
//! # Untyped escape hatch
//!
//! Anything outside the curated set goes through
//! [`BiDi::send_raw`](crate::bidi::BiDi::send_raw) /
//! [`BiDi::subscribe_raw`](crate::bidi::BiDi::subscribe_raw).

pub mod modules;

mod capabilities;
mod command;
mod error;
mod events;
mod handle;
mod ids;
mod transport;

pub use command::{BidiCommand, BidiEvent, Empty, RawEvent};
pub use error::BidiError;
pub use events::{EventStream, RawEventStream};
pub use handle::BiDi;
pub use ids::{
    BrowsingContextId, ChannelId, InterceptId, NavigationId, NodeId, PreloadScriptId, RealmId,
    RequestId, UserContextId,
};
