//! `Network` domain — network observation, headers, cookies, throttling.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::cdp::ids::{LoaderId, RequestId};
use crate::cdp::macros::string_enum;
use crate::error::WebDriverResult;

string_enum! {
    /// Connection type for network throttling
    /// (`Network.ConnectionType`).
    pub enum ConnectionType {
        /// No connection.
        None = "none",
        /// 2G cellular.
        Cellular2G = "cellular2g",
        /// 3G cellular.
        Cellular3G = "cellular3g",
        /// 4G cellular.
        Cellular4G = "cellular4g",
        /// Bluetooth.
        Bluetooth = "bluetooth",
        /// Ethernet.
        Ethernet = "ethernet",
        /// WiFi.
        Wifi = "wifi",
        /// WiMAX.
        Wimax = "wimax",
        /// Other.
        Other = "other",
    }
}

string_enum! {
    /// Reason a network request failed (`Network.ErrorReason`). Used by
    /// [`crate::cdp::domains::fetch::FailRequest`] and seen on
    /// [`LoadingFailed`] / [`crate::cdp::domains::fetch::RequestPaused`]
    /// events.
    pub enum ErrorReason {
        /// Generic failure.
        Failed = "Failed",
        /// Request was aborted (e.g. user navigation).
        Aborted = "Aborted",
        /// Request timed out.
        TimedOut = "TimedOut",
        /// Access was denied (e.g. CORS).
        AccessDenied = "AccessDenied",
        /// Connection was closed.
        ConnectionClosed = "ConnectionClosed",
        /// Connection was reset.
        ConnectionReset = "ConnectionReset",
        /// Connection was refused.
        ConnectionRefused = "ConnectionRefused",
        /// Connection was aborted.
        ConnectionAborted = "ConnectionAborted",
        /// Connection failed for another reason.
        ConnectionFailed = "ConnectionFailed",
        /// DNS resolution failed.
        NameNotResolved = "NameNotResolved",
        /// Browser is offline.
        InternetDisconnected = "InternetDisconnected",
        /// Address could not be reached.
        AddressUnreachable = "AddressUnreachable",
        /// Blocked by client (e.g. extension).
        BlockedByClient = "BlockedByClient",
        /// Blocked by server response (e.g. CSP).
        BlockedByResponse = "BlockedByResponse",
    }
}

string_enum! {
    /// Resource type classification used by `Network` and `Fetch` events
    /// (`Network.ResourceType`).
    pub enum ResourceType {
        /// HTML document.
        Document = "Document",
        /// CSS stylesheet.
        Stylesheet = "Stylesheet",
        /// Image (raster or SVG).
        Image = "Image",
        /// Audio or video.
        Media = "Media",
        /// Web font.
        Font = "Font",
        /// JavaScript script.
        Script = "Script",
        /// `<track>` text track.
        TextTrack = "TextTrack",
        /// `XMLHttpRequest`.
        Xhr = "XHR",
        /// `fetch()` API.
        Fetch = "Fetch",
        /// `<link rel="prefetch">`.
        Prefetch = "Prefetch",
        /// Server-Sent Events.
        EventSource = "EventSource",
        /// WebSocket.
        WebSocket = "WebSocket",
        /// Web app manifest.
        Manifest = "Manifest",
        /// Signed Exchange.
        SignedExchange = "SignedExchange",
        /// `navigator.sendBeacon` ping.
        Ping = "Ping",
        /// CSP violation report.
        CspViolationReport = "CSPViolationReport",
        /// CORS preflight.
        Preflight = "Preflight",
        /// Anything else.
        Other = "Other",
    }
}

/// Simulated network conditions for `Network.emulateNetworkConditions`.
///
/// See <https://chromedevtools.github.io/devtools-protocol/tot/Network/#method-emulateNetworkConditions>.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConditions {
    /// True to emulate the network being offline.
    pub offline: bool,
    /// Latency to add (milliseconds).
    pub latency: u32,
    /// Download throughput, bytes/second. `-1` disables download throttling.
    pub download_throughput: i32,
    /// Upload throughput, bytes/second. `-1` disables upload throttling.
    pub upload_throughput: i32,
    /// Connection type, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self {
            offline: false,
            latency: 0,
            download_throughput: -1,
            upload_throughput: -1,
            connection_type: None,
        }
    }
}

impl NetworkConditions {
    /// Construct an instance with throttling disabled.
    pub fn new() -> Self {
        Self::default()
    }
}

/// `Network.enable`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enable {
    /// Buffer size in bytes for resource bodies (default 0 — disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_buffer_size: Option<i64>,
    /// Per-resource max buffer size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_resource_buffer_size: Option<i64>,
}
impl CdpCommand for Enable {
    const METHOD: &'static str = "Network.enable";
    type Returns = Empty;
}

/// `Network.disable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Disable;
impl CdpCommand for Disable {
    const METHOD: &'static str = "Network.disable";
    type Returns = Empty;
}

/// `Network.clearBrowserCache`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ClearBrowserCache;
impl CdpCommand for ClearBrowserCache {
    const METHOD: &'static str = "Network.clearBrowserCache";
    type Returns = Empty;
}

/// `Network.clearBrowserCookies`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ClearBrowserCookies;
impl CdpCommand for ClearBrowserCookies {
    const METHOD: &'static str = "Network.clearBrowserCookies";
    type Returns = Empty;
}

/// `Network.setExtraHTTPHeaders`.
#[derive(Debug, Clone, Serialize)]
pub struct SetExtraHttpHeaders {
    /// Map of header name to value. Wire field is `headers`.
    pub headers: HashMap<String, String>,
}
impl CdpCommand for SetExtraHttpHeaders {
    const METHOD: &'static str = "Network.setExtraHTTPHeaders";
    type Returns = Empty;
}

