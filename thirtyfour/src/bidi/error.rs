//! BiDi-specific error type.
//!
//! Failed BiDi commands return a JSON envelope of the form
//! `{"type":"error","id":N,"error":"<code>","message":"...","stacktrace":"..."}`.
//! [`BidiError`] is the typed view of that envelope.

use serde::Deserialize;
use std::fmt;

/// A BiDi command error.
///
/// The `error` field is one of the W3C BiDi-defined error codes (e.g.
/// `"unknown command"`, `"no such frame"`, `"invalid argument"`); kept as a
/// `String` because the spec adds new codes over time.
#[derive(Debug, Clone, thiserror::Error)]
pub struct BidiError {
    /// The BiDi method that failed (e.g. `"browsingContext.navigate"`).
    pub command: String,
    /// W3C BiDi error code (`"invalid argument"`, `"no such frame"`, …).
    pub error: String,
    /// Human-readable message from the driver.
    pub message: String,
    /// Optional stack trace from the driver.
    pub stacktrace: Option<String>,
}

impl BidiError {
    /// True if this error indicates the BiDi session ended (e.g. browser
    /// closed). Detected by the W3C error code.
    pub fn is_session_ended(&self) -> bool {
        matches!(
            self.error.as_str(),
            "invalid session id" | "session not created" | "unable to close browser"
        )
    }

    /// True if this error indicates the target referenced by the command
    /// (browsing context, frame, element node, request, …) does not exist.
    pub fn is_no_such(&self) -> bool {
        self.error.starts_with("no such")
    }
}

impl fmt::Display for BidiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BiDi command {} failed: {} — {}", self.command, self.error, self.message)
    }
}

impl From<BidiError> for crate::error::WebDriverError {
    fn from(e: BidiError) -> Self {
        // BiDi errors carry richer information than the W3C HTTP error
        // shape, so we render them as `FatalError` strings rather than
        // pretending to be a typed W3C error code (would require synthetic
        // `WebDriverErrorInfo`s). Users who want the typed BiDi shape
        // should call `BiDi::send` directly and get back `BidiError`.
        crate::error::WebDriverError::FatalError(e.to_string())
    }
}

/// Wire shape of an error envelope as it appears on the WebSocket. Used by
/// the transport when parsing responses.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BidiErrorEnvelope {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub stacktrace: Option<String>,
}

impl BidiErrorEnvelope {
    pub(crate) fn into_error(self, command: impl Into<String>) -> BidiError {
        BidiError {
            command: command.into(),
            error: self.error,
            message: self.message,
            stacktrace: self.stacktrace,
        }
    }
}
