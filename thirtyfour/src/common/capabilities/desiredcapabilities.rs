use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, json, to_value};

use crate::ChromiumCapabilities;
use crate::common::capabilities::chrome::ChromeCapabilities;
use crate::common::capabilities::edge::EdgeCapabilities;
use crate::common::capabilities::firefox::FirefoxCapabilities;
use crate::common::capabilities::ie::InternetExplorerCapabilities;
use crate::common::capabilities::opera::OperaCapabilities;
use crate::common::capabilities::safari::SafariCapabilities;
use crate::error::WebDriverResult;

/// W3C-compatible WebDriver capabilities.
///
/// This is a thin wrapper over a JSON object. Construct one directly via
/// [`Capabilities::new`] for fully custom configuration, or — more commonly
/// — use one of the browser-specific helpers under [`DesiredCapabilities`]
/// (`DesiredCapabilities::chrome()`, `firefox()`, etc.) which preconfigure
/// `browserName` and expose typed setters for vendor-specific options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Capabilities(serde_json::Map<String, Value>);

impl Capabilities {
    /// Create an empty `Capabilities`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a reference to the value for the given capability key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Get a mutable reference to the value for the given capability key.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.0.get_mut(key)
    }

    /// Set a capability by serialising the given value.
    pub fn set<T>(&mut self, key: impl Into<String>, value: T) -> WebDriverResult<()>
    where
        T: Serialize,
    {
        self.0.insert(key.into(), to_value(value)?);
        Ok(())
    }

    /// Remove the capability with the given key. Returns the removed value,
    /// if any.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.remove(key)
    }

    /// Returns true if the capability map contains the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Number of capabilities in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no capabilities set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over `(key, value)` pairs.
    pub fn iter(&self) -> serde_json::map::Iter<'_> {
        self.0.iter()
    }
}

impl AsRef<Capabilities> for Capabilities {
    fn as_ref(&self) -> &Capabilities {
        self
    }
}

impl AsMut<Capabilities> for Capabilities {
    fn as_mut(&mut self) -> &mut Capabilities {
        self
    }
}

impl From<Capabilities> for Value {
    fn from(caps: Capabilities) -> Value {
        Value::Object(caps.0)
    }
}

impl From<serde_json::Map<String, Value>> for Capabilities {
    fn from(map: serde_json::Map<String, Value>) -> Self {
        Self(map)
    }
}

const W3C_CAPABILITY_NAMES: &[&str] = &[
    "acceptInsecureCerts",
    "browserName",
    "browserVersion",
    "platformName",
    "pageLoadStrategy",
    "proxy",
    "setWindowRect",
    "timeouts",
    "unhandledPromptBehavior",
    "strictFileInteractability",
];

const OSS_W3C_CONVERSION: &[(&str, &str)] = &[
    ("acceptSslCerts", "acceptInsecureCerts"),
    ("version", "browserVersion"),
    ("platform", "platformName"),
];

/// Convert the given serde_json::Value into a W3C-compatible Capabilities struct.
pub fn make_w3c_caps(caps: &Value) -> Value {
    let mut always_match = json!({});

    if let Some(caps_map) = caps.as_object() {
        for (k, v) in caps_map.iter() {
            if !v.is_null() {
                for (k_from, k_to) in OSS_W3C_CONVERSION {
                    if k_from == k {
                        always_match[k_to] = v.clone();
                    }
                }
            }

            if W3C_CAPABILITY_NAMES.contains(&k.as_str()) || k.contains(':') {
                always_match[k] = v.clone();
            }
        }
    }

    json!({
        "firstMatch": [{}], "alwaysMatch": always_match
    })
}

/// Provides static methods for constructing browser-specific capabilities.
///
/// ## Example
/// ```no_run
/// use thirtyfour::{DesiredCapabilities, WebDriver};
/// let caps = DesiredCapabilities::chrome();
/// let driver = WebDriver::new("http://localhost:4444", caps);
/// ```
#[derive(Debug)]
pub struct DesiredCapabilities;

impl DesiredCapabilities {
    /// Create a ChromeCapabilities struct.
    pub fn chrome() -> ChromeCapabilities {
        ChromeCapabilities::new()
    }

    /// Create a ChromiumCapabilities struct.
    pub fn chromium() -> ChromiumCapabilities {
        ChromiumCapabilities::new()
    }

    /// Create an EdgeCapabilities struct.
    pub fn edge() -> EdgeCapabilities {
        EdgeCapabilities::new()
    }

    /// Create a FirefoxCapabilities struct.
    pub fn firefox() -> FirefoxCapabilities {
        FirefoxCapabilities::new()
    }

