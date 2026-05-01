//! Curated typed wrappers for the most-used CDP commands and events.
//!
//! This is a deliberately small, hand-maintained subset — covering the
//! commands users reach for in WebDriver-style automation (navigation,
//! network observation, JS evaluation, element/DOM resolution, device
//! emulation, input simulation). For anything outside this set, use
//! [`crate::cdp::Cdp::send_raw`] / [`crate::cdp::CdpSession::send_raw`],
//! or implement [`crate::cdp::CdpCommand`] yourself for compile-time
//! typing.
//!
//! See <https://chromedevtools.github.io/devtools-protocol/> for the full
//! protocol reference.
//!
//! # Contributing
//!
//! When adding a command or event:
//!
//! 1. **Verify wire field names against the live spec.** Most fields are
//!    `#[serde(rename_all = "camelCase")]`, but CDP uses SCREAMING
//!    acronyms (`documentURL`, `baseURL`, etc.) on a number of fields —
//!    these need explicit `#[serde(rename = "...")]`.
//! 2. **For closed string sets, use the internal `string_enum!` macro**
//!    (in `cdp/macros.rs`) rather than raw `String` fields. The macro
//!    generates a forward-compat `Unknown(String)` variant.
//! 3. **Use [`crate::cdp::Empty`] for commands that return `{}`.** A bare
//!    `()` does NOT deserialise from `{}`.
//! 4. **For optional params, use `Option<T>` with
//!    `#[serde(skip_serializing_if = "Option::is_none")]`.** Required
//!    params on commands like `Page.navigate` should be plain fields.
//! 5. **Add an integration test** in `thirtyfour/tests/cdp_typed.rs` (or
//!    `cdp_events.rs` for events) that exercises the new command/event
//!    against a real chromedriver via `WebDriver::managed`. Wire-shape
//!    unit tests are not accepted: they test the author's mental model
//!    of CDP rather than CDP itself.

pub mod browser;
pub mod dom;
pub mod emulation;
pub mod fetch;
pub mod input;
pub mod log;
pub mod network;
pub mod page;
pub mod performance;
pub mod runtime;
pub mod storage;
pub mod target;
