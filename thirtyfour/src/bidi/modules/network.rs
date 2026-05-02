//! `network.*` — observe, intercept, and modify HTTP traffic.
//!
//! This module covers four overlapping use-cases:
//!
//! 1. **Observation** — subscribe to [`BeforeRequestSent`][brs],
//!    [`ResponseStarted`][rs], [`ResponseCompleted`][rc], or
//!    [`FetchError`][fe] to watch every request the page makes
//!    (including subresources) without altering it.
//! 2. **Interception** — register an [`AddIntercept`][ai] to pause
//!    requests in selected phases, then continue, fail, or synthesize
//!    a response using [`ContinueRequest`][creq] / [`FailRequest`][fr]
//!    / [`ProvideResponse`][pr] / [`ContinueResponse`][cresp] /
//!    [`ContinueWithAuth`][cwa].
//! 3. **Body capture** — register an [`AddDataCollector`][adc] to
//!    retain request and/or response bodies, then read them back via
//!    [`GetData`][gd].
//! 4. **Header injection / cache control** — apply session-level
//!    [`SetExtraHeaders`][seh] and [`SetCacheBehavior`][scb] without
//!    touching individual requests.
//!
//! See the [W3C `network` module specification][spec] for the canonical
//! definitions, including [URL pattern matching][spec-urlpattern] (used by
//! intercepts) and the request/response data shapes.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-network
//! [spec-urlpattern]: https://w3c.github.io/webdriver-bidi/#type-network-UrlPattern
//! [brs]: crate::bidi::modules::network::events::BeforeRequestSent
//! [rs]: crate::bidi::modules::network::events::ResponseStarted
//! [rc]: crate::bidi::modules::network::events::ResponseCompleted
//! [fe]: crate::bidi::modules::network::events::FetchError
//! [ai]: crate::bidi::modules::network::AddIntercept
//! [creq]: crate::bidi::modules::network::ContinueRequest
//! [fr]: crate::bidi::modules::network::FailRequest
//! [pr]: crate::bidi::modules::network::ProvideResponse
//! [cresp]: crate::bidi::modules::network::ContinueResponse
//! [cwa]: crate::bidi::modules::network::ContinueWithAuth
//! [adc]: crate::bidi::modules::network::AddDataCollector
//! [gd]: crate::bidi::modules::network::GetData
//! [seh]: crate::bidi::modules::network::SetExtraHeaders
//! [scb]: crate::bidi::modules::network::SetCacheBehavior

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, BidiEvent, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, CollectorId, InterceptId, RequestId, UserContextId};
use crate::common::protocol::string_enum;

string_enum! {
    /// Phase that an [`AddIntercept`] should pause on.
    ///
    /// Mirrors the spec's [`network.InterceptPhase`][spec] enumeration.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-InterceptPhase
    pub enum InterceptPhase {
        /// Pause the request before it leaves the network stack. The
        /// matching [`events::BeforeRequestSent`] event arrives with
        /// `is_blocked = true`. Resume with [`ContinueRequest`],
        /// abandon with [`FailRequest`], or fake a response with
        /// [`ProvideResponse`].
        BeforeRequestSent = "beforeRequestSent",
        /// Pause the response when its headers arrive but before it is
        /// delivered to the page. The matching
        /// [`events::ResponseStarted`] event arrives with
        /// `is_blocked = true`. Resume with [`ContinueResponse`].
        ResponseStarted = "responseStarted",
        /// Pause on HTTP 401 / 407 challenges. The matching
        /// [`events::AuthRequired`] event arrives with
        /// `is_blocked = true`. Resume with [`ContinueWithAuth`].
        AuthRequired = "authRequired",
    }
}

string_enum! {
    /// HTTP cache mode for [`SetCacheBehavior`]. Mirrors the spec's
    /// `cacheBehavior` enumeration on
    /// [`network.setCacheBehavior`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-setCacheBehavior
    pub enum CacheBehavior {
        /// Honour cache headers (the browser's default behaviour).
        Default = "default",
        /// Bypass any in-browser cache.
        Bypass = "bypass",
    }
}

