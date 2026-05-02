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

use std::collections::HashSet;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use super::CdpCommand;
use super::SessionId;
use super::capabilities::resolve_cdp_websocket_url;
use super::command::RawEvent;
use super::error::CdpError;
use super::stream::{EventStream, RawEventStream};
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
    /// CDP domain `*.enable` methods we've already sent on this session,
    /// shared across clones so [`CdpSession::subscribe`] doesn't re-enable
    /// a domain that's already been enabled. Held in `Arc` so that a
    /// `clone()` of a `CdpSession` shares the same set.
    enabled: Arc<Mutex<HashSet<&'static str>>>,
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
            enabled: Arc::new(Mutex::new(HashSet::new())),
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
    ///
    /// `params` accepts anything `Serialize`. Pass `()` for no-arg commands.
    pub async fn send_raw<P: serde::Serialize>(
        &self,
        method: &str,
        params: P,
    ) -> Result<Value, CdpError> {
        let value = serde_json::to_value(params).map_err(serde_err)?;
        self.transport.send_raw_sessioned(method, value, self.session_id.as_ref()).await
    }

    /// Subscribe to a typed event for this session.
    ///
    /// Idempotently sends the event's domain-enable command (e.g.
    /// `Network.enable`, `Page.enable`, `Runtime.enable`) the first time
    /// any subscriber for that domain calls this method on this session.
    /// Events whose domain has no generic enable (`Target.*`, `Fetch.*`)
    /// declare `ENABLE = None`; the user must enable them manually.
    ///
    /// # Example
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # use futures_util::StreamExt;
    /// # async fn run(driver: WebDriver) -> WebDriverResult<()> {
    /// use thirtyfour::cdp::events::RequestWillBeSent;
    /// let session = driver.cdp().connect().await?;
    /// // No need for `session.send(network::Enable::default()).await` —
    /// // it's sent automatically the first time we ask for a Network event.
    /// let mut requests = session.subscribe::<RequestWillBeSent>().await?;
    /// while let Some(e) = requests.next().await {
    ///     println!("{}", e.document_url);
    /// }
    /// # Ok(()) }
    /// ```
    pub async fn subscribe<E: CdpEvent>(&self) -> Result<EventStream<E>, CdpError> {
        if let Some(enable_method) = E::ENABLE {
            let mut enabled = self.enabled.lock().await;
            if !enabled.contains(enable_method) {
                // Hold the lock across the wire call so concurrent
                // subscribers wait for the in-flight enable to ack
                // instead of double-enabling.
                self.send_raw(enable_method, serde_json::json!({})).await?;
                enabled.insert(enable_method);
            }
        }
        Ok(EventStream::new(self.transport.subscribe_events(), self.session_id.clone(), E::METHOD))
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
