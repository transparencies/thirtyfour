//! `session.*` — protocol handshake, event subscription, status, end.
//!
//! The session module is the entry point for every BiDi conversation: it
//! negotiates the connection, registers interest in events, and tears the
//! session down. `thirtyfour` opens the session via WebDriver Classic
//! `New Session` (so [`session.new`][spec-new] is not modelled here), but
//! every other session-level command is.
//!
//! See the [W3C `session` module specification][spec] for the canonical
//! definitions, types, and remote-end algorithms.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-session
//! [spec-new]: https://w3c.github.io/webdriver-bidi/#command-session-new

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, UserContextId};

/// [`session.status`][spec] — query whether the driver is ready to accept
/// a new session.
///
/// Once a session is already active most drivers report `ready: false`
/// with an explanatory message; both fields are always populated. This is
/// the BiDi analogue of the WebDriver Classic `GET /status` endpoint.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-status
#[derive(Debug, Clone, Default, Serialize)]
pub struct Status;

/// Response payload for [`Status`].
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResult {
    /// `true` if the driver can create a new session right now.
    pub ready: bool,
    /// Human-readable readiness explanation. Always populated.
    pub message: String,
}

impl BidiCommand for Status {
    const METHOD: &'static str = "session.status";
    type Returns = StatusResult;
}

/// [`session.subscribe`][spec] — register interest in one or more events.
///
/// Each event name is either a fully qualified event method
/// (e.g. `"browsingContext.load"`) or a whole module name
/// (e.g. `"browsingContext"`, which subscribes to every event in that
/// module). Subscriptions can be scoped to specific browsing contexts or
/// user contexts; if both `contexts` and `user_contexts` are empty the
/// subscription is global.
///
/// Most callers should prefer the typed
/// [`BiDi::subscribe::<E>()`](crate::bidi::BiDi::subscribe) helper, which
/// auto-sends this command for `E::METHOD` and tears the subscription
/// down via [`Unsubscribe`] when the last stream drops.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-subscribe
#[derive(Debug, Clone, Default, Serialize)]
pub struct Subscribe {
    /// Event method names (e.g. `"browsingContext.load"`) or whole module
    /// names (e.g. `"browsingContext"`).
    pub events: Vec<String>,
    /// Restrict the subscription to specific browsing contexts. Empty =
    /// any context.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
    /// Restrict the subscription to specific user contexts. Empty = any
    /// user context.
    #[serde(rename = "userContexts", skip_serializing_if = "Vec::is_empty")]
    pub user_contexts: Vec<UserContextId>,
}

impl BidiCommand for Subscribe {
    const METHOD: &'static str = "session.subscribe";
    type Returns = SubscribeResult;
}

/// Response for [`Subscribe`].
#[derive(Debug, Clone, Deserialize)]
pub struct SubscribeResult {
    /// Server-assigned subscription id. Pass it to
    /// [`Unsubscribe::subscriptions`] to revoke this subscription
    /// individually. The field is optional because older drivers may
    /// omit it — fall back to unsubscribing by event name in that case.
    #[serde(default)]
    pub subscription: Option<String>,
}

/// [`session.unsubscribe`][spec] — drop a previously-installed subscription.
///
/// Subscriptions can be removed two ways:
/// - By **event name(s)** ([`events`][Self::events]) — useful for the
///   classic "subscribe then later unsubscribe" pattern when you didn't
///   keep the subscription id.
/// - By **subscription id** ([`subscriptions`][Self::subscriptions]) —
///   removes only the specific call previously returned by
///   [`Subscribe`]. Preferred when multiple subscriptions overlap.
///
/// Use [`contexts`][Self::contexts] to scope event-name unsubscribes to a
/// specific subset of browsing contexts.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-unsubscribe
#[derive(Debug, Clone, Default, Serialize)]
pub struct Unsubscribe {
    /// Event method or module names (mirror of [`Subscribe::events`]).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    /// Subscription ids previously returned by [`Subscribe`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subscriptions: Vec<String>,
    /// Browsing contexts scope (only applies to event-name unsubscribes).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
}

impl BidiCommand for Unsubscribe {
    const METHOD: &'static str = "session.unsubscribe";
    type Returns = Empty;
}

/// [`session.end`][spec] — terminate the BiDi session.
///
/// This ends the BiDi conversation but **does not** terminate the
/// underlying WebDriver Classic session — call
/// [`WebDriver::quit`](crate::WebDriver::quit) for that. After `end()`
/// the BiDi WebSocket is closed and the connection cannot be reused.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-end
#[derive(Debug, Clone, Default, Serialize)]
pub struct End;

impl BidiCommand for End {
    const METHOD: &'static str = "session.end";
    type Returns = Empty;
}

/// Convenience facade for the `session.*` module.
///
/// Returned by [`BiDi::session`](crate::bidi::BiDi::session). Each method
/// here wraps a single command for the common case; for fine-grained
/// control (subscription scopes, multi-id unsubscribe, etc.) build the
/// command struct directly and send it via
/// [`BiDi::send`](crate::bidi::BiDi::send).
#[derive(Debug)]
pub struct SessionModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> SessionModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// Run [`session.status`][spec] and return the result.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-status
    pub async fn status(&self) -> Result<StatusResult, BidiError> {
        self.bidi.send(Status).await
    }

    /// Subscribe to a single global event by name (e.g. `"browsingContext.load"`).
    ///
    /// This sends [`session.subscribe`][spec] with one event name and no
    /// scope. Prefer [`BiDi::subscribe::<E>()`](crate::bidi::BiDi::subscribe)
    /// when you also want a typed local stream — it sends the same
    /// command and reference-counts the subscription so you can call it
    /// repeatedly without leaking subscriptions on the wire.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-subscribe
    pub async fn subscribe(&self, event: impl Into<String>) -> Result<SubscribeResult, BidiError> {
        self.bidi
            .send(Subscribe {
                events: vec![event.into()],
                contexts: vec![],
                user_contexts: vec![],
            })
            .await
    }

    /// Subscribe to multiple global events at once.
    ///
    /// Single-roundtrip equivalent of calling
    /// [`subscribe`](Self::subscribe) once per event name.
    pub async fn subscribe_many(
        &self,
        events: impl IntoIterator<Item = String>,
    ) -> Result<SubscribeResult, BidiError> {
        self.bidi
            .send(Subscribe {
                events: events.into_iter().collect(),
                contexts: vec![],
                user_contexts: vec![],
            })
            .await
    }

    /// Unsubscribe by event method names.
    ///
    /// Equivalent to sending [`session.unsubscribe`][spec] with the named
    /// events and no subscription ids or context scope.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-unsubscribe
    pub async fn unsubscribe(
        &self,
        events: impl IntoIterator<Item = String>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(Unsubscribe {
                events: events.into_iter().collect(),
                subscriptions: vec![],
                contexts: vec![],
            })
            .await?;
        Ok(())
    }

    /// End the BiDi session via [`session.end`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-session-end
    pub async fn end(&self) -> Result<(), BidiError> {
        self.bidi.send(End).await?;
        Ok(())
    }
}