string_enum! {
    /// Body kind for [`AddDataCollector::data_types`] and
    /// [`GetData::data_type`].
    ///
    /// Mirrors the spec's [`network.DataType`][spec] type.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-DataType
    pub enum DataType {
        /// Request bodies.
        Request = "request",
        /// Response bodies.
        Response = "response",
    }
}

string_enum! {
    /// Storage backing for an [`AddDataCollector`].
    ///
    /// Currently only `blob` is defined; the enum exists for forward
    /// compatibility (the spec mentions a future `stream` collector
    /// type).
    pub enum CollectorType {
        /// In-memory blob.
        Blob = "blob",
    }
}

/// [`network.addIntercept`][spec] — register an interception that pauses
/// requests in the named phases.
///
/// `phases` selects which lifecycle stages will pause. `url_patterns`
/// scopes the intercept to URLs matching any of the supplied
/// [`network.UrlPattern`s][pattern] (string-or-object form, passed through
/// as JSON); empty/None matches every URL. `contexts` scopes to specific
/// top-level browsing contexts.
///
/// Prefer the convenience [`NetworkModule::add_intercept`] facade — it
/// returns an [`InterceptGuard`] that auto-removes the intercept on drop.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-addIntercept
/// [pattern]: https://w3c.github.io/webdriver-bidi/#type-network-UrlPattern
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIntercept {
    /// Phase(s) to intercept.
    pub phases: Vec<InterceptPhase>,
    /// Restrict to specific top-level browsing contexts. Empty = any.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
    /// URL match patterns. Each entry is a [`network.UrlPattern`][pattern]
    /// JSON value (string variant: `{"type":"string","pattern":"…"}`,
    /// object variant: `{"type":"pattern","hostname":"…",…}`).
    ///
    /// [pattern]: https://w3c.github.io/webdriver-bidi/#type-network-UrlPattern
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
    /// Server-assigned intercept id. Pass to [`RemoveIntercept`] to drop
    /// the interception.
    pub intercept: InterceptId,
}

/// [`network.removeIntercept`][spec] — drop a previously-registered
/// intercept.
///
/// Already-paused requests are unaffected — only future requests stop
/// matching. Use [`InterceptGuard`] to manage this automatically.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-removeIntercept
#[derive(Debug, Clone, Serialize)]
pub struct RemoveIntercept {
    /// Intercept id returned by [`AddIntercept`].
    pub intercept: InterceptId,
}

impl BidiCommand for RemoveIntercept {
    const METHOD: &'static str = "network.removeIntercept";
    type Returns = Empty;
}

/// [`network.continueRequest`][spec] — release a paused request,
/// optionally with modifications.
///
/// Only valid for requests paused in the
/// [`InterceptPhase::BeforeRequestSent`] phase. Each optional field
/// overrides the corresponding part of the original request. Bytes-typed
/// fields use the spec's [`network.BytesValue`][bytes]
/// (`{"type":"string","value":…}` or `{"type":"base64","value":…}`).
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-continueRequest
/// [bytes]: https://w3c.github.io/webdriver-bidi/#type-network-BytesValue
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueRequest {
    /// Request to continue.
    pub request: RequestId,
    /// Override request body (BiDi [`network.BytesValue`][bytes]).
    ///
    /// [bytes]: https://w3c.github.io/webdriver-bidi/#type-network-BytesValue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// Override cookies — array of
    /// [`network.CookieHeader`][cookie] JSON values.
    ///
    /// [cookie]: https://w3c.github.io/webdriver-bidi/#type-network-CookieHeader
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<serde_json::Value>>,
    /// Override headers — array of
    /// [`network.Header`][header] JSON values.
    ///
    /// [header]: https://w3c.github.io/webdriver-bidi/#type-network-Header
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<serde_json::Value>>,
    /// Override the HTTP method (e.g. `"POST"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Override the request URL (must be an absolute URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl BidiCommand for ContinueRequest {
    const METHOD: &'static str = "network.continueRequest";
    type Returns = Empty;
}

