//! Legacy compatibility shim for the original CDP API.
//!
//! New code should use [`crate::cdp`] directly:
//!
//! - [`crate::cdp::Cdp`] for typed request/response.
//! - [`crate::cdp::CdpSession`] for WebSocket-based event subscription
//!   (feature `cdp-events`).
//! - [`crate::WebDriver::cdp`] for the entry point.
//!
//! Everything in this module is deprecated and will be removed in a future
//! release.

#[allow(deprecated)]
mod chromecommand;
#[allow(deprecated)]
mod devtools;
#[allow(deprecated)]
mod networkconditions;

#[allow(deprecated)]
pub use chromecommand::ChromeCommand;
#[allow(deprecated)]
pub use devtools::ChromeDevTools;
#[allow(deprecated)]
pub use networkconditions::{ConnectionType, NetworkConditions};
