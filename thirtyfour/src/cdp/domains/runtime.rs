//! `Runtime` domain — JavaScript evaluation, remote object handles.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::cdp::ids::{ExecutionContextId, RemoteObjectId, ScriptId};
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
    #[serde(default)]
    pub subtype: Option<String>,
    /// Object class name (e.g. `"HTMLElement"`).
    #[serde(default)]
    pub class_name: Option<String>,
    /// Remote object value, when the value is serialisable.
    #[serde(default)]
    pub value: Option<Value>,
    /// Description (textual representation).
    #[serde(default)]
    pub description: Option<String>,
    /// Unique identifier (object handle). Use [`Runtime.releaseObject`](ReleaseObject)
    /// when no longer needed.
    #[serde(default)]
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
    #[serde(default)]
    pub script_id: Option<ScriptId>,
    /// URL of the exception location.
    #[serde(default)]
    pub url: Option<String>,
    /// Stack trace if available.
    #[serde(default)]
    pub stack_trace: Option<Value>,
    /// The thrown exception object.
    #[serde(default)]
    pub exception: Option<RemoteObject>,
    /// Execution context id.
    #[serde(default)]
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Common return shape for `Runtime.evaluate` and `Runtime.callFunctionOn`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationResult {
    /// Evaluation result.
    pub result: RemoteObject,
    /// Exception details, if the evaluation threw.
    #[serde(default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn methods() {
        assert_eq!(Enable::METHOD, "Runtime.enable");
        assert_eq!(Disable::METHOD, "Runtime.disable");
        assert_eq!(Evaluate::METHOD, "Runtime.evaluate");
        assert_eq!(CallFunctionOn::METHOD, "Runtime.callFunctionOn");
        assert_eq!(ReleaseObject::METHOD, "Runtime.releaseObject");
        assert_eq!(ReleaseObjectGroup::METHOD, "Runtime.releaseObjectGroup");
    }

    #[test]
    fn evaluate_minimal_skips_optional() {
        let v = serde_json::to_value(Evaluate::new("1 + 1")).unwrap();
        assert_eq!(v["expression"], "1 + 1");
        for opt in [
            "objectGroup",
            "includeCommandLineApi",
            "silent",
            "contextId",
            "returnByValue",
            "awaitPromise",
            "userGesture",
        ] {
            assert!(v.get(opt).is_none(), "{opt} should be omitted");
        }
    }

    #[test]
    fn evaluate_full_camel_case_field_names() {
        let v = serde_json::to_value(Evaluate {
            expression: "1".to_string(),
            object_group: Some("g".to_string()),
            include_command_line_api: Some(true),
            silent: Some(false),
            context_id: Some(ExecutionContextId::new(7)),
            return_by_value: Some(true),
            await_promise: Some(false),
            user_gesture: Some(true),
        })
        .unwrap();
        assert_eq!(v["objectGroup"], "g");
        assert_eq!(v["includeCommandLineApi"], true);
        assert_eq!(v["silent"], false);
        assert_eq!(v["contextId"], 7);
        assert_eq!(v["returnByValue"], true);
        assert_eq!(v["awaitPromise"], false);
        assert_eq!(v["userGesture"], true);
    }

    #[test]
    fn remote_object_minimal_string() {
        let body = json!({"type": "string", "value": "hello"});
        let r: RemoteObject = serde_json::from_value(body).unwrap();
        assert_eq!(r.r#type, "string");
        assert_eq!(r.value.unwrap(), json!("hello"));
        assert!(r.subtype.is_none());
        assert!(r.object_id.is_none());
    }

    #[test]
    fn remote_object_with_subtype_and_handle() {
        let body = json!({
            "type": "object",
            "subtype": "node",
            "className": "HTMLButtonElement",
            "objectId": "{\"injectedScriptId\":1,\"id\":2}",
            "description": "button#b"
        });
        let r: RemoteObject = serde_json::from_value(body).unwrap();
        assert_eq!(r.subtype.as_deref(), Some("node"));
        assert_eq!(r.class_name.as_deref(), Some("HTMLButtonElement"));
        assert_eq!(r.description.as_deref(), Some("button#b"));
        assert!(r.object_id.is_some());
    }

    #[test]
    fn evaluation_result_with_exception() {
        let body = json!({
            "result": {"type": "object", "subtype": "error"},
            "exceptionDetails": {
                "exceptionId": 1,
                "text": "Uncaught",
                "lineNumber": 0,
                "columnNumber": 0
            }
        });
        let r: EvaluationResult = serde_json::from_value(body).unwrap();
        assert!(r.exception_details.is_some());
        let ex = r.exception_details.unwrap();
        assert_eq!(ex.text, "Uncaught");
    }

    #[test]
    fn call_function_on_camel_case() {
        let v = serde_json::to_value(CallFunctionOn {
            function_declaration: "function(){return this.tagName;}".to_string(),
            object_id: Some(RemoteObjectId::from("OBJ")),
            arguments: Some(vec![json!({"value": 1})]),
            execution_context_id: None,
            await_promise: Some(true),
            return_by_value: Some(true),
            user_gesture: None,
        })
        .unwrap();
        assert_eq!(v["functionDeclaration"], "function(){return this.tagName;}");
        assert_eq!(v["objectId"], "OBJ");
        assert_eq!(v["awaitPromise"], true);
        assert_eq!(v["returnByValue"], true);
        assert!(v.get("executionContextId").is_none());
        assert!(v.get("userGesture").is_none());
        assert!(v["arguments"].is_array());
    }

    #[test]
    fn release_object_serialises_object_id() {
        let v = serde_json::to_value(ReleaseObject {
            object_id: RemoteObjectId::from("X"),
        })
        .unwrap();
        assert_eq!(v["objectId"], "X");
    }

    #[test]
    fn release_object_group_serialises_group() {
        let v = serde_json::to_value(ReleaseObjectGroup {
            object_group: "g".to_string(),
        })
        .unwrap();
        assert_eq!(v["objectGroup"], "g");
    }
}
