//! `script.*` — execute JavaScript, manage preload scripts, observe realms.
//!
//! BiDi's script module is the bidirectional successor to WebDriver
//! Classic's `Execute Script` endpoint. It can:
//!
//! - Run an expression ([`Evaluate`][ev]) or call a function with
//!   arguments ([`CallFunction`][cf]) in any [`script.Realm`][realm-spec].
//! - Install preload scripts ([`AddPreloadScript`][aps]) that run at
//!   document-start of every navigation, including in iframes.
//! - Enumerate realms ([`GetRealms`][gr]) and observe their lifecycle
//!   ([`RealmCreated`][rc], [`RealmDestroyed`][rd]).
//! - Receive messages from preload-script channels ([`Message`][msg]).
//! - Release retained references to remote objects ([`Disown`][dis]).
//!
//! The spec models JavaScript values as a deeply-recursive
//! [`script.RemoteValue`][remotevalue-spec] tree. To keep the Rust API
//! ergonomic this module exposes those values as [`serde_json::Value`]
//! — round-trip them as-is, or destructure with serde when you need
//! them strongly typed. Arguments to [`CallFunction`][cf] follow the
//! sibling [`script.LocalValue`][localvalue-spec] shape.
//!
//! [ev]: crate::bidi::modules::script::Evaluate
//! [cf]: crate::bidi::modules::script::CallFunction
//! [aps]: crate::bidi::modules::script::AddPreloadScript
//! [gr]: crate::bidi::modules::script::GetRealms
//! [rc]: crate::bidi::modules::script::events::RealmCreated
//! [rd]: crate::bidi::modules::script::events::RealmDestroyed
//! [msg]: crate::bidi::modules::script::events::Message
//! [dis]: crate::bidi::modules::script::Disown
//!
//! See the [W3C `script` module specification][spec] for the canonical
//! definitions.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-script
//! [realm-spec]: https://w3c.github.io/webdriver-bidi/#type-script-Realm
//! [remotevalue-spec]: https://w3c.github.io/webdriver-bidi/#type-script-RemoteValue
//! [localvalue-spec]: https://w3c.github.io/webdriver-bidi/#type-script-LocalValue

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, BidiEvent, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, ChannelId, PreloadScriptId, RealmId};
use crate::common::protocol::string_enum;

string_enum! {
    /// Result-ownership mode for [`Evaluate`] / [`CallFunction`]. Mirrors
    /// the spec's [`script.ResultOwnership`][spec] type.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-ResultOwnership
    pub enum ResultOwnership {
        /// Retain the returned object reference; release it later via
        /// [`Disown`].
        Root = "root",
        /// Drop the reference as soon as the response is delivered
        /// (default).
        None = "none",
    }
}

/// Target realm or context for [`Evaluate`] / [`CallFunction`] /
/// [`Disown`]. Mirrors the spec's [`script.Target`][spec] union.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-Target
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Target {
    /// Address by realm id (most precise; survives same-origin navigations).
    Realm {
        /// Realm to evaluate in.
        realm: RealmId,
    },
    /// Address by browsing-context id; the driver picks the active
    /// realm. Optional `sandbox` selects a named isolated realm
    /// instead.
    Context {
        /// Browsing context to evaluate in.
        context: BrowsingContextId,
        /// Optional sandbox name. Each `(context, sandbox)` pair lives
        /// in its own realm, isolated from the page's main world.
        #[serde(skip_serializing_if = "Option::is_none")]
        sandbox: Option<String>,
    },
}

/// [`script.evaluate`][spec] — run a JavaScript expression in a realm.
///
/// `expression` is parsed as a JavaScript expression (NOT a script). To
/// `await` a top-level promise, set [`await_promise`][Self::await_promise]
/// — the wire-level `expression` then runs inside the BiDi runtime's
/// implicit `async`.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-evaluate
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluate {
    /// Expression source.
    pub expression: String,
    /// Realm or context to evaluate in.
    pub target: Target,
    /// If `true`, await any returned promise before resolving.
    pub await_promise: bool,
    /// Whether to retain the result reference. Defaults to
    /// [`ResultOwnership::None`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ownership: Option<ResultOwnership>,
    /// If `true`, treat the call as a user activation — gates APIs
    /// like `Document.requestStorageAccess()` and pop-ups behind a
    /// real user gesture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_activation: Option<bool>,
}

