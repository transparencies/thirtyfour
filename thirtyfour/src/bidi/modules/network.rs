//! `network.*` BiDi module — request observation, interception, and modify
//! request/response.

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, BidiEvent, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, InterceptId, RequestId, UserContextId};
use crate::bidi::macros::string_enum;

string_enum! {
    /// Phase a [`AddIntercept`] should fire on.
    pub enum InterceptPhase {
        /// Before the request leaves the network stack.
        BeforeRequestSent = "beforeRequestSent",
        /// When the response arrives, before delivery.
        ResponseStarted = "responseStarted",
        /// On HTTP 401 / proxy auth.
        AuthRequired = "authRequired",
    }
}

string_enum! {
    /// HTTP cache mode for [`SetCacheBehavior`].
    pub enum CacheBehavior {
        /// Default — honour cache headers.
        Default = "default",
        /// Bypass cache entirely.
        Bypass = "bypass",
    }
}

/// `network.addIntercept`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIntercept {
    /// Phase(s) to intercept.
    pub phases: Vec<InterceptPhase>,
    /// Restrict to specific browsing contexts. Empty = global.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
    /// URL match patterns. Each is a string-or-object per BiDi
    /// `network.UrlPattern`. Pass-through as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_patterns: Option<Vec<serde_json::Value>>,
}

impl BidiCommand for AddIntercept {
    const METHOD: &'static str = "network.addIntercept";
    type Returns = AddInterceptResult;
}

/// Response for [`AddIntercept`].
#[derive(Debug, Clone, Deserialize)]
pub struct AddInterceptResult {
    /// Server-assigned intercept id.
    pub intercept: InterceptId,
}

/// `network.removeIntercept`.
#[derive(Debug, Clone, Serialize)]
pub struct RemoveIntercept {
    /// Intercept id returned by [`AddIntercept`].
    pub intercept: InterceptId,
}

impl BidiCommand for RemoveIntercept {
    const METHOD: &'static str = "network.removeIntercept";
    type Returns = Empty;
}

/// `network.continueRequest` — let an intercepted request proceed,
/// optionally with modifications.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueRequest {
    /// Request to continue.
    pub request: RequestId,
    /// Override request body (BiDi `network.BytesValue`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// Override cookies (BiDi `network.CookieHeader[]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<serde_json::Value>>,
    /// Override headers (BiDi `network.Header[]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<serde_json::Value>>,
    /// Override HTTP method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Override URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl BidiCommand for ContinueRequest {
    const METHOD: &'static str = "network.continueRequest";
    type Returns = Empty;
}

/// `network.continueResponse` — let an intercepted response proceed,
/// optionally with modifications.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueResponse {
    /// Request whose response is being continued.
    pub request: RequestId,
    /// Override cookies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<serde_json::Value>>,
    /// Override credentials (basic-auth).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<serde_json::Value>,
    /// Override headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<serde_json::Value>>,
    /// Override status reason phrase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_phrase: Option<String>,
    /// Override status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

impl BidiCommand for ContinueResponse {
    const METHOD: &'static str = "network.continueResponse";
    type Returns = Empty;
}

/// `network.continueWithAuth` — answer an `authRequired` intercept.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueWithAuth {
    /// Request to continue.
    pub request: RequestId,
    /// Action and credentials. Use [`AuthAction`] helpers below.
    #[serde(flatten)]
    pub action: AuthAction,
}

/// Action passed to [`ContinueWithAuth`].
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum AuthAction {
    /// Provide credentials.
    ProvideCredentials {
        /// Credentials object.
        credentials: AuthCredentials,
    },
    /// Cancel the auth request.
    Cancel,
    /// Default — let the browser handle it.
    Default,
}

/// Basic-auth credentials.
#[derive(Debug, Clone, Serialize)]
pub struct AuthCredentials {
    /// Auth scheme (`"basic"`).
    pub r#type: String,
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
}

impl AuthCredentials {
    /// Construct a basic-auth credentials object.
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            r#type: "basic".to_string(),
            username: username.into(),
            password: password.into(),
        }
    }
}

impl BidiCommand for ContinueWithAuth {
    const METHOD: &'static str = "network.continueWithAuth";
    type Returns = Empty;
}

/// `network.failRequest` — fail an intercepted request.
#[derive(Debug, Clone, Serialize)]
pub struct FailRequest {
    /// Request to fail.
    pub request: RequestId,
}

impl BidiCommand for FailRequest {
    const METHOD: &'static str = "network.failRequest";
    type Returns = Empty;
}

/// `network.provideResponse` — synthesize a response for an intercepted
/// request (instead of letting it reach the network).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideResponse {
    /// Request to respond to.
    pub request: RequestId,
    /// Response body (`network.BytesValue`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// Cookies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<serde_json::Value>>,
    /// Headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<serde_json::Value>>,
    /// Reason phrase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_phrase: Option<String>,
    /// Status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

impl BidiCommand for ProvideResponse {
    const METHOD: &'static str = "network.provideResponse";
    type Returns = Empty;
}

/// `network.setCacheBehavior`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCacheBehavior {
    /// Cache mode.
    pub cache_behavior: CacheBehavior,
    /// Restrict to specific browsing contexts. Empty = global.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
    /// Restrict to specific user contexts. Empty = global.
    #[serde(rename = "userContexts", skip_serializing_if = "Vec::is_empty")]
    pub user_contexts: Vec<UserContextId>,
}

impl BidiCommand for SetCacheBehavior {
    const METHOD: &'static str = "network.setCacheBehavior";
    type Returns = Empty;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events surfaced by the `network.*` module.
pub mod events {
    use super::*;

