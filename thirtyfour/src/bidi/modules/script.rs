//! `script.*` BiDi module — `evaluate`, `callFunction`, preload scripts, realms.
//!
//! Remote-object trees in BiDi (`RemoteValue`) are deeply recursive and
//! mostly used as opaque round-trip data: this module exposes them as
//! [`serde_json::Value`] so users don't pay an upfront serde cost, and can
//! deserialize themselves if/when they want to.

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, BidiEvent, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, ChannelId, PreloadScriptId, RealmId};
use crate::common::protocol::string_enum;

string_enum! {
    /// Mode controlling how `evaluate` / `callFunction` resolve return-value
    /// references.
    pub enum ResultOwnership {
        /// Return references owned by the calling realm; release explicitly
        /// via `script.disown`.
        Root = "root",
        /// Drop references as soon as the response is delivered.
        None = "none",
    }
}

/// `script.Target` — addressing a script realm.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Target {
    /// Address by realm id (most precise; survives navigations).
    Realm {
        /// Realm to evaluate in.
        realm: RealmId,
    },
    /// Address by browsing-context id; the driver picks the active realm.
    Context {
        /// Browsing context to evaluate in.
        context: BrowsingContextId,
        /// Optional sandbox name. Each `(context, sandbox)` is its own realm.
        #[serde(skip_serializing_if = "Option::is_none")]
        sandbox: Option<String>,
    },
}

/// `script.evaluate`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluate {
    /// Source code to evaluate. May contain `await` if `awaitPromise: true`.
    pub expression: String,
    /// Realm or context to evaluate in.
    pub target: Target,
    /// If true, await any returned promise before resolving.
    pub await_promise: bool,
    /// Whether the result reference should be retained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ownership: Option<ResultOwnership>,
    /// If true, treat the call as a user activation (allows
    /// `Document.requestStorageAccess()`, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_activation: Option<bool>,
}

impl BidiCommand for Evaluate {
    const METHOD: &'static str = "script.evaluate";
    type Returns = EvaluateResult;
}

/// Response for [`Evaluate`] / [`CallFunction`].
///
/// The driver returns one of two shapes — success (with `result`) or
/// exception (with `exceptionDetails`). Modeled here as a tagged enum on
/// `type`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EvaluateResult {
    /// Successful evaluation.
    Success {
        /// Realm the value lives in.
        realm: RealmId,
        /// Returned value (BiDi `RemoteValue`).
        result: serde_json::Value,
    },
    /// Threw an exception.
    Exception {
        /// Realm the throw happened in.
        realm: RealmId,
        /// Exception details (BiDi `ExceptionDetails`).
        #[serde(rename = "exceptionDetails")]
        exception_details: serde_json::Value,
    },
}

impl EvaluateResult {
    /// Borrow the `result` value if this is a success, else `None`.
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

/// `script.callFunction`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunction {
    /// Function-source string. Receives `this` and arguments per BiDi semantics.
    pub function_declaration: String,
    /// If true, await any returned promise before resolving.
    pub await_promise: bool,
    /// Realm or context to call in.
    pub target: Target,
    /// Arguments passed to the function (BiDi `LocalValue` JSON).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<serde_json::Value>,
    /// Optional `this` reference (BiDi `LocalValue` JSON).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub this: Option<serde_json::Value>,
    /// Whether the result reference should be retained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ownership: Option<ResultOwnership>,
    /// If true, treat the call as a user activation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_activation: Option<bool>,
}

impl BidiCommand for CallFunction {
    const METHOD: &'static str = "script.callFunction";
    type Returns = EvaluateResult;
}

/// `script.addPreloadScript`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPreloadScript {
    /// Function source — runs at document-start of every navigation.
    pub function_declaration: String,
    /// Optional list of `LocalValue` arguments to pass.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<serde_json::Value>,
    /// Optional sandbox name (creates an isolated realm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
    /// Restrict to specific contexts. Empty / omitted = global.
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

/// `script.removePreloadScript`.
#[derive(Debug, Clone, Serialize)]
pub struct RemovePreloadScript {
    /// Id returned by [`AddPreloadScript`].
    pub script: PreloadScriptId,
}

impl BidiCommand for RemovePreloadScript {
    const METHOD: &'static str = "script.removePreloadScript";
    type Returns = Empty;
}

/// `script.getRealms`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRealms {
    /// Restrict to a single browsing context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<BrowsingContextId>,
    /// Restrict to a realm type. See spec for valid strings (`window`,
    /// `dedicated-worker`, `shared-worker`, `service-worker`, `worker`,
    /// `paint-worklet`, `audio-worklet`, `worklet`).
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
    /// Discovered realms.
    pub realms: Vec<RealmInfo>,
}

/// Info for a single realm. Modeled as the spec's discriminated union over
/// `type`; the rest of the fields vary by type, so non-essential fields are
/// captured into [`RealmInfo::extra`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmInfo {
    /// Realm id.
    pub realm: RealmId,
    /// Origin string.
    pub origin: String,
    /// Realm type (`"window"`, `"worker"`, …).
    #[serde(rename = "type")]
    pub realm_type: String,
    /// Browsing context for window-type realms.
    #[serde(default)]
    pub context: Option<BrowsingContextId>,
    /// Other fields (e.g. `sandbox`, `owners`) flattened into a JSON map.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// `script.disown` — drop references to remote objects.
#[derive(Debug, Clone, Serialize)]
pub struct Disown {
    /// Object handles to disown.
    pub handles: Vec<String>,
    /// Realm or context to disown in.
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

    /// `script.realmCreated`.
    #[derive(Debug, Clone, Deserialize)]
    pub struct RealmCreated(pub RealmInfo);

    impl BidiEvent for RealmCreated {
        const METHOD: &'static str = "script.realmCreated";
    }

    /// `script.realmDestroyed`.
    #[derive(Debug, Clone, Deserialize)]
    pub struct RealmDestroyed {
        /// Realm that was destroyed.
        pub realm: RealmId,
    }

    impl BidiEvent for RealmDestroyed {
        const METHOD: &'static str = "script.realmDestroyed";
    }

    /// `script.message`. Posted from preload-script-installed channels via
    /// the function `channel(...)` argument.
    #[derive(Debug, Clone, Deserialize)]
    pub struct Message {
        /// Channel id the message was sent on.
        pub channel: ChannelId,
        /// Payload (`RemoteValue`).
        pub data: serde_json::Value,
        /// Source realm.
        pub source: serde_json::Value,
    }

    impl BidiEvent for Message {
        const METHOD: &'static str = "script.message";
    }
}

/// Module facade returned by [`BiDi::script`].
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

    /// `script.evaluate` — convenience: synchronous expression in the active
    /// realm of `context`.
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

    /// `script.callFunction` — convenience: call a function declaration in
    /// `context` with no arguments.
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

    /// `script.addPreloadScript`.
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

    /// `script.removePreloadScript`.
    pub async fn remove_preload_script(&self, script: PreloadScriptId) -> Result<(), BidiError> {
        self.bidi
            .send(RemovePreloadScript {
                script,
            })
            .await?;
        Ok(())
    }

    /// `script.getRealms` — all realms.
    pub async fn get_realms(&self) -> Result<GetRealmsResult, BidiError> {
        self.bidi
            .send(GetRealms {
                context: None,
                r#type: None,
            })
            .await
    }
}
