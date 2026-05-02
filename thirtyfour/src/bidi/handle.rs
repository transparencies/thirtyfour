//! The [`BiDi`] handle — the entry point for WebDriver BiDi.

use std::sync::Arc;

use serde_json::Value;

use super::BidiEvent;
use super::capabilities::resolve_bidi_websocket_url;
use super::command::BidiCommand;
use super::error::BidiError;
use super::modules;
use super::stream::{EventStream, RawEventStream};
use super::transport::ws::BidiTransport;
use crate::error::WebDriverResult;
use crate::session::handle::SessionHandle;

/// WebDriver BiDi connection handle.
///
/// `BiDi` owns the WebSocket connection to the driver's BiDi endpoint
/// and exposes typed access to every command and event via either:
///
/// - The module facades returned by [`session`](Self::session),
///   [`browser`](Self::browser),
///   [`browsing_context`](Self::browsing_context),
///   [`script`](Self::script), [`network`](Self::network),
///   [`storage`](Self::storage), [`log`](Self::log),
///   [`input`](Self::input), [`permissions`](Self::permissions),
///   [`emulation`](Self::emulation), and
///   [`web_extension`](Self::web_extension); or
/// - The lower-level [`send`](Self::send) /
///   [`subscribe::<E>()`](Self::subscribe) primitives.
///
/// For commands or events outside the curated set, fall back to
/// [`send_raw`](Self::send_raw) / [`subscribe_raw`](Self::subscribe_raw).
///
/// # Lifecycle and cloning
///
/// `BiDi` is cheap to clone — the underlying transport is `Arc`-wrapped
/// — so it's idiomatic to pass clones into spawned tasks. The
/// connection is established lazily by
/// [`WebDriver::bidi`](crate::WebDriver::bidi) on first use; subsequent
/// calls hand out clones of the same handle.
///
/// Subscription bookkeeping is reference-counted: each
/// [`subscribe::<E>()`](Self::subscribe) call bumps a per-event
/// counter, and the wire-level `session.unsubscribe` is sent
/// automatically when the last stream for that event drops.
///
/// # Example
///
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
    /// Resolve the BiDi WebSocket URL from session capabilities and
    /// connect.
    ///
    /// Internal: called once by
    /// [`WebDriver::bidi`](crate::WebDriver::bidi) and cached.
    pub(crate) async fn connect(handle: Arc<SessionHandle>) -> WebDriverResult<Self> {
        let url = resolve_bidi_websocket_url(&handle)?;
        let transport = BidiTransport::connect(&url).await?;
        Ok(Self {
            transport,
        })
    }

    /// Send a typed BiDi command and decode its `result`.
    ///
    /// `C: BidiCommand` carries both the wire method name (`C::METHOD`)
    /// and the response type (`C::Returns`), so this is a single
    /// generic surface for every command in the curated module set.
    ///
    /// Errors are returned as [`BidiError`], which mirrors the spec's
    /// error envelope (`error`, `message`, optional `stacktrace`).
    /// Serialise / deserialise failures are surfaced as a synthetic
    /// `<serde>` command error.
    ///
    /// # Example
    ///
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

    /// Send a BiDi command by name with the given params, returning the
    /// raw `result` value.
    ///
    /// Useful for one-off commands that aren't in the curated set under
    /// [`crate::bidi::modules`], or while prototyping a wire shape
    /// before promoting it to a typed [`BidiCommand`] impl.
    ///
    /// `params` accepts anything `Serialize`. Pass `()` for commands
    /// that take no parameters — the BiDi spec requires an object on
    /// the wire, so the transport coerces `null` to `{}`.
    pub async fn send_raw<P: serde::Serialize>(
        &self,
        method: &str,
        params: P,
    ) -> Result<Value, BidiError> {
        let value = serde_json::to_value(params).map_err(serde_err)?;
        self.transport.send_raw(method, value).await
    }

    /// Subscribe to a typed event.
    ///
    /// Sends `session.subscribe` for `E::METHOD` on first call (per
    /// connection) and returns a typed local stream. Subsequent calls
    /// for the same event share a refcount; `session.unsubscribe` is
    /// sent automatically when the last stream drops.
    ///
    /// For module-wide wildcards (e.g. `"browsingContext"` to receive
    /// every event in that module) there's no single typed event to
    /// dispatch on — fall back to [`Self::session`]`.subscribe(...)`
    /// followed by [`Self::subscribe_raw`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # use futures_util::StreamExt;
    /// # async fn run(driver: WebDriver) -> WebDriverResult<()> {
    /// use thirtyfour::bidi::events::Load;
    /// let bidi = driver.bidi().await?;
    /// let mut loads = bidi.subscribe::<Load>().await?;
    /// while let Some(event) = loads.next().await {
    ///     println!("loaded: {}", event.url);
    /// }
    /// # Ok(()) }
    /// ```
    pub async fn subscribe<E: BidiEvent>(&self) -> Result<EventStream<E>, BidiError> {
        self.transport.ensure_subscribed(E::METHOD).await?;
        Ok(EventStream::new(self.transport.clone(), self.transport.subscribe_events(), E::METHOD))
    }

    /// Subscribe to every event on the BiDi connection as a raw
    /// `(method, params)` stream.
    ///
    /// You're responsible for sending the matching `session.subscribe`
    /// commands first via [`Self::session`] — this method only returns
    /// the local stream end and does **no** wire-level subscription on
    /// its own.
    pub fn subscribe_raw(&self) -> RawEventStream {
        RawEventStream::new(self.transport.subscribe_events())
    }

    /// Return the [`session.*`](crate::bidi::modules::session) facade
    /// (status, subscribe, end).
    pub fn session(&self) -> modules::session::SessionModule<'_> {
        modules::session::SessionModule::new(self)
    }

    /// Return the [`browser.*`](crate::bidi::modules::browser) facade
    /// (close, user contexts, client windows, download behavior).
    pub fn browser(&self) -> modules::browser::BrowserModule<'_> {
        modules::browser::BrowserModule::new(self)
    }

    /// Return the [`browsingContext.*`](crate::bidi::modules::browsing_context)
    /// facade (navigate, screenshots, tree, locate nodes, print, …).
    pub fn browsing_context(&self) -> modules::browsing_context::BrowsingContextModule<'_> {
        modules::browsing_context::BrowsingContextModule::new(self)
    }

    /// Return the [`script.*`](crate::bidi::modules::script) facade
    /// (`evaluate`, `callFunction`, preload scripts, realms).
    pub fn script(&self) -> modules::script::ScriptModule<'_> {
        modules::script::ScriptModule::new(self)
    }

    /// Return the [`network.*`](crate::bidi::modules::network) facade
    /// (interception, modify req/resp, data collectors, extra headers).
    pub fn network(&self) -> modules::network::NetworkModule<'_> {
        modules::network::NetworkModule::new(self)
    }

    /// Return the [`storage.*`](crate::bidi::modules::storage) facade
    /// (cookies, partitions).
    pub fn storage(&self) -> modules::storage::StorageModule<'_> {
        modules::storage::StorageModule::new(self)
    }

    /// Return the [`log.*`](crate::bidi::modules::log) facade
    /// (event-only — subscribe via
    /// [`subscribe::<LogEntryAdded>()`](Self::subscribe)).
    pub fn log(&self) -> modules::log::LogModule<'_> {
        modules::log::LogModule::new(self)
    }

    /// Return the [`input.*`](crate::bidi::modules::input) facade
    /// (`performActions`, `releaseActions`, `setFiles`).
    pub fn input(&self) -> modules::input::InputModule<'_> {
        modules::input::InputModule::new(self)
    }

    /// Return the [`permissions.*`](crate::bidi::modules::permissions)
    /// facade (`setPermission`).
    pub fn permissions(&self) -> modules::permissions::PermissionsModule<'_> {
        modules::permissions::PermissionsModule::new(self)
    }

    /// Return the [`emulation.*`](crate::bidi::modules::emulation)
    /// facade (geolocation, locale, timezone, screen, user-agent
    /// overrides).
    pub fn emulation(&self) -> modules::emulation::EmulationModule<'_> {
        modules::emulation::EmulationModule::new(self)
    }

    /// Return the [`webExtension.*`](crate::bidi::modules::web_extension)
    /// facade (install / uninstall extensions).
    pub fn web_extension(&self) -> modules::web_extension::WebExtensionModule<'_> {
        modules::web_extension::WebExtensionModule::new(self)
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