/// [`network.continueResponse`][spec] — release a paused response,
/// optionally with modifications.
///
/// Only valid for requests paused in
/// [`InterceptPhase::ResponseStarted`]. The response body is delivered
/// from the network as-is; only the status, headers, cookies, and
/// (for an `authRequired` follow-up) credentials can be overridden.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-continueResponse
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueResponse {
    /// Request whose response is being continued.
    pub request: RequestId,
    /// Override cookies — array of
    /// [`network.SetCookieHeader`][cookie] JSON values.
    ///
    /// [cookie]: https://w3c.github.io/webdriver-bidi/#type-network-SetCookieHeader
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<serde_json::Value>>,
    /// Override credentials (BiDi [`network.AuthCredentials`][creds]).
    ///
    /// [creds]: https://w3c.github.io/webdriver-bidi/#type-network-AuthCredentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<serde_json::Value>,
    /// Override headers — array of [`network.Header`][header] JSON values.
    ///
    /// [header]: https://w3c.github.io/webdriver-bidi/#type-network-Header
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<serde_json::Value>>,
    /// Override the status reason phrase (e.g. `"OK"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_phrase: Option<String>,
    /// Override the status code (e.g. `200`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

impl BidiCommand for ContinueResponse {
    const METHOD: &'static str = "network.continueResponse";
    type Returns = Empty;
}

/// [`network.continueWithAuth`][spec] — answer an `authRequired` intercept.
///
/// Use the [`AuthAction`] enum to pick provide-credentials, cancel the
/// challenge, or fall back to the browser's default behaviour (which is
/// to prompt the user).
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-continueWithAuth
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueWithAuth {
    /// Request to continue.
    pub request: RequestId,
    /// Action and credentials.
    #[serde(flatten)]
    pub action: AuthAction,
}

/// Action variant for [`ContinueWithAuth`].
///
/// Mirrors the spec's `network.ContinueWithAuthCredentials` /
/// `network.ContinueWithAuthNoCredentials` union.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum AuthAction {
    /// Provide credentials to satisfy the challenge.
    ProvideCredentials {
        /// Credentials object.
        credentials: AuthCredentials,
    },
    /// Cancel the auth request — the user-agent treats this as if the
    /// user dismissed the dialog.
    Cancel,
    /// Default — fall back to the browser's normal behaviour
    /// (typically prompts the user).
    Default,
}

/// Basic-auth credentials. Mirrors
/// [`network.AuthCredentials`][spec].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-AuthCredentials
#[derive(Debug, Clone, Serialize)]
pub struct AuthCredentials {
    /// Auth scheme. The spec only defines `"password"`.
    pub r#type: String,
    /// Username.
    pub username: String,
    /// Password.
    pub password: String,
}

impl AuthCredentials {
    /// Construct a basic-auth ("password") credentials object.
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            r#type: "password".to_string(),
            username: username.into(),
            password: password.into(),
        }
    }
}

impl BidiCommand for ContinueWithAuth {
    const METHOD: &'static str = "network.continueWithAuth";
    type Returns = Empty;
}

/// [`network.failRequest`][spec] — fail an intercepted request.
///
/// Only valid in [`InterceptPhase::BeforeRequestSent`]. The page sees the
/// request as a network error.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-failRequest
#[derive(Debug, Clone, Serialize)]
pub struct FailRequest {
    /// Request to fail.
    pub request: RequestId,
}

impl BidiCommand for FailRequest {
    const METHOD: &'static str = "network.failRequest";
    type Returns = Empty;
}

