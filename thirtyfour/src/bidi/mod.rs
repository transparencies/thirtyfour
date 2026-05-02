//! [WebDriver BiDi] — W3C bidirectional WebDriver protocol.
//!
//! WebDriver BiDi is the cross-browser, event-driven sibling to WebDriver
//! Classic. It runs over a WebSocket that is negotiated by the standard
//! [`webSocketUrl: true`][websocket-cap] capability on `New Session`; the
//! driver responds with a `ws://…` URL the client connects to. Once
//! connected, every BiDi command is a typed JSON envelope with an
//! application-level `id`, and events are pushed to the client without
//! polling.
//!
//! `thirtyfour` ships curated typed bindings for every command and event
//! defined in the W3C BiDi specification, plus an untyped escape hatch for
//! anything browser-vendor-specific or newer than this crate.
//!
//! [`BiDi`][b] is the top-level handle returned by
//! [`WebDriver::bidi`](crate::WebDriver::bidi). It implements
//! [`BidiCommand`][bc]-based [`send`][b-send] and
//! [`subscribe`][b-sub] methods plus per-module facades.
//!
//! [b]: crate::bidi::BiDi
//! [bc]: crate::bidi::BidiCommand
//! [b-send]: crate::bidi::BiDi::send
//! [b-sub]: crate::bidi::BiDi::subscribe
//!
//! # Enabling
//!
//! BiDi is gated behind the `bidi` cargo feature **and** requires the
//! `webSocketUrl` capability on the session:
//!
//! ```toml
//! [dependencies]
//! thirtyfour = { version = "*", features = ["bidi"] }
//! ```
//!
//! ```no_run
//! # use thirtyfour::prelude::*;
//! # async fn run() -> WebDriverResult<()> {
//! let mut caps = DesiredCapabilities::chrome();
//! caps.enable_bidi()?;
//! let driver = WebDriver::new("http://localhost:4444", caps).await?;
//!
//! // Lazy-connect the BiDi WebSocket on first use; cached afterwards.
//! let bidi = driver.bidi().await?;
//! let status = bidi.session().status().await?;
//! assert!(status.ready || !status.message.is_empty());
//! # driver.quit().await }
//! ```
//!
//! [`WebDriver::bidi`](crate::WebDriver::bidi) is async because it lazily
//! dials the WebSocket on first use; subsequent calls clone the existing
//! handle. [`BiDi`][b] itself is `Clone` and cheap to share — the
//! underlying transport is `Arc`-backed.
//!
//! # Modules
//!
//! Each W3C BiDi module is mirrored by a Rust submodule under
//! [`modules`](crate::bidi::modules), and most are reachable via a
//! convenience facade on [`BiDi`][b].
//!
//! | BiDi module       | Rust module                                                       | Spec section                   |
//! |-------------------|-------------------------------------------------------------------|--------------------------------|
//! | `session`         | [`session`](crate::bidi::modules::session)                        | [§7.1][spec-session]           |
//! | `browser`         | [`browser`](crate::bidi::modules::browser)                        | [§7.2][spec-browser]           |
//! | `browsingContext` | [`browsing_context`](crate::bidi::modules::browsing_context)      | [§7.3][spec-bc]                |
//! | `emulation`       | [`emulation`](crate::bidi::modules::emulation)                    | [§7.4][spec-emulation]         |
//! | `network`         | [`network`](crate::bidi::modules::network)                        | [§7.5][spec-network]           |
//! | `script`          | [`script`](crate::bidi::modules::script)                          | [§7.6][spec-script]            |
//! | `storage`         | [`storage`](crate::bidi::modules::storage)                        | [§7.7][spec-storage]           |
//! | `log`             | [`log`](crate::bidi::modules::log)                                | [§7.8][spec-log]               |
//! | `input`           | [`input`](crate::bidi::modules::input)                            | [§7.9][spec-input]             |
//! | `webExtension`    | [`web_extension`](crate::bidi::modules::web_extension)            | [§7.10][spec-webext]           |
//! | `permissions`     | [`permissions`](crate::bidi::modules::permissions)                | [W3C Permissions][spec-perms]  |
//!
//! Every command in those modules is a Rust struct that implements
//! [`BidiCommand`][bc], pairing the request type with its response type
//! and the wire method name. Module facades wrap the most-used commands
//! in ergonomic async methods. For everything else, build the struct
//! directly and call [`BiDi::send`][b-send].
//!
//! # Events
//!
//! [`BiDi::subscribe::<E>()`](crate::bidi::BiDi::subscribe) returns a
//! [`futures::Stream`](futures_util::Stream) of typed events. Subscribing
//! also auto-sends the wire-level `session.subscribe` command for
//! `E::METHOD`; when the last stream for an event drops the framework
//! sends `session.unsubscribe` in the background.
//!
//! Common event types are re-exported from
//! [`events`](crate::bidi::events):
//!
//! ```no_run
//! # use thirtyfour::prelude::*;
//! # use futures_util::StreamExt;
//! # async fn run(driver: WebDriver) -> WebDriverResult<()> {
//! use thirtyfour::bidi::events::Load;
//! let bidi = driver.bidi().await?;
//! let mut loads = bidi.subscribe::<Load>().await?;
//! while let Some(event) = loads.next().await {
//!     println!("loaded: {}", event.url);
//! }
//! # Ok(()) }
//! ```
//!
//! See [`BiDi::subscribe_raw`](crate::bidi::BiDi::subscribe_raw) for an
//! untyped, no-deserialize firehose.
//!
//! # Untyped escape hatch
//!
//! Anything outside the curated set goes through
//! [`BiDi::send_raw`](crate::bidi::BiDi::send_raw) (commands) and
//! [`BiDi::subscribe_raw`](crate::bidi::BiDi::subscribe_raw) (events).
//! The wire shapes for command parameters and event payloads are
//! specified module-by-module in the [BiDi spec's modules section][spec-modules].
//!
//! # See also
//!
//! - [WebDriver BiDi specification][WebDriver BiDi]
//! - [Chrome DevTools Protocol support](crate::cdp) — Chromium-only
//!   counterpart, which can be used in the same session.
//!
//! [WebDriver BiDi]: https://w3c.github.io/webdriver-bidi/
//! [websocket-cap]: https://w3c.github.io/webdriver-bidi/#establishing
//! [spec-modules]: https://w3c.github.io/webdriver-bidi/#protocol-modules
//! [spec-session]: https://w3c.github.io/webdriver-bidi/#module-session
//! [spec-browser]: https://w3c.github.io/webdriver-bidi/#module-browser
//! [spec-bc]: https://w3c.github.io/webdriver-bidi/#module-browsingContext
//! [spec-emulation]: https://w3c.github.io/webdriver-bidi/#module-emulation
//! [spec-network]: https://w3c.github.io/webdriver-bidi/#module-network
//! [spec-script]: https://w3c.github.io/webdriver-bidi/#module-script
//! [spec-storage]: https://w3c.github.io/webdriver-bidi/#module-storage
//! [spec-log]: https://w3c.github.io/webdriver-bidi/#module-log
//! [spec-input]: https://w3c.github.io/webdriver-bidi/#module-input
//! [spec-webext]: https://w3c.github.io/webdriver-bidi/#module-webExtension
//! [spec-perms]: https://w3c.github.io/permissions/#webdriver-bidi-extension

pub mod events;
pub mod modules;

mod capabilities;
mod command;
mod error;
mod handle;
mod ids;
mod stream;
mod transport;

pub use command::{BidiCommand, BidiEvent, Empty, RawEvent};
pub use error::BidiError;
pub use handle::BiDi;
pub use ids::{
    BrowsingContextId, ChannelId, ClientWindowId, CollectorId, ExtensionId, InterceptId,
    NavigationId, NodeId, PreloadScriptId, RealmId, RequestId, UserContextId,
};
pub use stream::{EventStream, RawEventStream};
