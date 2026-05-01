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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // A throwaway typed command used to exercise the trait shape.
    #[derive(Serialize)]
    struct Ping {
        message: String,
    }
    #[derive(Deserialize, Debug, PartialEq)]
    struct Pong {
        echoed: String,
    }
    impl CdpCommand for Ping {
        const METHOD: &'static str = "Test.ping";
        type Returns = Pong;
    }

    #[test]
    fn cdp_command_trait_round_trips_request_and_response() {
        let req = Ping {
            message: "hi".to_string(),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v, json!({"message": "hi"}));
        let resp: Pong = serde_json::from_value(json!({"echoed": "hi"})).unwrap();
        assert_eq!(
            resp,
            Pong {
                echoed: "hi".to_string()
            }
        );
        assert_eq!(Ping::METHOD, "Test.ping");
    }

    #[test]
    fn empty_round_trips() {
        let v = serde_json::to_value(Empty {}).unwrap();
        assert_eq!(v, json!({}));
        let _: Empty = serde_json::from_value(json!({})).unwrap();
    }

    #[test]
    fn empty_rejects_extra_fields() {
        // `deny_unknown_fields` keeps drift from sneaking in.
        assert!(serde_json::from_value::<Empty>(json!({"x": 1})).is_err());
    }

    #[derive(Deserialize, Debug, Clone)]
    struct ExampleEvent {
        v: String,
    }
    impl CdpEvent for ExampleEvent {
        const METHOD: &'static str = "Test.example";
    }

    #[test]
    fn cdp_event_trait_method_string() {
        assert_eq!(ExampleEvent::METHOD, "Test.example");
        let evt: ExampleEvent = serde_json::from_value(json!({"v": "hi"})).unwrap();
        assert_eq!(evt.v, "hi");
    }

    #[test]
    fn raw_event_clone_preserves_session_id() {
        let raw = RawEvent {
            method: "Net.x".to_string(),
            params: json!({"k": 1}),
            session_id: Some(crate::cdp::SessionId::from("S1")),
        };
        let cloned = raw.clone();
        assert_eq!(cloned.method, "Net.x");
        assert_eq!(cloned.session_id.unwrap().as_str(), "S1");
    }
}
