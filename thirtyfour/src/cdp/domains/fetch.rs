//! `Fetch` domain — request/response interception.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::cdp::domains::network::{ErrorReason, ResourceType};
use crate::cdp::ids::FetchRequestId;
use crate::common::protocol::string_enum;
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
