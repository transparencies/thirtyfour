//! `Log` domain — page console / browser log entries.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::cdp::macros::string_enum;
use crate::error::WebDriverResult;

string_enum! {
    /// Severity level reported by [`LogEntry`].
    pub enum LogLevel {
        /// Verbose / debug-level output.
        Verbose = "verbose",
        /// Informational message.
        Info = "info",
        /// Warning.
        Warning = "warning",
        /// Error.
        Error = "error",
    }
}

string_enum! {
    /// Originating subsystem reported by [`LogEntry`]. Mirrors CDP's
    /// `Log.LogEntry.source`.
    pub enum LogSource {
        /// XML parsing/processing.
        Xml = "xml",
        /// Page JavaScript runtime.
        Javascript = "javascript",
        /// Network stack.
        Network = "network",
        /// Storage subsystem.
        Storage = "storage",
        /// Application cache.
        AppCache = "appcache",
        /// Rendering engine.
        Rendering = "rendering",
        /// Security policies.
        Security = "security",
        /// Deprecation warnings.
        Deprecation = "deprecation",
        /// Web worker.
        Worker = "worker",
        /// Best-practices violation.
        Violation = "violation",
        /// Browser intervention.
        Intervention = "intervention",
        /// Recommendation.
        Recommendation = "recommendation",
        /// Anything else.
        Other = "other",
    }
}

/// `Log.enable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Enable;
impl CdpCommand for Enable {
    const METHOD: &'static str = "Log.enable";
    type Returns = Empty;
}

/// `Log.disable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Disable;
impl CdpCommand for Disable {
    const METHOD: &'static str = "Log.disable";
    type Returns = Empty;
}

/// `Log.clear`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Clear;
impl CdpCommand for Clear {
    const METHOD: &'static str = "Log.clear";
    type Returns = Empty;
}

/// One log entry, as emitted by `Log.entryAdded`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Originating subsystem.
    pub source: LogSource,
    /// Severity level.
    pub level: LogLevel,
    /// Logged text.
    pub text: String,
    /// Timestamp of the entry.
    pub timestamp: f64,
    /// URL of the resource if known.
    pub url: Option<String>,
    /// Line number of the entry in the resource.
    pub line_number: Option<u32>,
}

/// `Log.entryAdded` event.
#[derive(Debug, Clone, Deserialize)]
pub struct EntryAdded {
    /// The log entry.
    pub entry: LogEntry,
}
impl CdpEvent for EntryAdded {
    const METHOD: &'static str = "Log.entryAdded";
}

/// Domain facade returned by [`Cdp::log`].
#[derive(Debug)]
pub struct LogDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> LogDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Log.enable`.
    pub async fn enable(&self) -> WebDriverResult<()> {
        self.cdp.send(Enable).await?;
        Ok(())
    }

    /// `Log.disable`.
    pub async fn disable(&self) -> WebDriverResult<()> {
        self.cdp.send(Disable).await?;
        Ok(())
    }

    /// `Log.clear`.
    pub async fn clear(&self) -> WebDriverResult<()> {
        self.cdp.send(Clear).await?;
        Ok(())
    }
}
