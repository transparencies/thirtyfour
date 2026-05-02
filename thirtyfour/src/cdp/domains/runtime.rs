//! `Runtime` domain — JavaScript evaluation, remote object handles,
//! `console.*` capture, and uncaught-exception observation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::cdp::ids::{ExecutionContextId, RemoteObjectId, ScriptId};
use crate::common::protocol::string_enum;
use crate::error::WebDriverResult;

/// `Runtime.enable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Enable;
impl CdpCommand for Enable {
    const METHOD: &'static str = "Runtime.enable";
    type Returns = Empty;
}

/// `Runtime.disable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Disable;
impl CdpCommand for Disable {
    const METHOD: &'static str = "Runtime.disable";
    type Returns = Empty;
}

/// A handle to a JavaScript object held alive by V8.
///
/// See <https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#type-RemoteObject>.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    /// Object type (`"object"`, `"function"`, `"undefined"`, `"string"`, `"number"`, `"boolean"`, `"symbol"`, `"bigint"`).
    pub r#type: String,
    /// Object subtype hint, only set for objects (e.g. `"array"`, `"node"`, `"date"`).
    pub subtype: Option<String>,
    /// Object class name (e.g. `"HTMLElement"`).
    pub class_name: Option<String>,
    /// Remote object value, when the value is serialisable.
    pub value: Option<Value>,
    /// Description (textual representation).
    pub description: Option<String>,
    /// Unique identifier (object handle). Use [`Runtime.releaseObject`](ReleaseObject)
    /// when no longer needed.
    pub object_id: Option<RemoteObjectId>,
}

/// `Runtime.evaluate`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluate {
    /// JavaScript expression to evaluate.
    pub expression: String,
    /// Object group name to release together with `Runtime.releaseObjectGroup`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Determines whether `Command Line API` should be available during evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// In silent mode, exceptions thrown during evaluation are not reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    /// Specifies in which execution context to perform evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<ExecutionContextId>,
    /// Returns by-value result (rather than a `RemoteObject` handle).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether the result is expected to be a Promise that should be awaited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
    /// Whether the evaluation is in user gesture context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_gesture: Option<bool>,
}

impl Evaluate {
    /// Build a basic `Runtime.evaluate` for the given expression.
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            object_group: None,
            include_command_line_api: None,
            silent: None,
            context_id: None,
            return_by_value: None,
            await_promise: None,
            user_gesture: None,
        }
    }
}

/// Exception details for a failed evaluation/call.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    /// Exception id.
    pub exception_id: i64,
    /// Exception text.
    pub text: String,
    /// Line number of the exception location (0-based).
    pub line_number: u32,
    /// Column number of the exception location (0-based).
    pub column_number: u32,
    /// Script id of the exception location.
    pub script_id: Option<ScriptId>,
    /// URL of the exception location.
    pub url: Option<String>,
    /// Stack trace if available.
    pub stack_trace: Option<Value>,
    /// The thrown exception object.
    pub exception: Option<RemoteObject>,
    /// Execution context id.
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Common return shape for `Runtime.evaluate` and `Runtime.callFunctionOn`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationResult {
    /// Evaluation result.
    pub result: RemoteObject,
    /// Exception details, if the evaluation threw.
    pub exception_details: Option<ExceptionDetails>,
}

impl CdpCommand for Evaluate {
    const METHOD: &'static str = "Runtime.evaluate";
    type Returns = EvaluationResult;
}

/// `Runtime.callFunctionOn`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionOn {
    /// Declaration of the function to call (e.g. `"function() { return this.tagName; }"`).
    pub function_declaration: String,
    /// Identifier of the object to call function on. Mutually exclusive with `executionContextId`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Function arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<Value>>,
    /// Execution context to call the function in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<ExecutionContextId>,
    /// Whether the result is expected to be a Promise that should be awaited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub await_promise: Option<bool>,
    /// Returns by-value result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_by_value: Option<bool>,
    /// Whether the call is in user gesture context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_gesture: Option<bool>,
}
impl CdpCommand for CallFunctionOn {
    const METHOD: &'static str = "Runtime.callFunctionOn";
    type Returns = EvaluationResult;
}

/// `Runtime.releaseObject` — frees an object handle.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseObject {
    /// Identifier of the object to release.
    pub object_id: RemoteObjectId,
}
impl CdpCommand for ReleaseObject {
    const METHOD: &'static str = "Runtime.releaseObject";
    type Returns = Empty;
}

/// `Runtime.releaseObjectGroup`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseObjectGroup {
    /// Object group name.
    pub object_group: String,
}
impl CdpCommand for ReleaseObjectGroup {
    const METHOD: &'static str = "Runtime.releaseObjectGroup";
    type Returns = Empty;
}

