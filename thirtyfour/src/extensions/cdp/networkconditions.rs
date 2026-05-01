use serde::{Deserialize, Serialize};

// Note: this module wraps chromedriver's `/session/{id}/chromium/network_conditions`
// vendor endpoint, which is **snake_case** on the wire (not the CDP camelCase).
// The original code had a `#[serde(rename = "camelCase")]` attribute on the
// struct (and `rename = "lowercase"` on the enum) — both are no-ops because
// they're attribute renames, not field/variant renames. The wire format has
// always been the default (snake_case for fields, capitalised variants).
// Don't add `rename_all = "camelCase"` here — it would silently break the
// vendor endpoint. For CDP `Network.emulateNetworkConditions` (camelCase),
// use [`crate::cdp::domains::network::NetworkConditions`] instead.

/// Connection type used by the chromedriver `network_conditions` vendor
/// endpoint.
///
/// **Deprecated.** Use [`crate::cdp::domains::network::ConnectionType`] with
/// the typed CDP `Network.emulateNetworkConditions` command instead.
#[deprecated(since = "0.37.0", note = "use thirtyfour::cdp::domains::network::ConnectionType")]
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConnectionType {
    /// No connection.
    None,
    /// 2G cellular.
    Cellular2G,
    /// 3G cellular.
    Cellular3G,
    /// 4G cellular.
    Cellular4G,
    /// Bluetooth.
    Bluetooth,
    /// Ethernet.
    Ethernet,
    /// WiFi.
    Wifi,
    /// WiMAX.
    Wimax,
    /// Other.
    Other,
}

/// Simulated network conditions for the chromedriver vendor endpoint
/// `/session/{id}/chromium/network_conditions`.
///
/// **Deprecated.** Use [`crate::cdp::domains::network::NetworkConditions`]
/// with [`crate::cdp::Cdp::network`] instead — that calls
/// `Network.emulateNetworkConditions` over the standard CDP path and works
/// the same way on Chrome, Edge, Brave, and Opera.
#[deprecated(
    since = "0.37.0",
    note = "use thirtyfour::cdp::domains::network::NetworkConditions via WebDriver::cdp().network()"
)]
#[allow(deprecated)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkConditions {
    /// True to simulate the network being offline.
    pub offline: bool,
    /// The latency to add, in milliseconds.
    pub latency: u32,
    /// The download throughput, in bytes/second. `-1` disables download throttling.
    pub download_throughput: i32,
    /// The upload throughput, in bytes/second. `-1` disables upload throttling.
    pub upload_throughput: i32,
    /// The connection type, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
}

#[allow(deprecated)]
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

#[allow(deprecated)]
impl NetworkConditions {
    /// Create a new `NetworkConditions` instance.
    pub fn new() -> Self {
        Self::default()
    }
}