    /// Create an InternetExplorerCapabilities struct.
    pub fn internet_explorer() -> InternetExplorerCapabilities {
        InternetExplorerCapabilities::new()
    }

    /// Create an OperaCapabilities struct.
    pub fn opera() -> OperaCapabilities {
        OperaCapabilities::new()
    }

    /// Create a SafariCapabilities struct.
    pub fn safari() -> SafariCapabilities {
        SafariCapabilities::new()
    }
}

/// Common capability accessors shared by all browser-specific capability
/// types.
///
/// Implemented automatically for any type that exposes a [`Capabilities`]
/// via [`AsRef`] / [`AsMut`]. Browser-specific capabilities (e.g.
/// [`ChromeCapabilities`]) get this trait for free, so calls like
/// `caps.set("foo", "bar")?` and `caps.get("foo")` work uniformly across
/// [`Capabilities`] and the browser-specific wrappers.
pub trait CapabilitiesHelper: AsRef<Capabilities> + AsMut<Capabilities> {
    /// Get a reference to the value for the given capability key.
    fn get(&self, key: &str) -> Option<&Value> {
        self.as_ref().get(key)
    }

    /// Get a mutable reference to the value for the given capability key.
    fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.as_mut().get_mut(key)
    }

    /// Set a capability by serialising the given value.
    fn set<T>(&mut self, key: impl Into<String>, value: T) -> WebDriverResult<()>
    where
        T: Serialize,
    {
        self.as_mut().set(key, value)
    }

    /// Set the desired browser version.
    fn set_version(&mut self, version: &str) -> WebDriverResult<()> {
        self.set("version", version)
    }

    /// Set the desired browser platform.
    fn set_platform(&mut self, platform: &str) -> WebDriverResult<()> {
        self.set("platform", platform)
    }

    /// Set whether the session supports executing user-supplied Javascript.
    fn set_javascript_enabled(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("javascriptEnabled", enabled)
    }

    /// Set whether the session can interact with database storage.
    fn set_database_enabled(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("databaseEnabled", enabled)
    }

    /// Set whether the session can set and query the browser's location context.
    fn set_location_context_enabled(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("locationContextEnabled", enabled)
    }

    /// Set whether the session can interact with the application cache.
    fn set_application_cache_enabled(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("applicationCacheEnabled", enabled)
    }

    /// Set whether the session can query for the browser's connectivity and disable it if desired.
    fn set_browser_connection_enabled(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("browserConnectionEnabled", enabled)
    }

    /// Set whether the session supports interactions with local storage.
    fn set_web_storage_enabled(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("webStorageEnabled", enabled)
    }

    /// Set whether the session should accept insecure SSL certificates by default.
    fn accept_insecure_certs(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("acceptInsecureCerts", enabled)
    }

    /// Set whether the session can rotate the current page's layout between portrait and landscape
    /// orientations. Only applies to mobile platforms.
    fn set_rotatable(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("rotatable", enabled)
    }

    /// Set whether the session is capable of generating native events when simulating user input.
    fn set_native_events(&mut self, enabled: bool) -> WebDriverResult<()> {
        self.set("nativeEvents", enabled)
    }

    /// Set the proxy to use.
    fn set_proxy(&mut self, proxy: Proxy) -> WebDriverResult<()> {
        self.set("proxy", proxy)
    }

    /// Set the behaviour to be followed when an unexpected alert is encountered.
    fn set_unexpected_alert_behaviour(&mut self, behaviour: AlertBehaviour) -> WebDriverResult<()> {
        self.set("unexpectedAlertBehaviour", behaviour)
    }

    /// Set whether elements are scrolled into the viewport for interation to align with the top
    /// or the bottom of the viewport. The default is to align with the top.
    fn set_element_scroll_behaviour(&mut self, behaviour: ScrollBehaviour) -> WebDriverResult<()> {
        self.set("elementScrollBehavior", behaviour)
    }

    /// Get whether the session can interact with modal popups such as `window.alert`.
    fn handles_alerts(&self) -> Option<bool> {
        self.as_ref().get("handlesAlerts").and_then(|x| x.as_bool())
    }

    /// Get whether the session supports CSS selectors when searching for elements.
    fn css_selectors_enabled(&self) -> Option<bool> {
        self.as_ref().get("cssSelectorsEnabled").and_then(|x| x.as_bool())
    }

    /// Get the current page load strategy.
    fn page_load_strategy(&self) -> WebDriverResult<PageLoadStrategy> {
        let strategy =
            self.as_ref().get("pageLoadStrategy").map(|x| from_value(x.clone())).transpose()?;
        Ok(strategy.unwrap_or_default())
    }

    /// Set the page load strategy to use.
    fn set_page_load_strategy(&mut self, strategy: PageLoadStrategy) -> WebDriverResult<()> {
        self.set("pageLoadStrategy", strategy)
    }
}

