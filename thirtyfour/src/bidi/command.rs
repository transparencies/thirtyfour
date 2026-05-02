//! Core traits and marker types for BiDi commands and events.
//!
//! Every typed command implements [`BidiCommand`], pairing the request
//! type with its response type and the wire method name. Every typed
//! event implements [`BidiEvent`].
//!
//! Implement these traits for any command or event not in the curated
//! set under [`crate::bidi::modules`] — the framework picks them up
//! automatically via [`BiDi::send`](crate::bidi::BiDi::send) and
//! [`BiDi::subscribe`](crate::bidi::BiDi::subscribe). There is no
//! second-class API.

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// A typed BiDi command.
///
/// `Self` is the request `params` type (must be [`Serialize`]).
/// [`Returns`](BidiCommand::Returns) is the deserialised `result` type
/// returned by [`BiDi::send`](crate::bidi::BiDi::send).
/// [`METHOD`](BidiCommand::METHOD) is the spec-defined wire name
/// (e.g. `"browsingContext.navigate"`).
///
/// # Example
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use thirtyfour::bidi::BidiCommand;
///
/// #[derive(Serialize)]
/// struct NavigateParams {
///     context: String,
///     url: String,
/// }
///
/// #[derive(Deserialize)]
/// struct NavigateResult {
///     navigation: Option<String>,
///     url: String,
/// }
///
/// impl BidiCommand for NavigateParams {
///     const METHOD: &'static str = "browsingContext.navigate";
///     type Returns = NavigateResult;
/// }
/// ```
pub trait BidiCommand: Serialize {
    /// Wire name of the command (e.g. `"browsingContext.navigate"`).
    const METHOD: &'static str;
    /// Response `result` type. Use [`Empty`] for commands that return
    /// an empty object (`{}`).
    type Returns: DeserializeOwned;
}

/// A typed BiDi event.
///
/// Implementations are deserialised into the `Self` type and delivered
/// through [`BiDi::subscribe::<E>()`](crate::bidi::BiDi::subscribe).
/// `METHOD` is the spec-defined wire name
/// (e.g. `"browsingContext.load"`).
pub trait BidiEvent: DeserializeOwned + Clone + Send + Sync + 'static {
    /// Wire name of the event (e.g. `"browsingContext.load"`).
    const METHOD: &'static str;
}

/// Marker type for BiDi commands whose response body is the empty
/// object `{}`.
///
/// Many commands (`session.subscribe`, `browsingContext.close`,
/// `script.removePreloadScript`, every `emulation.*` setter, …)
/// return `{}` on success. Implement [`BidiCommand`] with
/// `type Returns = Empty;` to model that.
///
/// `Empty` is `Deserialize` with `deny_unknown_fields`, so a non-empty
/// response body would be a deserialise error — useful as a defensive
/// check that the spec hasn't started returning meaningful data.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

/// A raw event delivered by
/// [`BiDi::subscribe_raw`](crate::bidi::BiDi::subscribe_raw).
///
/// No filtering or deserialisation is performed — every event the
/// driver pushes is surfaced as-is. Useful for debugging, dump
/// recording, or implementing a typed event that hasn't been added to
/// the curated set yet.
#[derive(Debug, Clone)]
pub struct RawEvent {
    /// Wire name of the event (e.g. `"network.beforeRequestSent"`).
    pub method: String,
    /// Event `params` as raw JSON.
    pub params: serde_json::Value,
}
