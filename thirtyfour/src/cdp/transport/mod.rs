//! Transport abstraction for CDP commands.
//!
//! Two implementations:
//!
//! - [`http`] sends commands through the WebDriver vendor endpoint
//!   `POST /session/{id}/goog/cdp/execute`. Synchronous request/response;
//!   cannot deliver events. This is the default used by [`crate::cdp::Cdp`].
//! - [`ws`] (feature `cdp-events`) opens a CDP WebSocket directly to the
//!   underlying browser and uses flat session mode for multiplexing. This
//!   is what powers [`crate::cdp::CdpSession`] and event subscription.
//!
//! The trait-based abstraction is also the entry point for a future
//! WebDriver BiDi layer: BiDi has the same request/response shape over a
//! WebSocket, just with a different envelope.

pub(crate) mod http;
#[cfg(feature = "cdp-events")]
pub(crate) mod ws;

use crate::error::WebDriverResult;
use serde_json::Value;

/// A transport capable of issuing CDP commands.
///
/// Sealed: implementations live entirely inside the crate. End users go
/// through [`crate::cdp::Cdp`] or [`crate::cdp::CdpSession`].
#[allow(async_fn_in_trait)] // sealed; never used as &dyn Transport.
pub(crate) trait Transport: private::Sealed {
    async fn send_raw(&self, method: &str, params: Value) -> WebDriverResult<Value>;
}

mod private {
    pub trait Sealed {}
}

impl private::Sealed for http::HttpTransport {}
#[cfg(feature = "cdp-events")]
impl private::Sealed for ws::WsTransport {}
