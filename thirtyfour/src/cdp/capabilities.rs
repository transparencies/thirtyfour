//! Discovery of the underlying CDP WebSocket URL for a WebDriver session.
//!
//! Tried in order:
//!
//! 1. `se:cdp` capability — set by Selenium Grid.
//! 2. `webSocketDebuggerUrl` field on the session — some drivers expose it
//!    directly. (The W3C `webSocketUrl` capability is BiDi, **not** CDP, and
//!    is intentionally NOT used here.)
//! 3. `goog:chromeOptions.debuggerAddress` (or the equivalent `ms:edgeOptions`
//!    field for Edge) → `GET http://<addr>/json/version` →
//!    `webSocketDebuggerUrl`.
//!
//! All three paths converge on the **browser-level** CDP WebSocket URL of
//! the form `ws://host:port/devtools/browser/<uuid>`. From there we use
//! flat session mode (`Target.attachToTarget` `flatten: true`) to multiplex
//! many sessions over the one connection.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::error::{WebDriverError, WebDriverErrorInner, WebDriverResult};
use crate::session::handle::SessionHandle;

#[derive(Debug, Deserialize)]
struct VersionInfo {
    #[serde(rename = "webSocketDebuggerUrl")]
    web_socket_debugger_url: Option<String>,
}

/// Resolve the CDP WebSocket URL for the given session.
pub(crate) async fn resolve_cdp_websocket_url(
    handle: &Arc<SessionHandle>,
) -> WebDriverResult<String> {
    let caps = handle.capabilities();

    // 1. `se:cdp` (Selenium Grid).
    if let Some(Value::String(url)) = caps.get("se:cdp") {
        return Ok(url.clone());
    }

    // 2. `webSocketDebuggerUrl` directly on the caps (some drivers).
    if let Some(Value::String(url)) = caps.get("webSocketDebuggerUrl") {
        return Ok(url.clone());
    }

    // 3. `goog:chromeOptions.debuggerAddress` or `ms:edgeOptions.debuggerAddress`.
    let debugger_address = caps
        .get("goog:chromeOptions")
        .and_then(|v| v.get("debuggerAddress"))
        .and_then(Value::as_str)
        .or_else(|| {
            caps.get("ms:edgeOptions")
                .and_then(|v| v.get("debuggerAddress"))
                .and_then(Value::as_str)
        })
        .ok_or_else(|| not_found("debuggerAddress not present in session capabilities"))?
        .to_string();

    let url = format!("http://{debugger_address}/json/version");

    // We can reuse the existing HTTP client. Build a `GET /json/version`
    // request directly — RequestData/SessionHandle::cmd is anchored to the
    // WebDriver server URL and we need to hit the browser's debug port instead.
    use bytes::Bytes;
    use http::Method;
    let request = http::Request::builder()
        .method(Method::GET)
        .uri(&url)
        .body(crate::session::http::Body::Empty)
        .map_err(|e| WebDriverError::ParseError(format!("invalid /json/version URL: {e}")))?;

    let response = handle.client.send(request).await?;
    let bytes: &Bytes = response.body();
    let info: VersionInfo = serde_json::from_slice(bytes)
        .map_err(|e| WebDriverError::Json(format!("/json/version parse error: {e}")))?;
    info.web_socket_debugger_url
        .ok_or_else(|| not_found("/json/version did not contain webSocketDebuggerUrl"))
}

fn not_found(msg: &str) -> WebDriverError {
    WebDriverError::from_inner(WebDriverErrorInner::NotFound(
        "CDP WebSocket URL".to_string(),
        msg.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Capabilities;
    use serde_json::json;

    /// Replicate the priority order used in `resolve_cdp_websocket_url` but
    /// skip the `/json/version` HTTP fallback (which is exercised by
    /// integration tests). Lets us pin the priority logic in unit tests.
    fn lookup_static_caps(caps: &Capabilities) -> Option<String> {
        if let Some(Value::String(url)) = caps.get("se:cdp") {
            return Some(url.clone());
        }
        if let Some(Value::String(url)) = caps.get("webSocketDebuggerUrl") {
            return Some(url.clone());
        }
        None
    }

    fn caps_with(pairs: &[(&str, Value)]) -> Capabilities {
        let mut c = Capabilities::new();
        for (k, v) in pairs {
            c.set(*k, v.clone()).unwrap();
        }
        c
    }

    #[test]
    fn se_cdp_capability_wins_first() {
        let caps = caps_with(&[
            ("se:cdp", json!("ws://grid/cdp")),
            ("webSocketDebuggerUrl", json!("ws://other/cdp")),
        ]);
        assert_eq!(lookup_static_caps(&caps).as_deref(), Some("ws://grid/cdp"));
    }

    #[test]
    fn web_socket_debugger_url_used_when_no_se_cdp() {
        let caps = caps_with(&[("webSocketDebuggerUrl", json!("ws://chrome/cdp"))]);
        assert_eq!(lookup_static_caps(&caps).as_deref(), Some("ws://chrome/cdp"));
    }

    #[test]
    fn falls_through_when_static_keys_absent() {
        let caps =
            caps_with(&[("goog:chromeOptions", json!({"debuggerAddress": "127.0.0.1:9222"}))]);
        // Static lookup returns None — `resolve_cdp_websocket_url` would
        // then hit `/json/version` which the unit test doesn't exercise.
        assert!(lookup_static_caps(&caps).is_none());
    }

    #[test]
    fn debugger_address_extraction_prefers_goog_then_edge() {
        let goog =
            caps_with(&[("goog:chromeOptions", json!({"debuggerAddress": "127.0.0.1:9222"}))]);
        let from_goog = goog
            .get("goog:chromeOptions")
            .and_then(|v| v.get("debuggerAddress"))
            .and_then(Value::as_str);
        assert_eq!(from_goog, Some("127.0.0.1:9222"));

        let edge = caps_with(&[("ms:edgeOptions", json!({"debuggerAddress": "127.0.0.1:9223"}))]);
        let from_edge = edge
            .get("ms:edgeOptions")
            .and_then(|v| v.get("debuggerAddress"))
            .and_then(Value::as_str);
        assert_eq!(from_edge, Some("127.0.0.1:9223"));
    }

    #[test]
    fn version_info_parses_with_url_present() {
        let body = json!({
            "Browser": "Chrome/121.0",
            "Protocol-Version": "1.3",
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/browser/ABC"
        });
        let info: VersionInfo = serde_json::from_value(body).unwrap();
        assert_eq!(
            info.web_socket_debugger_url.as_deref(),
            Some("ws://127.0.0.1:9222/devtools/browser/ABC")
        );
    }

    #[test]
    fn version_info_missing_url_is_none() {
        let body = json!({"Browser": "Chrome/121.0"});
        let info: VersionInfo = serde_json::from_value(body).unwrap();
        assert!(info.web_socket_debugger_url.is_none());
    }
}