string_enum! {
    /// Type of `console.*` call reported by [`ConsoleApiCalled`].
    ///
    /// Mirrors CDP's `Runtime.ConsoleAPICalledType`. Drivers occasionally
    /// add new values; the [`Unknown`][Self::Unknown] variant is a
    /// forward-compatibility escape hatch generated by the internal
    /// `string_enum!` macro.
    pub enum ConsoleApiType {
        /// `console.log`.
        Log = "log",
        /// `console.debug`.
        Debug = "debug",
        /// `console.info`.
        Info = "info",
        /// `console.error`.
        Error = "error",
        /// `console.warn` (CDP reports it as `"warning"`).
        Warning = "warning",
        /// `console.dir`.
        Dir = "dir",
        /// `console.dirxml`.
        Dirxml = "dirxml",
        /// `console.table`.
        Table = "table",
        /// `console.trace`.
        Trace = "trace",
        /// `console.clear`.
        Clear = "clear",
        /// `console.group`.
        StartGroup = "startGroup",
        /// `console.groupCollapsed`.
        StartGroupCollapsed = "startGroupCollapsed",
        /// `console.groupEnd`.
        EndGroup = "endGroup",
        /// `console.assert`.
        Assert = "assert",
        /// `console.profile`.
        Profile = "profile",
        /// `console.profileEnd`.
        ProfileEnd = "profileEnd",
        /// `console.count`.
        Count = "count",
        /// `console.timeEnd`.
        TimeEnd = "timeEnd",
    }
}

/// `Runtime.consoleAPICalled` event — fired when the page invokes
/// `console.log` / `console.warn` / etc.
///
/// `Runtime.enable` (or [`RuntimeDomain::enable`]) must be called on the
/// session before the event fires; otherwise the subscription will see
/// nothing.
///
/// Args are CDP `RemoteObject`s. Pass `returnByValue` semantics manually
/// by calling [`RuntimeDomain::evaluate_value`] for similar inline use, or
/// inspect the `value` / `description` fields directly.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleApiCalled {
    /// Which `console.*` method the page invoked.
    pub r#type: ConsoleApiType,
    /// Call arguments — one [`RemoteObject`] per argument the page passed.
    pub args: Vec<RemoteObject>,
    /// Identifier of the execution context the call came from.
    pub execution_context_id: ExecutionContextId,
    /// Wall-clock timestamp (milliseconds since the Unix epoch).
    pub timestamp: f64,
    /// Stack trace captured at the call site, when available. Left as
    /// untyped JSON because the full `StackTrace` shape is rarely needed.
    #[serde(default)]
    pub stack_trace: Option<Value>,
    /// Deprecated context label (some drivers still emit this).
    #[serde(default)]
    pub context: Option<String>,
}
impl CdpEvent for ConsoleApiCalled {
    const METHOD: &'static str = "Runtime.consoleAPICalled";
}

/// `Runtime.exceptionThrown` event — fired when an uncaught JavaScript
/// exception bubbles out of a page-side script.
///
/// As with [`ConsoleApiCalled`], `Runtime.enable` must be called first.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionThrown {
    /// Wall-clock timestamp (milliseconds since the Unix epoch).
    pub timestamp: f64,
    /// Details of the thrown exception.
    pub exception_details: ExceptionDetails,
}
impl CdpEvent for ExceptionThrown {
    const METHOD: &'static str = "Runtime.exceptionThrown";
}

/// Domain facade returned by [`Cdp::runtime`].
#[derive(Debug)]
pub struct RuntimeDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> RuntimeDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Runtime.enable`.
    pub async fn enable(&self) -> WebDriverResult<()> {
        self.cdp.send(Enable).await?;
        Ok(())
    }

    /// `Runtime.disable`.
    pub async fn disable(&self) -> WebDriverResult<()> {
        self.cdp.send(Disable).await?;
        Ok(())
    }

    /// `Runtime.evaluate` with a default config (returns a `RemoteObject` handle).
    pub async fn evaluate(&self, expression: impl Into<String>) -> WebDriverResult<RemoteObject> {
        let r = self.cdp.send(Evaluate::new(expression)).await?;
        Ok(r.result)
    }

    /// `Runtime.evaluate` with `returnByValue: true` — returns the value as JSON.
    pub async fn evaluate_value(&self, expression: impl Into<String>) -> WebDriverResult<Value> {
        let mut params = Evaluate::new(expression);
        params.return_by_value = Some(true);
        let r = self.cdp.send(params).await?;
        Ok(r.result.value.unwrap_or(Value::Null))
    }

    /// `Runtime.callFunctionOn` with `returnByValue: true`.
    pub async fn call_function_on(
        &self,
        object_id: RemoteObjectId,
        function_declaration: impl Into<String>,
    ) -> WebDriverResult<Value> {
        let r = self
            .cdp
            .send(CallFunctionOn {
                function_declaration: function_declaration.into(),
                object_id: Some(object_id),
                arguments: None,
                execution_context_id: None,
                await_promise: Some(true),
                return_by_value: Some(true),
                user_gesture: None,
            })
            .await?;
        Ok(r.result.value.unwrap_or(Value::Null))
    }

    /// `Runtime.releaseObject`.
    pub async fn release_object(&self, object_id: RemoteObjectId) -> WebDriverResult<()> {
        self.cdp
            .send(ReleaseObject {
                object_id,
            })
            .await?;
        Ok(())
    }
}
