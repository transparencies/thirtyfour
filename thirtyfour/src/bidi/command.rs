//! Core traits that pair a BiDi command's request type with its response
//! type, and an event type with its method name.
//!
//! Implement [`BidiCommand`] for any command not in the curated set under
//! [`crate::bidi::modules`]; the same `BiDi::send` / `EventStream`
//! infrastructure will pick it up. [`BidiEvent`] is the same idea for events
//! delivered to an active subscription.

use serde::Serialize;
use serde::de::DeserializeOwned;

pub use crate::common::protocol::Empty;

/// A typed BiDi command.
///
/// `Self` is the request `params` type (must be [`Serialize`]); [`Returns`]
/// is the typed `result` returned by [`crate::bidi::BiDi::send`]. `METHOD`
/// is the wire name, e.g. `"browsingContext.navigate"`.
///
/// [`Returns`]: BidiCommand::Returns
///
/// # Example
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
    /// Response `result` type. Use [`Empty`] for commands that return `{}`.
    type Returns: DeserializeOwned;
}

/// A typed BiDi event delivered through [`crate::bidi::BiDi::subscribe`].
pub trait BidiEvent: DeserializeOwned + Clone + Send + Sync + 'static {
    /// Wire name of the event (e.g. `"browsingContext.load"`).
    const METHOD: &'static str;
}

/// A raw event delivered by [`crate::bidi::BiDi::subscribe_raw`].
#[derive(Debug, Clone)]
pub struct RawEvent {
    /// Wire name of the event (e.g. `"network.beforeRequestSent"`).
    pub method: String,
    /// Event params as raw JSON.
    pub params: serde_json::Value,
}
