//! Newtypes for opaque BiDi identifiers.
//!
//! The W3C BiDi spec uses many distinct string-shaped identifiers
//! ([`browsingContext.BrowsingContext`][bc], [`script.Realm`][realm],
//! [`network.Request`][req], etc.). On the wire they all look the same
//! — a JSON string — so it's easy to mix them up in Rust unless the
//! types track the difference.
//!
//! Each newtype here is `#[serde(transparent)]` (so the wire shape is
//! unchanged), implements `From<String>` / `From<&str>` /
//! [`Display`](std::fmt::Display), and exposes [`as_str`](BrowsingContextId::as_str)
//! for borrow access.
//!
//! [bc]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-BrowsingContext
//! [realm]: https://w3c.github.io/webdriver-bidi/#type-script-Realm
//! [req]: https://w3c.github.io/webdriver-bidi/#type-network-Request

use crate::common::protocol::string_id;

string_id! {
    /// Identifier for a browsing context. Mirrors the spec's
    /// [`browsingContext.BrowsingContext`][spec] type.
    ///
    /// On most drivers this is the same string used as a WebDriver
    /// Classic window handle — including for child frames, where it
    /// identifies the frame as a navigable rather than the parent window.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-BrowsingContext
    BrowsingContextId
}

string_id! {
    /// Identifier for a navigation initiated through BiDi. Mirrors the
    /// spec's [`browsingContext.Navigation`][spec] type.
    ///
    /// Returned by
    /// [`browsingContext.navigate`](crate::bidi::modules::browsing_context::Navigate)
    /// and surfaced on lifecycle events
    /// ([`Load`](crate::bidi::events::Load), …) so you can correlate
    /// "the navigation I started" against subsequent traffic.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-Navigation
    NavigationId
}

string_id! {
    /// Identifier for a JavaScript realm. Mirrors the spec's
    /// [`script.Realm`][spec] type.
    ///
    /// A realm is the BiDi term for an execution context — a window's
    /// main world, a worker, a sandbox. The id is stable for the
    /// realm's lifetime and is preserved across calls into
    /// [`script.evaluate`](crate::bidi::modules::script::Evaluate) /
    /// [`script.callFunction`](crate::bidi::modules::script::CallFunction).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-Realm
    RealmId
}

string_id! {
    /// Identifier for a preload script registered via
    /// [`script.addPreloadScript`](crate::bidi::modules::script::AddPreloadScript).
    /// Mirrors the spec's [`script.PreloadScript`][spec] type.
    ///
    /// Pass to
    /// [`script.removePreloadScript`](crate::bidi::modules::script::RemovePreloadScript)
    /// to uninstall.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-PreloadScript
    PreloadScriptId
}

string_id! {
    /// Identifier for a network request. Mirrors the spec's
    /// [`network.Request`][spec] type.
    ///
    /// Stable across redirects — a redirected request keeps the same
    /// id as the original.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-Request
    RequestId
}

string_id! {
    /// Identifier for a network interception registered via
    /// [`network.addIntercept`](crate::bidi::modules::network::AddIntercept).
    /// Mirrors the spec's [`network.Intercept`][spec] type.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-Intercept
    InterceptId
}

string_id! {
    /// Identifier for a [user context][spec] — BiDi's incognito-like
    /// storage isolation primitive. Mirrors the spec's
    /// [`browser.UserContext`][type-spec] type.
    ///
    /// The default user context is always called `"default"` and
    /// cannot be removed.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#user-contexts
    /// [type-spec]: https://w3c.github.io/webdriver-bidi/#type-browser-UserContext
    UserContextId
}

string_id! {
    /// Opaque identifier for a script-shared DOM node — the `sharedId`
    /// field of a [`script.SharedReference`][spec].
    ///
    /// Stable across navigations within the same browsing context.
    /// Pass to [`input.setFiles`](crate::bidi::modules::input::SetFiles)
    /// or include in
    /// [`script.callFunction`](crate::bidi::modules::script::CallFunction)
    /// arguments to address a specific DOM node.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-SharedReference
    NodeId
}

string_id! {
    /// Channel identifier for [`script.message`](crate::bidi::events::ScriptMessage)
    /// events. Mirrors the spec's [`script.Channel`][spec] type.
    ///
    /// Channels are created by passing a
    /// [`script.ChannelValue`][channel] argument to
    /// [`script.addPreloadScript`](crate::bidi::modules::script::AddPreloadScript);
    /// the script can then post messages back through them.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-Channel
    /// [channel]: https://w3c.github.io/webdriver-bidi/#type-script-ChannelValue
    ChannelId
}

string_id! {
    /// Identifier for an OS-level browser window. Mirrors the spec's
    /// [`browser.ClientWindow`][spec] type.
    ///
    /// One window can host many tabs (top-level browsing contexts).
    /// Pass to
    /// [`browser.setClientWindowState`](crate::bidi::modules::browser::SetClientWindowState)
    /// to resize / maximise / minimise it.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-browser-ClientWindow
    ClientWindowId
}

string_id! {
    /// Identifier for a network data collector. Mirrors the spec's
    /// [`network.Collector`][spec] type.
    ///
    /// Returned by
    /// [`network.addDataCollector`](crate::bidi::modules::network::AddDataCollector);
    /// pass to
    /// [`network.getData`](crate::bidi::modules::network::GetData),
    /// [`network.disownData`](crate::bidi::modules::network::DisownData),
    /// or
    /// [`network.removeDataCollector`](crate::bidi::modules::network::RemoveDataCollector).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-Collector
    CollectorId
}

string_id! {
    /// Identifier for an installed web extension. Mirrors the spec's
    /// [`webExtension.Extension`][spec] type.
    ///
    /// Returned by
    /// [`webExtension.install`](crate::bidi::modules::web_extension::Install);
    /// pass to
    /// [`webExtension.uninstall`](crate::bidi::modules::web_extension::Uninstall)
    /// to remove.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-webExtension-Extension
    ExtensionId
}
