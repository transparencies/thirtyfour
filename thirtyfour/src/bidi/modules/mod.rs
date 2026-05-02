//! Curated typed bindings for WebDriver BiDi modules.
//!
//! Each submodule mirrors a [W3C BiDi spec module]. Commands implement
//! [`crate::bidi::BidiCommand`] and events implement
//! [`crate::bidi::BidiEvent`]; both flow through [`crate::bidi::BiDi`].
//!
//! [W3C BiDi spec module]: https://w3c.github.io/webdriver-bidi/

/// `browser.*` — top-level browser control and user contexts.
pub mod browser;
/// `browsingContext.*` — tabs, frames, navigation, screenshots.
pub mod browsing_context;
/// `input.*` — `performActions`, `releaseActions`, `setFiles`.
pub mod input;
/// `log.*` — log entry events.
pub mod log;
/// `network.*` — interception and request/response control.
pub mod network;
/// `permissions.*` — `setPermission`.
pub mod permissions;
/// `script.*` — `evaluate`, `callFunction`, preload scripts, realms.
pub mod script;
/// `session.*` — protocol handshake, subscription, status.
pub mod session;
/// `storage.*` — cookies and partitions.
pub mod storage;
