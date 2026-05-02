//! Curated typed bindings for every WebDriver BiDi module.
//!
//! Each submodule mirrors a [W3C BiDi spec module][spec-modules]:
//!
//! - Commands implement [`BidiCommand`](crate::bidi::BidiCommand) and are
//!   sent via [`BiDi::send`](crate::bidi::BiDi::send) (or the matching
//!   façade method).
//! - Events implement [`BidiEvent`](crate::bidi::BidiEvent) and are
//!   surfaced via [`BiDi::subscribe`](crate::bidi::BiDi::subscribe).
//!
//! Each module's docs link directly to the corresponding spec section
//! and every command/event has a link to its own spec entry. Where the
//! spec talks about CDDL "remote end" / "local end" definitions, the
//! rule of thumb is:
//!
//! - **Remote end definition** → command parameters / event params
//!   _sent or received by the client_. Modelled as a `Serialize` /
//!   `Deserialize` struct on the Rust side.
//! - **Local end definition** → the response the client deserialises.
//!   Modelled as the `Returns` associated type of the command.
//!
//! [spec-modules]: https://w3c.github.io/webdriver-bidi/#protocol-modules

/// `browser.*` — top-level browser control, user contexts, client windows,
/// download behavior. See [`browser`].
pub mod browser;
/// `browsingContext.*` — tabs, frames, navigation, screenshots, printing,
/// node location, CSP bypass. See [`browsing_context`].
pub mod browsing_context;
/// `emulation.*` — geolocation, locale, timezone, screen, user agent,
/// scripting and touch overrides. See [`emulation`].
pub mod emulation;
/// `input.*` — `performActions`, `releaseActions`, `setFiles`, plus the
/// `fileDialogOpened` event. See [`input`].
pub mod input;
/// `log.*` — log entry events. See [`log`].
pub mod log;
/// `network.*` — interception, request/response control, data collectors,
/// extra headers. See [`network`].
pub mod network;
/// `permissions.*` — `setPermission`. See [`permissions`].
pub mod permissions;
/// `script.*` — `evaluate`, `callFunction`, preload scripts, realms.
/// See [`script`].
pub mod script;
/// `session.*` — protocol handshake, subscription, status. See [`session`].
pub mod session;
/// `storage.*` — cookies and partition lookup. See [`storage`].
pub mod storage;
/// `webExtension.*` — install / uninstall web extensions. See [`web_extension`].
pub mod web_extension;
