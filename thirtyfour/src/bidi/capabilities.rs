//! Discovery of the BiDi WebSocket URL for a WebDriver session.
//!
//! BiDi is negotiated via the W3C `webSocketUrl: true` capability on
//! `New Session`. The driver responds with the actual `ws://...` URL on
//! the session capabilities, which is what we read here.

use std::sync::Arc;

use serde_json::Value;

use crate::error::{WebDriverError, WebDriverErrorInner, WebDriverResult};
use crate::session::handle::SessionHandle;

/// Resolve the BiDi WebSocket URL for the given session.
pub(crate) fn resolve_bidi_websocket_url(handle: &Arc<SessionHandle>) -> WebDriverResult<String> {
    let caps = handle.capabilities();

    if let Some(Value::String(url)) = caps.get("webSocketUrl") {
        return Ok(url.clone());
    }

    Err(WebDriverError::from_inner(WebDriverErrorInner::NotFound(
        "BiDi WebSocket URL".to_string(),
        "session capabilities did not include a `webSocketUrl` string — \
         did you call `caps.enable_bidi()` (or set `webSocketUrl: true` \
         manually) before opening the session, and is the driver \
         BiDi-capable (chromedriver ≥ 115, geckodriver ≥ 0.31)?"
            .to_string(),
    )))
}
