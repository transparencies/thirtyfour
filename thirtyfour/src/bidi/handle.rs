//! The [`BiDi`] handle — the entry point for WebDriver BiDi.
//!
//! Returned by [`crate::WebDriver::bidi`]. Cheap to clone (wraps an
//! `Arc`-shared transport).

use std::sync::Arc;

use serde_json::Value;

use super::BidiEvent;
use super::capabilities::resolve_bidi_websocket_url;
use super::command::BidiCommand;
use super::error::BidiError;
use super::events::{EventStream, RawEventStream};
use super::modules;
use super::transport::ws::BidiTransport;
use crate::error::WebDriverResult;
use crate::session::handle::SessionHandle;

/// WebDriver BiDi handle.
///
/// Cheap to clone; wraps an internal `Arc`. Open one with
/// [`crate::WebDriver::bidi`]; the WebSocket is connected lazily on first
/// call and reused thereafter.
///
/// # Example
/// ```no_run
/// # use thirtyfour::prelude::*;
/// # async fn run() -> WebDriverResult<()> {
/// let mut caps = DesiredCapabilities::chrome();
/// caps.enable_bidi()?;
/// let driver = WebDriver::new("http://localhost:4444", caps).await?;
///
/// let bidi = driver.bidi().await?;
/// let status = bidi.session().status().await?;
/// println!("ready: {}", status.ready);
/// # driver.quit().await }
/// ```
#[derive(Debug, Clone)]
pub struct BiDi {
    transport: BidiTransport,
}

impl BiDi {
    /// Connect to the BiDi WebSocket discovered from the session's
    /// capabilities.
    pub(crate) async fn connect(handle: Arc<SessionHandle>) -> WebDriverResult<Self> {
        let url = resolve_bidi_websocket_url(&handle)?;
        let transport = BidiTransport::connect(&url).await?;
        Ok(Self {
            transport,
        })
    }

    /// Send a typed BiDi command and decode its `result`.
    ///
    /// # Example
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # async fn run(driver: WebDriver) -> WebDriverResult<()> {
    /// use thirtyfour::bidi::modules::session::Status;
    /// let info = driver.bidi().await?.send(Status).await?;
    /// println!("{}", info.message);
    /// # Ok(()) }
    /// ```
    pub async fn send<C: BidiCommand>(&self, params: C) -> Result<C::Returns, BidiError> {
        let raw = self
            .transport
            .send_raw(C::METHOD, serde_json::to_value(params).map_err(serde_err)?)
            .await?;
        serde_json::from_value(raw).map_err(serde_err)
    }

    /// Send a BiDi command by name with raw JSON params, returning the raw
    /// `result` value. Useful for one-off commands that aren't in the
    /// curated set under [`crate::bidi::modules`].
    pub async fn send_raw(&self, method: &str, params: Value) -> Result<Value, BidiError> {
        self.transport.send_raw(method, params).await
    }

    /// Subscribe to a typed event. The driver only delivers events the
    /// client has explicitly subscribed to via `session.subscribe`; this
    /// method is just the local stream end. Use [`Self::session`] to send
    /// the subscription command.
    pub fn subscribe<E: BidiEvent>(&self) -> EventStream<E> {
        EventStream::new(self.transport.subscribe_events(), E::METHOD)
    }

    /// Subscribe to all events on the BiDi connection as raw `(method, params)`.
    pub fn subscribe_raw(&self) -> RawEventStream {
        RawEventStream::new(self.transport.subscribe_events())
    }

    /// `session.*` module facade (status, subscribe, end, …).
    pub fn session(&self) -> modules::session::SessionModule<'_> {
        modules::session::SessionModule::new(self)
    }

    /// `browser.*` module facade (close, user contexts).
    pub fn browser(&self) -> modules::browser::BrowserModule<'_> {
        modules::browser::BrowserModule::new(self)
    }

    /// `browsingContext.*` module facade (navigation, screenshots, tree …).
    pub fn browsing_context(&self) -> modules::browsing_context::BrowsingContextModule<'_> {
        modules::browsing_context::BrowsingContextModule::new(self)
    }

    /// `script.*` module facade (`evaluate`, `callFunction`, preload scripts).
    pub fn script(&self) -> modules::script::ScriptModule<'_> {
        modules::script::ScriptModule::new(self)
    }

    /// `network.*` module facade (interception, modify req/resp).
    pub fn network(&self) -> modules::network::NetworkModule<'_> {
        modules::network::NetworkModule::new(self)
    }

    /// `storage.*` module facade (cookies, partitions).
    pub fn storage(&self) -> modules::storage::StorageModule<'_> {
        modules::storage::StorageModule::new(self)
    }

    /// `log.*` module facade (event-only — no commands).
    pub fn log(&self) -> modules::log::LogModule<'_> {
        modules::log::LogModule::new(self)
    }

    /// `input.*` module facade (`performActions`, `releaseActions`, `setFiles`).
    pub fn input(&self) -> modules::input::InputModule<'_> {
        modules::input::InputModule::new(self)
    }

    /// `permissions.*` module facade (`setPermission`).
    pub fn permissions(&self) -> modules::permissions::PermissionsModule<'_> {
        modules::permissions::PermissionsModule::new(self)
    }
}

fn serde_err(e: serde_json::Error) -> BidiError {
    BidiError {
        command: "<serde>".to_string(),
        error: "unknown error".to_string(),
        message: e.to_string(),
        stacktrace: None,
    }
}
