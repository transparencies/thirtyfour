//! Newtypes for opaque BiDi identifiers.
//!
//! The spec uses a handful of distinct string-shaped ids that are easy to
//! mix up. These newtypes preserve the wire format (`#[serde(transparent)]`)
//! while keeping them distinct in Rust.
//!
//! The `string_id!` macro lives in [`crate::common::protocol`] and is
//! shared with the CDP layer.

use crate::common::protocol::string_id;

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
