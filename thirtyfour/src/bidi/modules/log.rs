//! `log.*` — JavaScript console output and uncaught-exception observation.
//!
//! The log module is event-only: there are no commands. Subscribe to
//! [`LogEntryAdded`](crate::bidi::events::LogEntryAdded) via
//! [`BiDi::subscribe`](crate::bidi::BiDi::subscribe) to receive every
//! `console.*` call and every uncaught JavaScript exception.
//!
//! See the [W3C `log` module specification][spec] for the canonical
//! definitions.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-log

use serde::Deserialize;

use crate::bidi::BiDi;
use crate::bidi::command::BidiEvent;
use crate::bidi::ids::RealmId;
use crate::common::protocol::string_enum;

string_enum! {
    /// Log entry severity for [`events::EntryAdded::level`]. Mirrors
    /// the spec's `log.LogLevel` enumeration.
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

    /// [`log.entryAdded`][spec] — fires for every `console.*` call and
    /// uncaught exception.
    ///
    /// The spec models the entry as a discriminated union over `type`:
    /// `"console"` and `"javascript"` are the standard variants, each
    /// with its own type-specific fields. This struct types the common
    /// fields and routes the rest through [`EntryAdded::extra`] so the
    /// caller can destructure with serde where needed.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-log-entryAdded
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EntryAdded {
        /// Entry kind. `"console"` for `console.*` calls,
        /// `"javascript"` for uncaught exceptions, plus any
        /// driver-specific values.
        #[serde(rename = "type")]
        pub entry_type: String,
        /// Severity.
        pub level: LogLevel,
        /// Source realm — a [`script.Source`][spec] JSON value.
        ///
        /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-Source
        #[serde(default)]
        pub source: serde_json::Value,
        /// Coalesced text representation. For `console.log("a", 1)`
        /// this is something like `"a 1"` — the underlying argument
        /// values live in [`extra`][Self::extra]`.args`.
        #[serde(default)]
        pub text: Option<String>,
        /// Driver timestamp (ms since unix epoch).
        pub timestamp: u64,
        /// Stack trace as a [`script.StackTrace`][spec] JSON value (or
        /// `null` if no stack was captured).
        ///
        /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-StackTrace
        #[serde(default)]
        pub stack_trace: serde_json::Value,
        /// JavaScript realm the entry came from (where applicable).
        #[serde(default)]
        pub realm: Option<RealmId>,
        /// Type-specific extras — e.g. `method` (`"log"`, `"warn"`,
        /// …) and `args` (array of [`script.RemoteValue`][rv]) for
        /// `"console"` entries.
        ///
        /// [rv]: https://w3c.github.io/webdriver-bidi/#type-script-RemoteValue
        #[serde(flatten)]
        pub extra: serde_json::Map<String, serde_json::Value>,
    }

    impl BidiEvent for EntryAdded {
        const METHOD: &'static str = "log.entryAdded";
    }
}

/// Module facade returned by [`BiDi::log`](crate::bidi::BiDi::log).
///
/// The `log` module is event-only — the spec defines no commands. This
/// facade exists for symmetry with the other modules; subscribe to
/// [`events::EntryAdded`] (or
/// [`bidi::events::LogEntryAdded`](crate::bidi::events::LogEntryAdded))
/// via [`BiDi::subscribe`](crate::bidi::BiDi::subscribe) to receive log
/// entries.
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