/// [`network.provideResponse`][spec] — supply a synthetic response for
/// an intercepted request (the actual network request is skipped).
///
/// Most useful in [`InterceptPhase::BeforeRequestSent`] for stubbing
/// fetches in tests. The response progresses through the rest of the
/// lifecycle as if it had come from the wire.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-provideResponse
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvideResponse {
    /// Request to respond to.
    pub request: RequestId,
    /// Response body — a [`network.BytesValue`][bytes].
    ///
    /// [bytes]: https://w3c.github.io/webdriver-bidi/#type-network-BytesValue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// `Set-Cookie` headers — array of
    /// [`network.SetCookieHeader`][cookie] JSON values.
    ///
    /// [cookie]: https://w3c.github.io/webdriver-bidi/#type-network-SetCookieHeader
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies: Option<Vec<serde_json::Value>>,
    /// Response headers — array of [`network.Header`][header] JSON values.
    ///
    /// [header]: https://w3c.github.io/webdriver-bidi/#type-network-Header
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<serde_json::Value>>,
    /// Reason phrase (e.g. `"OK"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_phrase: Option<String>,
    /// Status code (defaults to `200` if omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

impl BidiCommand for ProvideResponse {
    const METHOD: &'static str = "network.provideResponse";
    type Returns = Empty;
}

/// [`network.setCacheBehavior`][spec] — switch HTTP-cache behaviour
/// globally or for specific contexts.
///
/// `Bypass` disables the in-browser cache for matching requests
/// (analogous to Chrome's "Disable cache" checkbox).
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-setCacheBehavior
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCacheBehavior {
    /// New cache mode.
    pub cache_behavior: CacheBehavior,
    /// Restrict to specific browsing contexts. Empty = applies as the
    /// new default.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
    /// Restrict to specific user contexts. Empty = applies as the new
    /// default.
    #[serde(rename = "userContexts", skip_serializing_if = "Vec::is_empty")]
    pub user_contexts: Vec<UserContextId>,
}

impl BidiCommand for SetCacheBehavior {
    const METHOD: &'static str = "network.setCacheBehavior";
    type Returns = Empty;
}

/// [`network.addDataCollector`][spec] — start retaining request and/or
/// response bodies for later retrieval via [`GetData`].
///
/// `max_encoded_data_size` is a per-item byte limit; bodies that exceed
/// it are still observed but not retained. Scope is mutually exclusive:
/// supply at most one of `contexts` / `user_contexts`. Either matches a
/// set of top-level traversables; an unscoped collector retains data from
/// every navigation.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-addDataCollector
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddDataCollector {
    /// Body kinds to retain.
    pub data_types: Vec<DataType>,
    /// Per-item maximum encoded size in bytes. Items larger than this
    /// are not retained.
    pub max_encoded_data_size: u64,
    /// Backing storage type (defaults to [`CollectorType::Blob`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collector_type: Option<CollectorType>,
    /// Restrict to specific top-level browsing contexts. Mutually
    /// exclusive with [`user_contexts`][Self::user_contexts].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// Restrict to specific user contexts. Mutually exclusive with
    /// [`contexts`][Self::contexts].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for AddDataCollector {
    const METHOD: &'static str = "network.addDataCollector";
    type Returns = AddDataCollectorResult;
}

/// Response for [`AddDataCollector`].
#[derive(Debug, Clone, Deserialize)]
pub struct AddDataCollectorResult {
    /// Server-assigned collector id. Pass to [`GetData`],
    /// [`DisownData`], or [`RemoveDataCollector`].
    pub collector: CollectorId,
}

/// [`network.removeDataCollector`][spec] — drop a data collector and
/// release every request's retained data for that collector.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-removeDataCollector
#[derive(Debug, Clone, Serialize)]
pub struct RemoveDataCollector {
    /// Collector id to remove.
    pub collector: CollectorId,
}

impl BidiCommand for RemoveDataCollector {
    const METHOD: &'static str = "network.removeDataCollector";
    type Returns = Empty;
}

/// [`network.getData`][spec] — fetch retained body data for a request.
///
/// `collector` is optional: if omitted, the driver returns the data from
/// any collector that retained it. `disown: true` releases the data
/// after returning it (saves memory) — but requires `collector` to be
/// supplied so the driver knows which collector to release from.
///
/// Errors:
/// - `no such network collector` — `collector` references an unknown id.
/// - `no such network data` — no collector retained body data for the
///   `(request, data_type)` pair.
/// - `unavailable network data` — data was retained but is no longer
///   available (typically because it exceeded `max_encoded_data_size`).
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-getData
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetData {
    /// Body kind to retrieve.
    pub data_type: DataType,
    /// Request to look up.
    pub request: RequestId,
    /// Restrict to a specific collector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collector: Option<CollectorId>,
    /// If `true`, release the data after fetching (requires
    /// `collector`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disown: Option<bool>,
}

impl BidiCommand for GetData {
    const METHOD: &'static str = "network.getData";
    type Returns = GetDataResult;
}

/// Response for [`GetData`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetDataResult {
    /// Body payload as a [`network.BytesValue`][spec]
    /// (`{"type":"string","value":…}` or
    /// `{"type":"base64","value":…}`).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-BytesValue
    pub bytes: serde_json::Value,
}

