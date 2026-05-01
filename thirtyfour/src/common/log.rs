//! Browser log entries returned by the legacy `/log` and `/log/types`
//! WebDriver endpoints, plus the [`LoggingPrefsLogLevel`][lpl] capability
//! enum shared between browsers.
//!
//! [lpl]: crate::LoggingPrefsLogLevel
//!
//! These endpoints (`GET /session/{id}/log/types`,
//! `POST /session/{id}/log`) are not part of the W3C WebDriver spec, but
//! `chromedriver` (and other Chromium-based drivers) still implement them.
//! `geckodriver` does not.
//!
//! To make Chromium emit browser log entries you must enable logging via
//! capabilities at session creation. See
//! [`ChromiumLikeCapabilities::set_logging_prefs`][slp] and
//! [`ChromiumLikeCapabilities::set_browser_log_level`][sblp].
//!
//! [slp]: crate::ChromiumLikeCapabilities::set_logging_prefs
//! [sblp]: crate::ChromiumLikeCapabilities::set_browser_log_level

use serde::{Deserialize, Serialize};

/// One entry returned by the legacy `POST /session/{id}/log` endpoint.
///
/// `chromedriver` returns these when `goog:loggingPrefs` requests the
/// matching log type at session creation. The shape is the legacy Selenium
/// "log entry" object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserLogEntry {
    /// Severity level (e.g. `"INFO"`, `"WARNING"`, `"SEVERE"`, `"DEBUG"`,
    /// `"FINE"`). Drivers emit the SCREAMING form rather than the lowercase
    /// CDP form.
    pub level: String,
    /// Log message text. For `console.*` calls chromedriver formats this as
    /// `<source-url> <line> <column> <serialized message>`.
    pub message: String,
    /// Timestamp in milliseconds since the Unix epoch.
    pub timestamp: i64,
    /// Optional source field (e.g. `"console-api"`, `"network"`,
    /// `"javascript"`). Not all drivers populate it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Selenium logging preference level (capability enum).
///
/// Used as the value side of an entry in `goog:loggingPrefs` (Chromium) or
/// `loggingPrefs` (Firefox/Selenium-grid). The variants serialise to the
/// SCREAMING form Selenium expects (e.g. `Severe` → `"SEVERE"`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LoggingPrefsLogLevel {
    /// Disable logging.
    Off,
    /// Severe errors only.
    Severe,
    /// Warnings and above.
    Warning,
    /// Informational and above.
    Info,
    /// Configuration messages and above.
    Config,
    /// Fine-grained debug and above.
    Fine,
    /// Finer-grained debug and above.
    Finer,
    /// Finest-grained debug and above.
    Finest,
    /// All logs.
    All,
}