/// `Network.setUserAgentOverride`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserAgentOverride {
    /// User agent string to use.
    pub user_agent: String,
    /// Browser language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_language: Option<String>,
    /// Platform string (e.g. `"Linux x86_64"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}
impl CdpCommand for SetUserAgentOverride {
    const METHOD: &'static str = "Network.setUserAgentOverride";
    type Returns = Empty;
}

/// `Network.emulateNetworkConditions`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulateNetworkConditions {
    /// True to emulate the network being offline.
    pub offline: bool,
    /// Latency to add (milliseconds).
    pub latency: u32,
    /// Download throughput, bytes/second. `-1` disables.
    pub download_throughput: i32,
    /// Upload throughput, bytes/second. `-1` disables.
    pub upload_throughput: i32,
    /// Connection type if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
}

impl From<NetworkConditions> for EmulateNetworkConditions {
    fn from(c: NetworkConditions) -> Self {
        Self {
            offline: c.offline,
            latency: c.latency,
            download_throughput: c.download_throughput,
            upload_throughput: c.upload_throughput,
            connection_type: c.connection_type,
        }
    }
}

impl CdpCommand for EmulateNetworkConditions {
    const METHOD: &'static str = "Network.emulateNetworkConditions";
    type Returns = Empty;
}

/// `Network.getResponseBody`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseBody {
    /// Identifier of the network request.
    pub request_id: RequestId,
}

/// Response for [`GetResponseBody`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    /// Response body.
    pub body: String,
    /// True if `body` is base64-encoded.
    pub base64_encoded: bool,
}

impl CdpCommand for GetResponseBody {
    const METHOD: &'static str = "Network.getResponseBody";
    type Returns = ResponseBody;
}

/// `Network.requestWillBeSent` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestWillBeSent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Loader identifier.
    pub loader_id: LoaderId,
    /// URL of the document this request is loaded for. CDP spells this
    /// `documentURL` (capital `URL`), not the camelCase you'd expect.
    #[serde(rename = "documentURL")]
    pub document_url: String,
    /// Request data — full structure documented at
    /// <https://chromedevtools.github.io/devtools-protocol/tot/Network/#type-Request>.
    pub request: serde_json::Value,
    /// Timestamp of the event.
    pub timestamp: f64,
    /// Wall-clock time of the event.
    pub wall_time: f64,
    /// Initiator info.
    pub initiator: serde_json::Value,
}
impl CdpEvent for RequestWillBeSent {
    const METHOD: &'static str = "Network.requestWillBeSent";
}

/// `Network.responseReceived` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseReceived {
    /// Request identifier.
    pub request_id: RequestId,
    /// Loader identifier.
    pub loader_id: LoaderId,
    /// Timestamp.
    pub timestamp: f64,
    /// Resource type (e.g. `Document`, `Xhr`, `Image`).
    pub r#type: ResourceType,
    /// Full response details (status, headers, mime type, etc.).
    pub response: serde_json::Value,
}
impl CdpEvent for ResponseReceived {
    const METHOD: &'static str = "Network.responseReceived";
}

/// `Network.loadingFinished` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadingFinished {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: f64,
    /// Total number of bytes received.
    pub encoded_data_length: f64,
}
impl CdpEvent for LoadingFinished {
    const METHOD: &'static str = "Network.loadingFinished";
}

/// `Network.loadingFailed` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadingFailed {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: f64,
    /// Resource type.
    pub r#type: ResourceType,
    /// User-friendly error message.
    pub error_text: String,
    /// Whether the loading was canceled.
    pub canceled: Option<bool>,
}
impl CdpEvent for LoadingFailed {
    const METHOD: &'static str = "Network.loadingFailed";
}

/// Domain facade returned by [`Cdp::network`].
#[derive(Debug)]
pub struct NetworkDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> NetworkDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Network.enable` (default buffer sizes).
    pub async fn enable(&self) -> WebDriverResult<()> {
        self.cdp.send(Enable::default()).await?;
        Ok(())
    }

    /// `Network.disable`.
    pub async fn disable(&self) -> WebDriverResult<()> {
        self.cdp.send(Disable).await?;
        Ok(())
    }

    /// `Network.clearBrowserCache`.
    pub async fn clear_browser_cache(&self) -> WebDriverResult<()> {
        self.cdp.send(ClearBrowserCache).await?;
        Ok(())
    }

    /// `Network.clearBrowserCookies`.
    pub async fn clear_browser_cookies(&self) -> WebDriverResult<()> {
        self.cdp.send(ClearBrowserCookies).await?;
        Ok(())
    }

    /// `Network.setExtraHTTPHeaders`.
    pub async fn set_extra_http_headers(
        &self,
        headers: HashMap<String, String>,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(SetExtraHttpHeaders {
                headers,
            })
            .await?;
        Ok(())
    }

    /// `Network.setUserAgentOverride`.
    pub async fn set_user_agent_override(
        &self,
        user_agent: impl Into<String>,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(SetUserAgentOverride {
                user_agent: user_agent.into(),
                accept_language: None,
                platform: None,
            })
            .await?;
        Ok(())
    }

    /// `Network.emulateNetworkConditions` from a [`NetworkConditions`].
    pub async fn emulate_network_conditions(
        &self,
        conditions: NetworkConditions,
    ) -> WebDriverResult<()> {
        self.cdp.send(EmulateNetworkConditions::from(conditions)).await?;
        Ok(())
    }

    /// `Network.getResponseBody`.
    pub async fn get_response_body(
        &self,
        request_id: impl Into<RequestId>,
    ) -> WebDriverResult<ResponseBody> {
        self.cdp
            .send(GetResponseBody {
                request_id: request_id.into(),
            })
            .await
    }
}