/// [`network.disownData`][spec] — release retained body data for one
/// collector without removing the collector itself.
///
/// Useful when you want to keep collecting future data but explicitly
/// drop one captured request's payload.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-disownData
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisownData {
    /// Body kind to release.
    pub data_type: DataType,
    /// Collector to disown for.
    pub collector: CollectorId,
    /// Request to release data for.
    pub request: RequestId,
}

impl BidiCommand for DisownData {
    const METHOD: &'static str = "network.disownData";
    type Returns = Empty;
}

/// [`network.setExtraHeaders`][spec] — install headers that the driver
/// adds to every outgoing request (overwriting matching headers).
///
/// Scope:
/// - No `contexts` / `user_contexts` → set the default extra headers.
/// - `contexts` → scope to those top-level traversables.
/// - `user_contexts` → scope to those user contexts.
///
/// `contexts` and `user_contexts` are mutually exclusive.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-setExtraHeaders
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetExtraHeaders {
    /// Headers to install. Each entry is a [`network.Header`][header]
    /// JSON value (`{"name":"…","value":{"type":"string","value":"…"}}`).
    ///
    /// [header]: https://w3c.github.io/webdriver-bidi/#type-network-Header
    pub headers: Vec<serde_json::Value>,
    /// Restrict to these top-level browsing contexts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// Restrict to these user contexts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetExtraHeaders {
    const METHOD: &'static str = "network.setExtraHeaders";
    type Returns = Empty;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events surfaced by the `network.*` module.
pub(crate) mod events {
    use super::*;

    /// [`network.beforeRequestSent`][spec] — fires just before a request
    /// leaves the network stack.
    ///
    /// If `is_blocked` is `true`, an intercept paused the request — call
    /// [`ContinueRequest`], [`FailRequest`], or [`ProvideResponse`] with
    /// `request.id`.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-network-beforeRequestSent
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BeforeRequestSent {
        /// Browsing context the request belongs to. `None` for requests
        /// without an associated navigable (e.g. service workers).
        pub context: Option<BrowsingContextId>,
        /// Underlying request data.
        pub request: RequestData,
        /// Driver timestamp (ms since epoch).
        pub timestamp: u64,
        /// Optional initiator info ([`network.Initiator`][init]).
        ///
        /// [init]: https://w3c.github.io/webdriver-bidi/#type-network-Initiator
        #[serde(default)]
        pub initiator: Option<serde_json::Value>,
        /// Whether the request is currently blocked by an intercept.
        #[serde(default)]
        pub is_blocked: bool,
        /// Intercept ids that match this request (only populated when
        /// `is_blocked` is `true`).
        #[serde(default)]
        pub intercepts: Vec<InterceptId>,
    }

    impl BidiEvent for BeforeRequestSent {
        const METHOD: &'static str = "network.beforeRequestSent";
    }

    /// [`network.responseStarted`][spec] — fires when response headers
    /// arrive but before the body is delivered.
    ///
    /// If `is_blocked` is `true`, an intercept paused the response —
    /// call [`ContinueResponse`] with `request.id`.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-network-responseStarted
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseStarted {
        /// Browsing context.
        pub context: Option<BrowsingContextId>,
        /// Request data.
        pub request: RequestData,
        /// Response data (headers, status, …).
        pub response: ResponseData,
        /// Driver timestamp.
        pub timestamp: u64,
        /// Whether the response is currently blocked by an intercept.
        #[serde(default)]
        pub is_blocked: bool,
        /// Intercept ids that match this response.
        #[serde(default)]
        pub intercepts: Vec<InterceptId>,
    }

    impl BidiEvent for ResponseStarted {
        const METHOD: &'static str = "network.responseStarted";
    }

    /// [`network.responseCompleted`][spec] — fires when the response is
    /// fully received (or the request is cancelled).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-network-responseCompleted
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

    /// [`network.fetchError`][spec] — fires when a request fails before
    /// completing (network error, DNS failure, blocked by extension, …).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-network-fetchError
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FetchError {
        /// Browsing context.
        pub context: Option<BrowsingContextId>,
        /// Request data.
        pub request: RequestData,
        /// Driver timestamp.
        pub timestamp: u64,
        /// Browser-defined error text.
        pub error_text: String,
    }

    impl BidiEvent for FetchError {
        const METHOD: &'static str = "network.fetchError";
    }

    /// [`network.authRequired`][spec] — fires on an HTTP 401 / 407
    /// challenge. Pair with [`ContinueWithAuth`] when an
    /// [`InterceptPhase::AuthRequired`] intercept matches.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-network-authRequired
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AuthRequired {
        /// Browsing context.
        pub context: Option<BrowsingContextId>,
        /// Request data.
        pub request: RequestData,
        /// Response data (status will be 401 or 407; `authChallenges` is
        /// populated).
        pub response: ResponseData,
        /// Driver timestamp.
        pub timestamp: u64,
        /// Whether the response is currently blocked by an intercept.
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

/// Strongly-typed subset of BiDi's [`network.RequestData`][spec].
///
/// Vendor-specific or rarely-used fields (timing breakdown, body size,
/// destination, initiator type, …) flow through into
/// [`RequestData::extra`]. Strongly-typed extras can be added later
/// without breaking the wire shape.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-RequestData
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestData {
    /// Request id. The BiDi wire shape names this field `request` — we
    /// rename it to `id` so callers can write
    /// `event.request.id` instead of the doubly-nested
    /// `event.request.request`.
    #[serde(rename = "request")]
    pub id: RequestId,
    /// HTTP method (`"GET"`, `"POST"`, …).
    pub method: String,
    /// Full request URL.
    pub url: String,
    /// Headers — array of [`network.Header`][hdr].
    ///
    /// [hdr]: https://w3c.github.io/webdriver-bidi/#type-network-Header
    #[serde(default)]
    pub headers: Vec<serde_json::Value>,
    /// Cookies sent on the request — array of
    /// [`network.Cookie`][cookie].
    ///
    /// [cookie]: https://w3c.github.io/webdriver-bidi/#type-network-Cookie
    #[serde(default)]
    pub cookies: Vec<serde_json::Value>,
    /// Other fields from the wire shape (timing, body size, destination,
    /// initiator type, …).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Strongly-typed subset of BiDi's [`network.ResponseData`][spec].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-network-ResponseData
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
    /// Headers — array of [`network.Header`][hdr].
    ///
    /// [hdr]: https://w3c.github.io/webdriver-bidi/#type-network-Header
    #[serde(default)]
    pub headers: Vec<serde_json::Value>,
    /// Other fields from the wire shape (mime type, encoded vs decoded
    /// size, `authChallenges`, …).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// RAII guard around an active `network.addIntercept` registration.
///
/// Returned by [`NetworkModule::add_intercept`]. Holds the
/// [`InterceptId`] so you can pass it explicitly to
/// [`NetworkModule::remove_intercept`] or call [`Self::remove`]
/// directly. If the guard is dropped without calling `remove`, a
/// best-effort `network.removeIntercept` is spawned on the current
/// tokio runtime — errors are swallowed because this is a safety net,
/// not a primary cleanup path.
///
/// Prefer the explicit `.remove().await` form when you want errors.
#[derive(Debug)]
pub struct InterceptGuard {
    bidi: BiDi,
    intercept: Option<InterceptId>,
}

impl InterceptGuard {
    pub(crate) fn new(bidi: BiDi, intercept: InterceptId) -> Self {
        Self {
            bidi,
            intercept: Some(intercept),
        }
    }

    /// The underlying server-assigned intercept id.
    pub fn id(&self) -> &InterceptId {
        // Always Some until `remove` is called and consumes the guard.
        self.intercept.as_ref().expect("InterceptGuard already removed")
    }

    /// Remove the intercept now, returning any error from the wire call.
    /// After this, the guard's `Drop` is a no-op.
    pub async fn remove(mut self) -> Result<(), BidiError> {
        if let Some(intercept) = self.intercept.take() {
            self.bidi
                .send(RemoveIntercept {
                    intercept,
                })
                .await?;
        }
        Ok(())
    }
}

impl Drop for InterceptGuard {
    fn drop(&mut self) {
        let Some(intercept) = self.intercept.take() else {
            return;
        };
        // Best-effort: spawn the removal on the current tokio runtime.
        // If no runtime is current, the intercept stays registered for
        // the rest of the session.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let bidi = self.bidi.clone();
            handle.spawn(async move {
                let _ = bidi
                    .send(RemoveIntercept {
                        intercept,
                    })
                    .await;
            });
        }
    }
}

/// Convenience facade for the `network.*` module.
///
/// Returned by [`BiDi::network`](crate::bidi::BiDi::network). For
/// scoped variants of these commands (per-context cache mode, scoped
/// data collectors, scoped extra headers) build the command struct
/// directly and send it via [`BiDi::send`](crate::bidi::BiDi::send).
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

    /// Register a global intercept and return an [`InterceptGuard`].
    ///
    /// Wraps [`network.addIntercept`][spec]. The returned guard removes
    /// the intercept on drop (best-effort; spawned on the current tokio
    /// runtime); call [`InterceptGuard::remove`] for explicit error
    /// handling.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-addIntercept
    pub async fn add_intercept(
        &self,
        phases: Vec<InterceptPhase>,
        url_patterns: Option<Vec<serde_json::Value>>,
    ) -> Result<InterceptGuard, BidiError> {
        let result = self
            .bidi
            .send(AddIntercept {
                phases,
                contexts: vec![],
                url_patterns,
            })
            .await?;
        Ok(InterceptGuard::new(self.bidi.clone(), result.intercept))
    }

    /// Drop an intercept by id via [`network.removeIntercept`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-removeIntercept
    pub async fn remove_intercept(&self, intercept: InterceptId) -> Result<(), BidiError> {
        self.bidi
            .send(RemoveIntercept {
                intercept,
            })
            .await?;
        Ok(())
    }

    /// Release a paused request unmodified via
    /// [`network.continueRequest`][spec].
    ///
    /// To modify the request first (rewrite URL, headers, body), build a
    /// [`ContinueRequest`] struct directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-continueRequest
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

    /// Release a paused response unmodified via
    /// [`network.continueResponse`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-continueResponse
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

    /// Fail a paused request via [`network.failRequest`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-failRequest
    pub async fn fail_request(&self, request: RequestId) -> Result<(), BidiError> {
        self.bidi
            .send(FailRequest {
                request,
            })
            .await?;
        Ok(())
    }

    /// Set the global cache mode via [`network.setCacheBehavior`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-setCacheBehavior
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

    /// Register a global data collector via
    /// [`network.addDataCollector`][spec]. Pair with [`get_data`](Self::get_data).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-addDataCollector
    pub async fn add_data_collector(
        &self,
        data_types: Vec<DataType>,
        max_encoded_data_size: u64,
    ) -> Result<AddDataCollectorResult, BidiError> {
        self.bidi
            .send(AddDataCollector {
                data_types,
                max_encoded_data_size,
                collector_type: None,
                contexts: None,
                user_contexts: None,
            })
            .await
    }

    /// Drop a data collector via [`network.removeDataCollector`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-removeDataCollector
    pub async fn remove_data_collector(&self, collector: CollectorId) -> Result<(), BidiError> {
        self.bidi
            .send(RemoveDataCollector {
                collector,
            })
            .await?;
        Ok(())
    }

    /// Fetch retained body data for `request` via
    /// [`network.getData`][spec].
    ///
    /// To restrict the lookup to one collector or to disown after read,
    /// build the [`GetData`] struct directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-getData
    pub async fn get_data(
        &self,
        data_type: DataType,
        request: RequestId,
    ) -> Result<GetDataResult, BidiError> {
        self.bidi
            .send(GetData {
                data_type,
                request,
                collector: None,
                disown: None,
            })
            .await
    }

    /// Release retained data for one collector via
    /// [`network.disownData`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-disownData
    pub async fn disown_data(
        &self,
        data_type: DataType,
        collector: CollectorId,
        request: RequestId,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(DisownData {
                data_type,
                collector,
                request,
            })
            .await?;
        Ok(())
    }

    /// Apply extra request headers globally via
    /// [`network.setExtraHeaders`][spec].
    ///
    /// To scope to specific contexts/user-contexts build a
    /// [`SetExtraHeaders`] struct directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-network-setExtraHeaders
    pub async fn set_extra_headers(
        &self,
        headers: Vec<serde_json::Value>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetExtraHeaders {
                headers,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }
}
