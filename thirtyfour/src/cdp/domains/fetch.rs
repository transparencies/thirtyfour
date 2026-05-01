//! `Fetch` domain — request/response interception.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::cdp::domains::network::{ErrorReason, ResourceType};
use crate::cdp::ids::FetchRequestId;
use crate::cdp::macros::string_enum;
use crate::error::WebDriverResult;

string_enum! {
    /// Stage at which the `Fetch` agent intercepts requests.
    pub enum RequestStage {
        /// Intercept before the request is sent (default).
        Request = "Request",
        /// Intercept after the response headers are received.
        Response = "Response",
    }
}

/// `Fetch.RequestPattern` for selecting requests to intercept.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPattern {
    /// URL pattern (with `*` wildcards). Defaults to `"*"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    /// Resource type filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<ResourceType>,
    /// Stage at which to intercept. Defaults to [`RequestStage::Request`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_stage: Option<RequestStage>,
}

/// `Fetch.enable`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enable {
    /// Patterns to intercept. Empty = all requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patterns: Option<Vec<RequestPattern>>,
    /// If true, requests for which an authentication challenge is encountered are paused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle_auth_requests: Option<bool>,
}
impl CdpCommand for Enable {
    const METHOD: &'static str = "Fetch.enable";
    type Returns = Empty;
}

/// `Fetch.disable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Disable;
impl CdpCommand for Disable {
    const METHOD: &'static str = "Fetch.disable";
    type Returns = Empty;
}

/// One header for use in `Fetch.fulfillRequest` and `Fetch.continueRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderEntry {
    /// Header name.
    pub name: String,
    /// Header value.
    pub value: String,
}

/// `Fetch.continueRequest`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueRequest {
    /// Identifier of the paused request.
    pub request_id: FetchRequestId,
    /// Override URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Override method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Override request body (base64-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_data: Option<String>,
    /// Override request headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HeaderEntry>>,
}
impl CdpCommand for ContinueRequest {
    const METHOD: &'static str = "Fetch.continueRequest";
    type Returns = Empty;
}

/// `Fetch.fulfillRequest`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FulfillRequest {
    /// Identifier of the paused request.
    pub request_id: FetchRequestId,
    /// HTTP status code.
    pub response_code: u32,
    /// Response headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<Vec<HeaderEntry>>,
    /// Body (base64-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Response phrase (e.g. `"OK"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_phrase: Option<String>,
}
impl CdpCommand for FulfillRequest {
    const METHOD: &'static str = "Fetch.fulfillRequest";
    type Returns = Empty;
}

/// `Fetch.failRequest`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailRequest {
    /// Identifier of the paused request.
    pub request_id: FetchRequestId,
    /// Error reason. See [`ErrorReason`] for the closed set; use
    /// [`ErrorReason::Unknown`] to send a value the curated set doesn't
    /// have a typed variant for yet.
    pub error_reason: ErrorReason,
}
impl CdpCommand for FailRequest {
    const METHOD: &'static str = "Fetch.failRequest";
    type Returns = Empty;
}

/// `Fetch.requestPaused` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPaused {
    /// Identifier of the paused request.
    pub request_id: FetchRequestId,
    /// Request data.
    pub request: serde_json::Value,
    /// Frame id of the request.
    pub frame_id: String,
    /// Resource type.
    pub resource_type: ResourceType,
    /// Response error reason if intercepted at the response stage.
    pub response_error_reason: Option<ErrorReason>,
    /// Response status code if intercepted at the response stage.
    pub response_status_code: Option<u32>,
    /// Response status text if intercepted at the response stage.
    pub response_status_text: Option<String>,
    /// Response headers if intercepted at the response stage.
    pub response_headers: Option<Vec<HeaderEntry>>,
}
impl CdpEvent for RequestPaused {
    const METHOD: &'static str = "Fetch.requestPaused";
}

