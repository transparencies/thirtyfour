//! `log.*` BiDi module — event-only.

use serde::Deserialize;

use crate::bidi::BiDi;
use crate::bidi::command::BidiEvent;
use crate::bidi::ids::RealmId;
use crate::common::protocol::string_enum;

string_enum! {
    /// Severity for [`events::EntryAdded`].
    pub enum LogLevel {
        /// `console.debug` and friends.
        Debug = "debug",
        /// `console.info` / `console.log`.
        Info = "info",
        /// `console.warn`.
        Warn = "warn",
        /// `console.error` and uncaught exceptions.
        Error = "error",
    }
}

/// Events surfaced by the `log.*` module.
pub(crate) mod events {
    use super::*;

    /// `log.entryAdded`. Modeled as a JSON-flexible struct since the spec
    /// distinguishes `console`, `javascript`, and other entry types via the
    /// `type` field with different schemas. Common fields are typed; the
    /// rest land in [`EntryAdded::extra`].
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EntryAdded {
        /// Entry kind (`"console"`, `"javascript"`, …).
        #[serde(rename = "type")]
        pub entry_type: String,
        /// Severity.
        pub level: LogLevel,
        /// Source realm.
        #[serde(default)]
        pub source: serde_json::Value,
        /// Coalesced text.
        #[serde(default)]
        pub text: Option<String>,
        /// Driver timestamp (ms since epoch).
        pub timestamp: u64,
        /// Stack trace (if any).
        #[serde(default)]
        pub stack_trace: serde_json::Value,
        /// JS realm id (where applicable).
        #[serde(default)]
        pub realm: Option<RealmId>,
        /// Extra type-specific fields (e.g. `method` and `args` for console).
        #[serde(flatten)]
        pub extra: serde_json::Map<String, serde_json::Value>,
    }

    impl BidiEvent for EntryAdded {
        const METHOD: &'static str = "log.entryAdded";
    }
}

/// Module facade returned by [`BiDi::log`]. The `log` module is event-only
/// in the spec — there are no commands. The facade exists for symmetry with
/// the other modules.
#[derive(Debug)]
pub struct LogModule<'a> {
    #[allow(dead_code)]
    bidi: &'a BiDi,
}

impl<'a> LogModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }
}
