//! Typed BiDi events.
//!
//! Each event implements [`crate::bidi::BidiEvent`] and is the typed
//! payload for [`crate::bidi::BiDi::subscribe`]. Names are renamed
//! where the bare W3C ones would collide.

pub use crate::bidi::modules::browsing_context::events::{
    ContextCreated, ContextDestroyed, DomContentLoaded, FragmentNavigated, Load, NavigationStarted,
    UserPromptClosed, UserPromptOpened,
};
pub use crate::bidi::modules::log::events::EntryAdded as LogEntryAdded;
pub use crate::bidi::modules::network::events::{
    AuthRequired, BeforeRequestSent, FetchError, ResponseCompleted, ResponseStarted,
};
pub use crate::bidi::modules::script::events::{
    Message as ScriptMessage, RealmCreated, RealmDestroyed,
};
