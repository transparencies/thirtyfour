//! `Log` domain — page console / browser log entries.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::error::WebDriverResult;

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
    /// Source: `"xml"`, `"javascript"`, `"network"`, `"storage"`, `"appcache"`,
    /// `"rendering"`, `"security"`, `"deprecation"`, `"worker"`, `"violation"`,
    /// `"intervention"`, `"recommendation"`, `"other"`.
    pub source: String,
    /// `"verbose"`, `"info"`, `"warning"`, or `"error"`.
    pub level: String,
    /// Logged text.
    pub text: String,
    /// Timestamp of the entry.
    pub timestamp: f64,
    /// URL of the resource if known.
    #[serde(default)]
    pub url: Option<String>,
    /// Line number of the entry in the resource.
    #[serde(default)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn methods() {
        assert_eq!(Enable::METHOD, "Log.enable");
        assert_eq!(Disable::METHOD, "Log.disable");
        assert_eq!(Clear::METHOD, "Log.clear");
    }

    #[test]
    fn event_method() {
        assert_eq!(EntryAdded::METHOD, "Log.entryAdded");
    }

    #[test]
    fn entry_added_minimal() {
        let body = json!({
            "entry": {
                "source": "javascript",
                "level": "error",
                "text": "Uncaught TypeError",
                "timestamp": 1.0
            }
        });
        let evt: EntryAdded = serde_json::from_value(body).unwrap();
        assert_eq!(evt.entry.source, "javascript");
        assert_eq!(evt.entry.level, "error");
        assert_eq!(evt.entry.text, "Uncaught TypeError");
        assert!(evt.entry.url.is_none());
        assert!(evt.entry.line_number.is_none());
    }

    #[test]
    fn entry_added_full() {
        let body = json!({
            "entry": {
                "source": "network",
                "level": "warning",
                "text": "Slow response",
                "timestamp": 100.0,
                "url": "https://x",
                "lineNumber": 42
            }
        });
        let evt: EntryAdded = serde_json::from_value(body).unwrap();
        assert_eq!(evt.entry.url.as_deref(), Some("https://x"));
        assert_eq!(evt.entry.line_number, Some(42));
    }
}