impl BidiCommand for Evaluate {
    const METHOD: &'static str = "script.evaluate";
    type Returns = EvaluateResult;
}

/// Result of [`Evaluate`] / [`CallFunction`]. Mirrors the spec's
/// [`script.EvaluateResult`][spec] union — either a successful return
/// value or an exception.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-EvaluateResult
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EvaluateResult {
    /// Successful evaluation.
    Success {
        /// Realm the value lives in.
        realm: RealmId,
        /// Returned value as a [`script.RemoteValue`][spec] JSON tree.
        ///
        /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-RemoteValue
        result: serde_json::Value,
    },
    /// Threw an exception.
    Exception {
        /// Realm the throw happened in.
        realm: RealmId,
        /// Exception details — a [`script.ExceptionDetails`][spec]
        /// JSON value.
        ///
        /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-ExceptionDetails
        #[serde(rename = "exceptionDetails")]
        exception_details: serde_json::Value,
    },
}

impl EvaluateResult {
    /// Borrow the `result` value if this is a [`Success`][Self::Success],
    /// else `None`.
    pub fn ok_value(&self) -> Option<&serde_json::Value> {
        match self {
            EvaluateResult::Success {
                result,
                ..
            } => Some(result),
            _ => None,
        }
    }

    /// True if the call returned an exception.
    pub fn is_exception(&self) -> bool {
        matches!(self, EvaluateResult::Exception { .. })
    }
}

/// [`script.callFunction`][spec] — call a JavaScript function with
/// arguments and a `this` binding.
///
/// `function_declaration` is the source of a function expression:
///
/// ```js
/// function (a, b) { return a + b; }
/// // or
/// (a, b) => a + b
/// ```
///
/// Arguments are passed as [`script.LocalValue`][local] JSON values
/// (primitive shape: `{"type":"number","value":1}`,
/// `{"type":"string","value":"hi"}`, etc.). Use
/// [`this`][Self::this] to set the `this` binding.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-callFunction
/// [local]: https://w3c.github.io/webdriver-bidi/#type-script-LocalValue
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunction {
    /// Function-source string (a function expression or arrow function).
    pub function_declaration: String,
    /// If `true`, await any returned promise before resolving.
    pub await_promise: bool,
    /// Realm or context to call in.
    pub target: Target,
    /// Arguments — array of [`script.LocalValue`][local] JSON values.
    ///
    /// [local]: https://w3c.github.io/webdriver-bidi/#type-script-LocalValue
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<serde_json::Value>,
    /// Optional `this` reference (a [`script.LocalValue`][local]).
    ///
    /// [local]: https://w3c.github.io/webdriver-bidi/#type-script-LocalValue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub this: Option<serde_json::Value>,
    /// Whether to retain the result reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ownership: Option<ResultOwnership>,
    /// If `true`, treat the call as a user activation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_activation: Option<bool>,
}

impl BidiCommand for CallFunction {
    const METHOD: &'static str = "script.callFunction";
    type Returns = EvaluateResult;
}

/// [`script.addPreloadScript`][spec] — install a script that runs at
/// document-start of every navigation in matching contexts.
///
/// Preload scripts run in their own (or a named-`sandbox`) isolated
/// realm. They can take arguments — most commonly a
/// [`script.ChannelValue`][channel] which the page can post messages
/// through, surfacing as [`events::Message`].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-addPreloadScript
/// [channel]: https://w3c.github.io/webdriver-bidi/#type-script-ChannelValue
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPreloadScript {
    /// Function-source string (`(channel) => { … }`).
    pub function_declaration: String,
    /// Arguments — usually one or more channel values.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<serde_json::Value>,
    /// Optional sandbox name (creates an isolated realm shared with
    /// `script.callFunction(target: Context { sandbox: Some(…) })`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
    /// Restrict to specific top-level browsing contexts. Empty = every
    /// context.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
}

