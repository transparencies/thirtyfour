//! Typed BiDi events — re-exported from each module's `events`
//! submodule for convenience.
//!
//! Each event implements [`BidiEvent`](crate::bidi::BidiEvent) and is
//! the typed payload for
//! [`BiDi::subscribe::<E>()`](crate::bidi::BiDi::subscribe). Names are
//! renamed where the bare W3C ones would collide
//! (`script.message` → [`ScriptMessage`],
//! `log.entryAdded` → [`LogEntryAdded`]).
//!
//! See the [W3C events section][spec] for the canonical list of events
//! and their wire shapes.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#events

pub use crate::bidi::modules::browsing_context::events::{
    ContextCreated, ContextDestroyed, DomContentLoaded, DownloadEnd, DownloadWillBegin,
    FragmentNavigated, HistoryUpdated, Load, NavigationAborted, NavigationCommitted,
    NavigationFailed, NavigationStarted, UserPromptClosed, UserPromptOpened,
};
pub use crate::bidi::modules::input::events::FileDialogOpened;
pub use crate::bidi::modules::log::events::EntryAdded as LogEntryAdded;
pub use crate::bidi::modules::network::events::{
    AuthRequired, BeforeRequestSent, FetchError, ResponseCompleted, ResponseStarted,
};
pub use crate::bidi::modules::script::events::{
    Message as ScriptMessage, RealmCreated, RealmDestroyed,
};