/// Domain facade returned by [`Cdp::fetch`].
#[derive(Debug)]
pub struct FetchDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> FetchDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Fetch.enable` for all requests at the request stage.
    pub async fn enable(&self) -> WebDriverResult<()> {
        self.cdp.send(Enable::default()).await?;
        Ok(())
    }

    /// `Fetch.disable`.
    pub async fn disable(&self) -> WebDriverResult<()> {
        self.cdp.send(Disable).await?;
        Ok(())
    }

    /// `Fetch.continueRequest` with no overrides — let the request through.
    pub async fn continue_request(&self, request_id: FetchRequestId) -> WebDriverResult<()> {
        self.cdp
            .send(ContinueRequest {
                request_id,
                url: None,
                method: None,
                post_data: None,
                headers: None,
            })
            .await?;
        Ok(())
    }

    /// `Fetch.failRequest` with the given [`ErrorReason`].
    pub async fn fail_request(
        &self,
        request_id: FetchRequestId,
        error_reason: ErrorReason,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(FailRequest {
                request_id,
                error_reason,
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
        assert_eq!(Enable::METHOD, "Fetch.enable");
        assert_eq!(Disable::METHOD, "Fetch.disable");
        assert_eq!(ContinueRequest::METHOD, "Fetch.continueRequest");
        assert_eq!(FulfillRequest::METHOD, "Fetch.fulfillRequest");
        assert_eq!(FailRequest::METHOD, "Fetch.failRequest");
    }

    #[test]
    fn event_methods() {
        assert_eq!(RequestPaused::METHOD, "Fetch.requestPaused");
    }

    #[test]
    fn enable_default_skips_optional_fields() {
        let v = serde_json::to_value(Enable::default()).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn enable_full() {
        let v = serde_json::to_value(Enable {
            patterns: Some(vec![RequestPattern {
                url_pattern: Some("*.json".to_string()),
                resource_type: Some(ResourceType::Xhr),
                request_stage: Some(RequestStage::Request),
            }]),
            handle_auth_requests: Some(true),
        })
        .unwrap();
        assert_eq!(v["patterns"][0]["urlPattern"], "*.json");
        assert_eq!(v["patterns"][0]["resourceType"], "XHR");
        assert_eq!(v["patterns"][0]["requestStage"], "Request");
        assert_eq!(v["handleAuthRequests"], true);
    }

    #[test]
    fn request_stage_round_trip() {
        for (variant, wire) in
            [(RequestStage::Request, "Request"), (RequestStage::Response, "Response")]
        {
            assert_eq!(serde_json::to_value(&variant).unwrap(), json!(wire));
        }
    }

    #[test]
    fn request_pattern_default_skips_all() {
        let v = serde_json::to_value(RequestPattern::default()).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn continue_request_minimal() {
        let v = serde_json::to_value(ContinueRequest {
            request_id: FetchRequestId::from("R1"),
            url: None,
            method: None,
            post_data: None,
            headers: None,
        })
        .unwrap();
        assert_eq!(v["requestId"], "R1");
        for f in ["url", "method", "postData", "headers"] {
            assert!(v.get(f).is_none(), "{f} should be omitted");
        }
    }

    #[test]
    fn continue_request_full_field_names() {
        let v = serde_json::to_value(ContinueRequest {
            request_id: FetchRequestId::from("R1"),
            url: Some("https://x".to_string()),
            method: Some("POST".to_string()),
            post_data: Some("YWI=".to_string()),
            headers: Some(vec![HeaderEntry {
                name: "X".to_string(),
                value: "Y".to_string(),
            }]),
        })
        .unwrap();
        assert_eq!(v["url"], "https://x");
        assert_eq!(v["method"], "POST");
        assert_eq!(v["postData"], "YWI=");
        assert_eq!(v["headers"][0]["name"], "X");
        assert_eq!(v["headers"][0]["value"], "Y");
    }

    #[test]
    fn fulfill_request_required_fields() {
        let v = serde_json::to_value(FulfillRequest {
            request_id: FetchRequestId::from("R1"),
            response_code: 200,
            response_headers: None,
            body: None,
            response_phrase: None,
        })
        .unwrap();
        assert_eq!(v["requestId"], "R1");
        assert_eq!(v["responseCode"], 200);
        assert!(v.get("body").is_none());
    }

    #[test]
    fn fulfill_request_with_body() {
        let v = serde_json::to_value(FulfillRequest {
            request_id: FetchRequestId::from("R1"),
            response_code: 201,
            response_headers: Some(vec![HeaderEntry {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            }]),
            body: Some("e30=".to_string()),
            response_phrase: Some("Created".to_string()),
        })
        .unwrap();
        assert_eq!(v["responseHeaders"][0]["name"], "Content-Type");
        assert_eq!(v["body"], "e30=");
        assert_eq!(v["responsePhrase"], "Created");
    }

    #[test]
    fn fail_request_serialises_with_typed_reason() {
        let v = serde_json::to_value(FailRequest {
            request_id: FetchRequestId::from("R1"),
            error_reason: ErrorReason::Failed,
        })
        .unwrap();
        assert_eq!(v["requestId"], "R1");
        assert_eq!(v["errorReason"], "Failed");
    }

    #[test]
    fn fail_request_with_unknown_reason() {
        // Forward-compat path: a future browser error reason can still
        // be sent via `ErrorReason::Unknown` without waiting for a
        // thirtyfour update.
        let v = serde_json::to_value(FailRequest {
            request_id: FetchRequestId::from("R1"),
            error_reason: ErrorReason::Unknown("FutureReason".to_string()),
        })
        .unwrap();
        assert_eq!(v["errorReason"], "FutureReason");
    }

    // Round-trip and Unknown-variant tests for `ErrorReason` and
    // `ResourceType` live alongside their canonical definitions in
    // `cdp/domains/network.rs`.

    #[test]
    fn request_paused_event_parses() {
        let body = json!({
            "requestId": "R1",
            "request": {"url": "https://x"},
            "frameId": "F1",
            "resourceType": "Document"
        });
        let evt: RequestPaused = serde_json::from_value(body).unwrap();
        assert_eq!(evt.request_id.as_str(), "R1");
        assert_eq!(evt.frame_id, "F1");
        assert_eq!(evt.resource_type, ResourceType::Document);
        assert!(evt.response_status_code.is_none());
    }

    #[test]
    fn request_paused_at_response_stage() {
        let body = json!({
            "requestId": "R1",
            "request": {"url": "https://x"},
            "frameId": "F1",
            "resourceType": "Document",
            "responseErrorReason": null,
            "responseStatusCode": 200,
            "responseStatusText": "OK",
            "responseHeaders": [{"name": "X", "value": "Y"}]
        });
        let evt: RequestPaused = serde_json::from_value(body).unwrap();
        assert_eq!(evt.response_status_code, Some(200));
        assert_eq!(evt.response_status_text.as_deref(), Some("OK"));
        assert!(evt.response_headers.is_some());
    }
}