    /// `network.beforeRequestSent`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BeforeRequestSent {
        /// Browsing context the request belongs to (top-level for nav).
        pub context: Option<BrowsingContextId>,
        /// Underlying request data (BiDi `network.RequestData`).
        pub request: RequestData,
        /// Driver timestamp (ms since epoch).
        pub timestamp: u64,
        /// Optional initiator info.
        #[serde(default)]
        pub initiator: Option<serde_json::Value>,
        /// Whether the request is currently blocked by an intercept.
        #[serde(default)]
        pub is_blocked: bool,
        /// Intercept ids that match this request.
        #[serde(default)]
        pub intercepts: Vec<InterceptId>,
    }

    impl BidiEvent for BeforeRequestSent {
        const METHOD: &'static str = "network.beforeRequestSent";
    }

    /// `network.responseStarted`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseStarted {
        /// Browsing context.
        pub context: Option<BrowsingContextId>,
        /// Request data.
        pub request: RequestData,
        /// Response data.
        pub response: ResponseData,
        /// Driver timestamp.
        pub timestamp: u64,
        /// Whether currently blocked by an intercept.
        #[serde(default)]
        pub is_blocked: bool,
        /// Intercept ids that match this response.
        #[serde(default)]
        pub intercepts: Vec<InterceptId>,
    }

    impl BidiEvent for ResponseStarted {
        const METHOD: &'static str = "network.responseStarted";
    }

    /// `network.responseCompleted`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseCompleted {
        /// Browsing context.
        pub context: Option<BrowsingContextId>,
        /// Request data.
        pub request: RequestData,
        /// Response data.
        pub response: ResponseData,
        /// Driver timestamp.
        pub timestamp: u64,
    }

    impl BidiEvent for ResponseCompleted {
        const METHOD: &'static str = "network.responseCompleted";
    }

    /// `network.fetchError`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FetchError {
        /// Browsing context.
        pub context: Option<BrowsingContextId>,
        /// Request data.
        pub request: RequestData,
        /// Driver timestamp.
        pub timestamp: u64,
        /// Error string (browser-defined).
        pub error_text: String,
    }

    impl BidiEvent for FetchError {
        const METHOD: &'static str = "network.fetchError";
    }

    /// `network.authRequired`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AuthRequired {
        /// Browsing context.
        pub context: Option<BrowsingContextId>,
        /// Request data.
        pub request: RequestData,
        /// Response data (status will be 401 / 407).
        pub response: ResponseData,
        /// Driver timestamp.
        pub timestamp: u64,
        /// Whether currently blocked by an intercept.
        #[serde(default)]
        pub is_blocked: bool,
    }

    impl BidiEvent for AuthRequired {
        const METHOD: &'static str = "network.authRequired";
    }
}

// ---------------------------------------------------------------------------
// Wire-shape types reused by events.
// ---------------------------------------------------------------------------

/// Subset of BiDi's `network.RequestData` we model strongly. Vendor or
/// rarely-used fields fall through to [`RequestData::extra`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestData {
    /// Request id.
    pub request: RequestId,
    /// HTTP method.
    pub method: String,
    /// Full URL.
    pub url: String,
    /// Headers (`network.Header[]`).
    #[serde(default)]
    pub headers: Vec<serde_json::Value>,
    /// Cookies sent on the request.
    #[serde(default)]
    pub cookies: Vec<serde_json::Value>,
    /// Other fields (timing, body size, …).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Subset of BiDi's `network.ResponseData` we model strongly.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseData {
    /// Final URL after redirects.
    pub url: String,
    /// HTTP protocol (`"http/1.1"`, `"h2"`, …).
    #[serde(default)]
    pub protocol: Option<String>,
    /// HTTP status code.
    pub status: u16,
    /// Status reason phrase.
    pub status_text: String,
    /// Headers.
    #[serde(default)]
    pub headers: Vec<serde_json::Value>,
    /// Other fields (mime type, encoding, …).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Module facade returned by [`BiDi::network`].
#[derive(Debug)]
pub struct NetworkModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> NetworkModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// `network.addIntercept`.
    pub async fn add_intercept(
        &self,
        phases: Vec<InterceptPhase>,
        url_patterns: Option<Vec<serde_json::Value>>,
    ) -> Result<AddInterceptResult, BidiError> {
        self.bidi
            .send(AddIntercept {
                phases,
                contexts: vec![],
                url_patterns,
            })
            .await
    }

    /// `network.removeIntercept`.
    pub async fn remove_intercept(&self, intercept: InterceptId) -> Result<(), BidiError> {
        self.bidi
            .send(RemoveIntercept {
                intercept,
            })
            .await?;
        Ok(())
    }

    /// `network.continueRequest` (no modifications).
    pub async fn continue_request(&self, request: RequestId) -> Result<(), BidiError> {
        self.bidi
            .send(ContinueRequest {
                request,
                body: None,
                cookies: None,
                headers: None,
                method: None,
                url: None,
            })
            .await?;
        Ok(())
    }

    /// `network.continueResponse` (no modifications).
    pub async fn continue_response(&self, request: RequestId) -> Result<(), BidiError> {
        self.bidi
            .send(ContinueResponse {
                request,
                cookies: None,
                credentials: None,
                headers: None,
                reason_phrase: None,
                status_code: None,
            })
            .await?;
        Ok(())
    }

    /// `network.failRequest`.
    pub async fn fail_request(&self, request: RequestId) -> Result<(), BidiError> {
        self.bidi
            .send(FailRequest {
                request,
            })
            .await?;
        Ok(())
    }

    /// `network.setCacheBehavior` — global.
    pub async fn set_cache_behavior(&self, mode: CacheBehavior) -> Result<(), BidiError> {
        self.bidi
            .send(SetCacheBehavior {
                cache_behavior: mode,
                contexts: vec![],
                user_contexts: vec![],
            })
            .await?;
        Ok(())
    }
}
