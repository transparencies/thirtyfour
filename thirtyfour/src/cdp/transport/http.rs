//! HTTP transport for CDP, layered on the WebDriver vendor endpoint
//! `POST /session/{id}/goog/cdp/execute`.
//!
//! Each command is wrapped in `{"cmd": <method>, "params": <object>}` and
//! posted to the WebDriver session; chromedriver returns the CDP `result`
//! field as the WebDriver `value`. Reuses the existing
//! [`SessionHandle::cmd`] machinery so it shares the HTTP client, session
//! id, and config with the surrounding [`crate::WebDriver`].
//!
//! The [`Transport`] trait abstraction here exists so [`crate::cdp::Cdp`]
//! and the WebSocket-based [`crate::cdp::CdpSession`] (feature
//! `cdp-events`) can share one `send_raw` shape.

use std::sync::Arc;

use http::Method;
use serde_json::{Value, json};

use super::Transport;
use crate::common::command::FormatRequestData;
use crate::common::requestdata::RequestData;
use crate::common::types::SessionId;
use crate::error::WebDriverResult;
use crate::session::handle::SessionHandle;

/// HTTP transport that forwards each command to the chromedriver vendor
/// endpoint. Holds an `Arc<SessionHandle>` so it shares the same HTTP client,
/// session id, and config as the surrounding [`crate::WebDriver`].
#[derive(Debug, Clone)]
pub(crate) struct HttpTransport {
    pub(crate) handle: Arc<SessionHandle>,
}

impl HttpTransport {
    pub(crate) fn new(handle: Arc<SessionHandle>) -> Self {
        Self {
            handle,
        }
    }

    pub(crate) fn handle(&self) -> &Arc<SessionHandle> {
        &self.handle
    }
}

impl Transport for HttpTransport {
    async fn send_raw(&self, method: &str, params: Value) -> WebDriverResult<Value> {
        // chromedriver rejects `"params": null` with `invalid argument:
        // params not passed`. Unit-struct CDP commands (Browser.getVersion,
        // Network.clearBrowserCache, etc.) serialise to `null`, so coerce to
        // the empty object that the wire format expects.
        let params = if params.is_null() {
            json!({})
        } else {
            params
        };
        let cmd = ExecuteCdp {
            method: method.to_string(),
            params,
        };
        let response = self.handle.cmd(cmd).await?;
        // chromedriver wraps the CDP `result` field directly under "value"
        // in the WebDriver envelope. `value_json` strips that wrapper.
        response.value_json()
    }
}

/// Internal request shape for the WebDriver vendor endpoint.
#[derive(Debug)]
struct ExecuteCdp {
    method: String,
    params: Value,
}

impl FormatRequestData for ExecuteCdp {
    fn format_request(&self, session_id: &SessionId) -> RequestData {
        RequestData::new(Method::POST, format!("/session/{}/goog/cdp/execute", session_id))
            .add_body(json!({
                "cmd": self.method,
                "params": self.params,
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::types::SessionId;

    fn body_of(cmd: &ExecuteCdp) -> Value {
        let session = SessionId::from("S1".to_string());
        cmd.format_request(&session).body.clone().unwrap_or(Value::Null)
    }

    #[test]
    fn format_request_routes_to_goog_cdp_execute() {
        let session = SessionId::from("XYZ".to_string());
        let cmd = ExecuteCdp {
            method: "Browser.getVersion".to_string(),
            params: json!({}),
        };
        let req = cmd.format_request(&session);
        assert_eq!(&*req.uri, "/session/XYZ/goog/cdp/execute");
    }

    #[test]
    fn format_request_carries_cmd_and_params() {
        let cmd = ExecuteCdp {
            method: "Network.setExtraHTTPHeaders".to_string(),
            params: json!({"headers": {"X": "Y"}}),
        };
        let body = body_of(&cmd);
        assert_eq!(body["cmd"], "Network.setExtraHTTPHeaders");
        assert_eq!(body["params"]["headers"]["X"], "Y");
    }

    #[test]
    fn format_request_does_not_mutate_null_params_at_format_time() {
        // The null-to-{} coercion lives in `Transport::send_raw`, not in
        // `format_request`. This test pins that contract — if the coercion
        // ever moves down the stack, lots of upstream call sites would
        // need to be revisited.
        let cmd = ExecuteCdp {
            method: "X.y".to_string(),
            params: Value::Null,
        };
        let body = body_of(&cmd);
        assert!(body["params"].is_null());
    }
}