impl BidiCommand for AddPreloadScript {
    const METHOD: &'static str = "script.addPreloadScript";
    type Returns = AddPreloadScriptResult;
}

/// Response for [`AddPreloadScript`].
#[derive(Debug, Clone, Deserialize)]
pub struct AddPreloadScriptResult {
    /// Server-assigned id used by [`RemovePreloadScript`].
    pub script: PreloadScriptId,
}

/// [`script.removePreloadScript`][spec] — uninstall a preload script.
///
/// Already-running scripts in active realms are not affected; only
/// future navigations stop applying it.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-removePreloadScript
#[derive(Debug, Clone, Serialize)]
pub struct RemovePreloadScript {
    /// Id returned by [`AddPreloadScript`].
    pub script: PreloadScriptId,
}

impl BidiCommand for RemovePreloadScript {
    const METHOD: &'static str = "script.removePreloadScript";
    type Returns = Empty;
}

/// [`script.getRealms`][spec] — list active realms, optionally filtered.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-getRealms
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRealms {
    /// Restrict to a single browsing context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<BrowsingContextId>,
    /// Restrict to a [realm type][spec]: `"window"`, `"dedicated-worker"`,
    /// `"shared-worker"`, `"service-worker"`, `"worker"`,
    /// `"paint-worklet"`, `"audio-worklet"`, or `"worklet"`.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-RealmType
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

impl BidiCommand for GetRealms {
    const METHOD: &'static str = "script.getRealms";
    type Returns = GetRealmsResult;
}

/// Response for [`GetRealms`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetRealmsResult {
    /// All realms matching the filter (or every realm if unfiltered).
    pub realms: Vec<RealmInfo>,
}

