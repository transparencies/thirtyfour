//! Newtypes for opaque BiDi identifiers.
//!
//! The spec uses a handful of distinct string-shaped ids that are easy to
//! mix up. These newtypes preserve the wire format (`#[serde(transparent)]`)
//! while keeping them distinct in Rust.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Construct from any string-like value.
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            /// Borrow the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id! {
    /// Identifier for a browsing context (`browsingContext.BrowsingContext`).
    /// On most drivers this is the same string used as a WebDriver classic
    /// window handle.
    BrowsingContextId
}

string_id! {
    /// Identifier for a navigation initiated through BiDi
    /// (`browsingContext.Navigation`).
    NavigationId
}

string_id! {
    /// Identifier for a script realm (`script.Realm`). Stable while the
    /// realm exists; reused across `script.evaluate` / `script.callFunction`.
    RealmId
}

string_id! {
    /// Identifier for a preload script registered via
    /// `script.addPreloadScript`.
    PreloadScriptId
}

string_id! {
    /// Identifier for a network request (`network.Request`).
    RequestId
}

string_id! {
    /// Identifier for a network request interception
    /// (`network.Intercept`).
    InterceptId
}

string_id! {
    /// Identifier for a user context — BiDi's "incognito-like" isolation
    /// container (`browser.UserContext`).
    UserContextId
}

string_id! {
    /// Opaque identifier for a script-shared DOM node
    /// (`script.SharedReference.sharedId`). Stable across navigations
    /// within the same browsing context.
    NodeId
}

string_id! {
    /// Channel identifier for `script.message` events.
    ChannelId
}