impl<T: AsRef<Capabilities> + AsMut<Capabilities>> CapabilitiesHelper for T {}

/// Helper trait for adding browser-specific capabilities.
///
/// For example, chrome stores capabilities under `goog:chromeOptions` and firefox
/// stores capabilities under `moz:firefoxOptions`.
pub trait BrowserCapabilitiesHelper: CapabilitiesHelper {
    /// The key containing the browser-specific capabilities.
    const KEY: &'static str;

    /// Add any Serialize-able object to the capabilities under the browser's custom key.
    fn set_browser_option(
        &mut self,
        key: impl Into<String>,
        value: impl Serialize,
    ) -> WebDriverResult<()> {
        let key = key.into();
        let value = to_value(value)?;
        match self.as_mut().get_mut(Self::KEY) {
            Some(Value::Object(v)) => {
                v.insert(key, value);
            }
            _ => {
                self.as_mut().set(Self::KEY, json!({ key: value }))?;
            }
        }
        Ok(())
    }

    /// Remove the custom browser-specific property if it exists.
    fn remove_browser_option(&mut self, key: &str) {
        if let Some(Value::Object(v)) = self.as_mut().get_mut(Self::KEY) {
            v.remove(key);
        }
    }

    /// Get the custom browser-specific property if it exists.
    fn browser_option<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.as_ref()
            .get(Self::KEY)
            .and_then(|options| options.get(key))
            .and_then(|option| from_value(option.clone()).ok())
    }

    /// Get the current list of command-line arguments as a vec.
    fn args(&self) -> Vec<String> {
        self.browser_option("args").unwrap_or_default()
    }

    /// Remove the specified command-line argument if it had been added previously.
    fn remove_arg(&mut self, arg: &str) -> WebDriverResult<()> {
        let mut args = self.args();
        if args.is_empty() {
            Ok(())
        } else {
            args.retain(|v| v != arg);
            self.set_browser_option("args", to_value(args)?)
        }
    }

    /// Return true if the specified arg is currently set.
    fn has_arg(&self, arg: &str) -> bool {
        self.args().iter().any(|s| s == arg)
    }
}

/// Proxy configuration settings.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "proxyType", rename_all = "lowercase")]
pub enum Proxy {
    /// Direct connection to the webdriver server.
    Direct,
    /// Manual proxy configuration.
    #[serde(rename_all = "camelCase")]
    Manual {
        /// FTP proxy.
        #[serde(skip_serializing_if = "Option::is_none")]
        ftp_proxy: Option<String>,
        /// HTTP proxy.
        #[serde(skip_serializing_if = "Option::is_none")]
        http_proxy: Option<String>,
        /// SSL proxy.
        #[serde(skip_serializing_if = "Option::is_none")]
        ssl_proxy: Option<String>,
        /// SOCKS proxy.
        #[serde(skip_serializing_if = "Option::is_none")]
        socks_proxy: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// The SOCKS version.
        socks_version: Option<u8>,
        /// SOCKS username.
        #[serde(skip_serializing_if = "Option::is_none")]
        socks_username: Option<String>,
        /// SOCKS password.
        #[serde(skip_serializing_if = "Option::is_none")]
        socks_password: Option<String>,
        /// Urls to skip the proxy.
        #[serde(skip_serializing_if = "Option::is_none")]
        no_proxy: Option<Vec<String>>,
    },
    /// Autoconfiguration url.
    #[serde(rename = "pac")]
    AutoConfig {
        /// The autoconfiguration url.
        #[serde(rename = "proxyAutoconfigUrl")]
        url: String,
    },
    /// Auto-detect proxy.
    AutoDetect,
    /// Use the system proxy configuration.
    System,
}

/// The action to take when an alert is encountered.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertBehaviour {
    /// Automatically accept the alert.
    Accept,
    /// Automatically dismiss the alert.
    Dismiss,
    /// Ignore the alert.
    Ignore,
}

/// The automatic scrolling behaviour for this session.
#[derive(Debug, Clone, Serialize)]
#[repr(u8)]
pub enum ScrollBehaviour {
    /// Scroll until the element is at the top of the screen, if possible.
    Top = 0,
    /// Scroll until the element is at the bottom of the screen, if possible.
    Bottom = 1,
}

/// The page load strategy for this session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageLoadStrategy {
    /// Wait for full page loading (the default).
    #[default]
    Normal,
    /// Wait for the DOMContentLoaded event (html content downloaded and parsed only).
    Eager,
    /// Return immediately after the initial page content is fully received
    /// (html content downloaded).
    None,
}
