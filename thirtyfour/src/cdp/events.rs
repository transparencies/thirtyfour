//! Re-exports of typed CDP events from across [`crate::cdp::domains`],
//! grouped here so a single `use thirtyfour::cdp::events::*` covers the
//! common cases.
//!
//! Each event implements [`crate::cdp::CdpEvent`] and is the typed
//! payload for [`crate::cdp::CdpSession::subscribe`].
//!
//! Available with the `cdp-events` feature.

pub use crate::cdp::domains::fetch::RequestPaused;
pub use crate::cdp::domains::log::EntryAdded as LogEntryAdded;
pub use crate::cdp::domains::network::{
    LoadingFailed, LoadingFinished, RequestWillBeSent, ResponseReceived,
};
pub use crate::cdp::domains::page::{FrameNavigated, LifecycleEvent, LoadEventFired};
pub use crate::cdp::domains::runtime::{ConsoleApiCalled, ExceptionThrown};
pub use crate::cdp::domains::target::{AttachedToTarget, DetachedFromTarget};
