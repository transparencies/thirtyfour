//! WebSocket-backed CDP session — supports event subscription.
//!
//! Built on top of [`crate::cdp::transport::ws::WsTransport`]. Each
//! `CdpSession` is bound to one CDP `sessionId`; the underlying WebSocket
//! and its reader/writer tasks are shared across all sessions on the same
//! connection.
//!
//! Session lifecycle:
//!
//! 1. [`CdpSession::connect`] resolves the browser-level WS URL via
//!    [`crate::cdp::capabilities`], dials, and attaches to the active page
//!    target with `Target.attachToTarget` `flatten: true`.
//! 2. [`CdpSession::send`] / [`Self::send_raw`] issue commands routed to
//!    this session's `sessionId`.
//! 3. [`CdpSession::subscribe`] / [`Self::subscribe_all`] return event
//!    streams scoped to this session.
//! 4. Drop / [`Self::detach`] sends `Target.detachFromTarget`.
//!
//! See module-level docs at [`crate::cdp`] for the protocol overview.

use std::sync::Arc;

use serde_json::Value;

use super::CdpCommand;
use super::SessionId;
use super::capabilities::resolve_cdp_websocket_url;
use super::command::RawEvent;
use super::error::CdpError;
use super::events::{EventStream, RawEventStream};
use super::transport::ws::WsTransport;
use crate::cdp::CdpEvent;
use crate::cdp::domains::target::{
    AttachToTarget, AttachToTargetResult, DetachFromTarget, GetTargets,
};
use crate::error::WebDriverResult;
use crate::session::handle::SessionHandle;

/// A WebSocket-backed CDP session.
///
/// Bound to one CDP `sessionId`. Cheap to clone — the underlying transport
/// is shared.
#[derive(Debug, Clone)]
pub struct CdpSession {
    transport: WsTransport,
    session_id: Option<SessionId>,
}

impl CdpSession {
    /// Connect to the browser-level CDP WebSocket and attach to the
    /// session's active page target in flat mode.
    pub(crate) async fn connect(handle: Arc<SessionHandle>) -> WebDriverResult<Self> {
        let url = resolve_cdp_websocket_url(&handle).await?;
        let transport = WsTransport::connect(&url).await?;

        // Browser-level session bootstrap: pick the first attached `page`
        // target and attach to it in flat mode. This produces a child
        // session id we route page-scoped commands through.
        let raw = transport
            .send_raw_sessioned(GetTargets::METHOD, serde_json::json!({}), None)
            .await
            .map_err(into_wde)?;
        let infos: super::domains::target::GetTargetsResult =
            serde_json::from_value(raw).map_err(|e| {
                crate::error::WebDriverError::Json(format!("Target.getTargets parse: {e}"))
            })?;
        let target =
            infos.target_infos.into_iter().find(|t| t.r#type == "page").ok_or_else(|| {
                crate::error::WebDriverError::from_inner(
                    crate::error::WebDriverErrorInner::NotFound(
                        "page target".to_string(),
                        "no page target found via Target.getTargets".to_string(),
                    ),
                )
            })?;

        let attach_params = AttachToTarget::flat(target.target_id);
        let raw = transport
            .send_raw_sessioned(AttachToTarget::METHOD, serde_json::to_value(&attach_params)?, None)
            .await
            .map_err(into_wde)?;
        let attached: AttachToTargetResult = serde_json::from_value(raw)?;

        Ok(Self {
            transport,
            session_id: Some(attached.session_id),
        })
    }

    /// The CDP `sessionId` this handle is bound to.
    pub fn session_id(&self) -> Option<&SessionId> {
        self.session_id.as_ref()
    }

    /// Send a typed command, scoped to this session.
    pub async fn send<C: CdpCommand>(&self, params: C) -> Result<C::Returns, CdpError> {
        let raw =
            self.send_raw(C::METHOD, serde_json::to_value(params).map_err(serde_err)?).await?;
        serde_json::from_value(raw).map_err(serde_err)
    }

    /// Send a raw command, scoped to this session.
    pub async fn send_raw(&self, method: &str, params: Value) -> Result<Value, CdpError> {
        self.transport.send_raw_sessioned(method, params, self.session_id.as_ref()).await
    }

    /// Subscribe to a typed event for this session.
    pub fn subscribe<E: CdpEvent>(&self) -> EventStream<E> {
        EventStream::new(self.transport.subscribe_events(), self.session_id.clone(), E::METHOD)
    }

    /// Subscribe to all events for this session as raw `(method, params)` pairs.
    pub fn subscribe_all(&self) -> RawEventStream {
        RawEventStream::new(self.transport.subscribe_events(), self.session_id.clone())
    }

    /// Subscribe to all events on the underlying WebSocket connection,
    /// regardless of session. Useful when auto-attach is enabled and you
    /// want to observe child sessions too.
    pub fn subscribe_connection(&self) -> tokio::sync::broadcast::Receiver<RawEvent> {
        self.transport.subscribe_events()
    }

    /// Send `Target.detachFromTarget` for this session and return.
    pub async fn detach(self) -> Result<(), CdpError> {
        if let Some(session) = self.session_id.clone() {
            let raw = self
                .transport
                .send_raw_sessioned(
                    DetachFromTarget::METHOD,
                    serde_json::to_value(DetachFromTarget {
                        session_id: Some(session),
                    })
                    .map_err(serde_err)?,
                    None,
                )
                .await?;
            // detach returns Empty; ignore the body
            drop(raw);
        }
        Ok(())
    }
}

fn into_wde(e: CdpError) -> crate::error::WebDriverError {
    crate::error::WebDriverError::from_inner(crate::error::WebDriverErrorInner::ParseError(
        e.to_string(),
    ))
}

fn serde_err(e: serde_json::Error) -> CdpError {
    CdpError {
        command: "<serde>".to_string(),
        code: -32603,
        message: e.to_string(),
        data: None,
    }
}
