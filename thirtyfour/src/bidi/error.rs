//! BiDi-specific error type.
//!
//! Failed BiDi commands respond with a JSON envelope of the form
//! `{"type":"error","id":N,"error":"<code>","message":"...","stacktrace":"..."}`.
//! [`BidiError`] is the typed view of that envelope.
//!
//! The full list of [W3C BiDi error codes][spec] is open-ended — the
//! spec continues to add codes over time, and drivers can return
//! values not yet documented. The `error` field is therefore a `String`
//! rather than an enum.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#errors

use serde::Deserialize;
use std::fmt;

/// A failed BiDi command.
///
/// The `error` field is a [W3C BiDi error code][spec] string
/// (`"unknown command"`, `"no such frame"`, `"invalid argument"`,
/// `"unsupported operation"`, …). Helpers on this type
/// ([`is_session_ended`](Self::is_session_ended),
/// [`is_no_such`](Self::is_no_such)) classify common categories.
///
/// `BidiError` implements `From<BidiError> for WebDriverError`, so it
/// composes with code that uses `WebDriverResult<T>` — but you'll
/// generally get more useful information from the typed shape directly.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#errors
#[derive(Debug, Clone, thiserror::Error)]
pub struct BidiError {
    /// The BiDi method that failed (e.g. `"browsingContext.navigate"`).
    pub command: String,
    /// W3C BiDi error code — `"invalid argument"`, `"no such frame"`,
    /// `"unsupported operation"`, etc.
    pub error: String,
    /// Human-readable message from the driver.
    pub message: String,
    /// Optional stack trace from the driver. When present, it points
    /// into driver-internal code, not the page under test.
    pub stacktrace: Option<String>,
}

impl BidiError {
    /// True if this error indicates the underlying WebDriver session
    /// has ended (typically because the browser process exited).
    ///
    /// Detected from the error code; specifically matches
    /// `"invalid session id"`, `"session not created"`, and
    /// `"unable to close browser"`.
    pub fn is_session_ended(&self) -> bool {
        matches!(
            self.error.as_str(),
            "invalid session id" | "session not created" | "unable to close browser"
        )
    }

    /// True if this error indicates that the target referenced by the
    /// command (browsing context, frame, element node, request,
    /// intercept, …) does not exist.
    ///
    /// Implemented as a prefix match on `"no such"`, which covers every
    /// `no such X` error code defined by the spec.
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
        // `WebDriverErrorInfo`s). Callers who want the typed BiDi shape
        // should use `BiDi::send` directly and match on `BidiError`.
        crate::error::WebDriverError::FatalError(e.to_string())
    }
}

/// Wire shape of an error envelope as it appears on the WebSocket.
/// Used by the transport when parsing responses.
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