/// Information about a single realm. Mirrors the spec's
/// [`script.RealmInfo`][spec] discriminated union.
///
/// Type-specific fields (e.g. `sandbox` for window realms, `owners` for
/// worker realms) are flattened into [`extra`][Self::extra].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-RealmInfo
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmInfo {
    /// Realm id.
    pub realm: RealmId,
    /// Origin string (`"https://example.com"`, `"null"`, …).
    pub origin: String,
    /// Realm type (`"window"`, `"dedicated-worker"`, …).
    #[serde(rename = "type")]
    pub realm_type: String,
    /// Browsing context for window-type realms.
    #[serde(default)]
    pub context: Option<BrowsingContextId>,
    /// Other type-specific fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// [`script.disown`][spec] — release retained object references.
///
/// Only meaningful for evaluations that used
/// [`ResultOwnership::Root`] — without that, references are released
/// automatically.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-disown
#[derive(Debug, Clone, Serialize)]
pub struct Disown {
    /// Object handles to disown (the `handle` field on each
    /// [`script.RemoteValue`][spec]).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-RemoteValue
    pub handles: Vec<String>,
    /// Realm or context where the handles live.
    pub target: Target,
}

impl BidiCommand for Disown {
    const METHOD: &'static str = "script.disown";
    type Returns = Empty;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events surfaced by the `script.*` module.
pub(crate) mod events {
    use super::*;

    /// [`script.realmCreated`][spec] — fires when a new realm becomes
    /// available (page navigation, worker startup, …).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-script-realmCreated
    #[derive(Debug, Clone, Deserialize)]
    pub struct RealmCreated(pub RealmInfo);

    impl BidiEvent for RealmCreated {
        const METHOD: &'static str = "script.realmCreated";
    }

    /// [`script.realmDestroyed`][spec] — fires when a realm is torn down.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-script-realmDestroyed
    #[derive(Debug, Clone, Deserialize)]
    pub struct RealmDestroyed {
        /// Realm that was destroyed.
        pub realm: RealmId,
    }

    impl BidiEvent for RealmDestroyed {
        const METHOD: &'static str = "script.realmDestroyed";
    }

    /// [`script.message`][spec] — a preload-script channel posted a
    /// message.
    ///
    /// Channels are created by passing a
    /// [`script.ChannelValue`][channel] argument to
    /// [`AddPreloadScript`]; the script can then post arbitrary
    /// messages back through it.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-script-message
    /// [channel]: https://w3c.github.io/webdriver-bidi/#type-script-ChannelValue
    #[derive(Debug, Clone, Deserialize)]
    pub struct Message {
        /// Channel id the message was sent on.
        pub channel: ChannelId,
        /// Payload — a [`script.RemoteValue`][rv] JSON value.
        ///
        /// [rv]: https://w3c.github.io/webdriver-bidi/#type-script-RemoteValue
        pub data: serde_json::Value,
        /// Source realm — a [`script.Source`][src] JSON value.
        ///
        /// [src]: https://w3c.github.io/webdriver-bidi/#type-script-Source
        pub source: serde_json::Value,
    }

    impl BidiEvent for Message {
        const METHOD: &'static str = "script.message";
    }
}

/// Convenience facade for the `script.*` module.
///
/// Returned by [`BiDi::script`](crate::bidi::BiDi::script). The methods
/// here cover the common forms of each command (no-argument call, no
/// sandbox, default ownership). For sandboxed evaluation, custom
/// arguments, retained references, or user-activation gating, build the
/// command struct directly.
#[derive(Debug)]
pub struct ScriptModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> ScriptModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// Run a JavaScript expression in `context`'s default realm via
    /// [`script.evaluate`][spec].
    ///
    /// Set `await_promise` if the expression resolves to a promise you
    /// want awaited.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-evaluate
    pub async fn evaluate(
        &self,
        context: BrowsingContextId,
        expression: impl Into<String>,
        await_promise: bool,
    ) -> Result<EvaluateResult, BidiError> {
        self.bidi
            .send(Evaluate {
                expression: expression.into(),
                target: Target::Context {
                    context,
                    sandbox: None,
                },
                await_promise,
                result_ownership: None,
                user_activation: None,
            })
            .await
    }

    /// Call a function in `context`'s default realm with no arguments
    /// via [`script.callFunction`][spec].
    ///
    /// `function_declaration` is a function expression source string —
    /// see [`CallFunction`].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-callFunction
    pub async fn call_function(
        &self,
        context: BrowsingContextId,
        function_declaration: impl Into<String>,
        await_promise: bool,
    ) -> Result<EvaluateResult, BidiError> {
        self.bidi
            .send(CallFunction {
                function_declaration: function_declaration.into(),
                await_promise,
                target: Target::Context {
                    context,
                    sandbox: None,
                },
                arguments: vec![],
                this: None,
                result_ownership: None,
                user_activation: None,
            })
            .await
    }

    /// Install a global preload script via
    /// [`script.addPreloadScript`][spec].
    ///
    /// For sandboxed scripts, channel arguments, or per-context scope
    /// build an [`AddPreloadScript`] directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-addPreloadScript
    pub async fn add_preload_script(
        &self,
        function_declaration: impl Into<String>,
    ) -> Result<AddPreloadScriptResult, BidiError> {
        self.bidi
            .send(AddPreloadScript {
                function_declaration: function_declaration.into(),
                arguments: vec![],
                sandbox: None,
                contexts: vec![],
            })
            .await
    }

    /// Uninstall a preload script via
    /// [`script.removePreloadScript`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-removePreloadScript
    pub async fn remove_preload_script(&self, script: PreloadScriptId) -> Result<(), BidiError> {
        self.bidi
            .send(RemovePreloadScript {
                script,
            })
            .await?;
        Ok(())
    }

    /// List every realm via [`script.getRealms`][spec].
    ///
    /// To filter by context or realm type, build a [`GetRealms`] struct
    /// directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-script-getRealms
    pub async fn get_realms(&self) -> Result<GetRealmsResult, BidiError> {
        self.bidi
            .send(GetRealms {
                context: None,
                r#type: None,
            })
            .await
    }
}
