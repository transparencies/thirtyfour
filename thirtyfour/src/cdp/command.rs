//! Core traits that pair a CDP command's request type with its response
//! type, and an event type with its method name.
//!
//! Implement [`CdpCommand`] for any command not in the curated set under
//! [`crate::cdp::domains`]; the same `Cdp::send` and `CdpSession::send` paths
//! will pick it up. [`CdpEvent`] is the same idea for events delivered over
//! a WebSocket session.

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// A typed CDP command.
///
/// `Self` is the request params type (must be [`Serialize`]); [`Returns`] is
/// the response type returned by `Cdp::send` / `CdpSession::send`. `METHOD`
/// is the wire name, e.g. `"Page.navigate"`.
///
/// [`Returns`]: CdpCommand::Returns
///
/// # Example
/// ```
/// use serde::{Deserialize, Serialize};
/// use thirtyfour::cdp::CdpCommand;
///
/// #[derive(Serialize)]
/// struct GetTitleParams;
///
/// #[derive(Deserialize)]
/// struct GetTitleReturns {
///     title: String,
/// }
///
/// impl CdpCommand for GetTitleParams {
///     const METHOD: &'static str = "Page.getTitle";
///     type Returns = GetTitleReturns;
/// }
/// ```
pub trait CdpCommand: Serialize {
    /// Wire name of the command (e.g. `"Page.navigate"`).
    const METHOD: &'static str;
    /// Response type. Use [`Empty`] for commands that return `{}`.
    type Returns: DeserializeOwned;
}

/// A typed CDP event delivered over a [`CdpSession`].
///
/// [`CdpSession`]: crate::cdp::CdpSession
pub trait CdpEvent: DeserializeOwned + Clone + Send + Sync + 'static {
    /// Wire name of the event (e.g. `"Network.requestWillBeSent"`).
    const METHOD: &'static str;
}

/// Marker type for CDP commands whose response body is `{}`.
///
/// Many commands (`Network.enable`, `Page.reload`, `Emulation.clearDeviceMetricsOverride`,
/// etc.) return an empty object on success. Use this as the [`CdpCommand::Returns`]
/// type for those.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

/// A raw event delivered by [`crate::cdp::CdpSession::subscribe_all`].
#[derive(Debug, Clone)]
pub struct RawEvent {
    /// Wire name of the event.
    pub method: String,
    /// Event params as raw JSON.
    pub params: serde_json::Value,
    /// `sessionId` the event was routed to (flat-mode), if any.
    pub session_id: Option<crate::cdp::SessionId>,
}

// `CdpCommand` and `CdpEvent` are trait shells with no runtime behaviour;
// the only thing worth verifying is "does this command actually round-trip
// through a real browser", which is what the integration tests in
// `thirtyfour/tests/cdp_typed.rs` and `cdp_events.rs` cover.
