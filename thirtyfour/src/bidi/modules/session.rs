//! `session.*` BiDi module — protocol handshake, subscription, status.

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, UserContextId};

/// `session.status`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Status;

/// Response for [`Status`].
#[derive(Debug, Clone, Deserialize)]
pub struct StatusResult {
    /// True when the driver is ready to accept a new session.
    pub ready: bool,
    /// Human-readable readiness message.
    pub message: String,
}

impl BidiCommand for Status {
    const METHOD: &'static str = "session.status";
    type Returns = StatusResult;
}

/// `session.subscribe` — register interest in one or more event names.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Subscribe {
    /// Event names like `"browsingContext.load"` or whole modules like
    /// `"browsingContext"` (subscribes to every event in the module).
    pub events: Vec<String>,
    /// Optional list of browsing-context ids to scope the subscription.
    /// Empty / omitted = global.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
    /// Optional list of user-context ids to scope the subscription.
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
    /// Server-assigned id that can be passed to [`Unsubscribe`] to remove
    /// just this subscription. Optional — older drivers may not send it.
    #[serde(default)]
    pub subscription: Option<String>,
}

/// `session.unsubscribe` — remove a previous subscription. Either by event
/// names or by subscription id.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Unsubscribe {
    /// Event names previously passed to [`Subscribe`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    /// Subscription ids previously returned by [`Subscribe`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subscriptions: Vec<String>,
    /// Optional list of browsing-context ids that scoped the original
    /// subscription.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contexts: Vec<BrowsingContextId>,
}

impl BidiCommand for Unsubscribe {
    const METHOD: &'static str = "session.unsubscribe";
    type Returns = Empty;
}

/// `session.end` — terminates the BiDi session.
#[derive(Debug, Clone, Default, Serialize)]
pub struct End;

impl BidiCommand for End {
    const METHOD: &'static str = "session.end";
    type Returns = Empty;
}

/// Module facade returned by [`BiDi::session`].
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

    /// `session.status`.
    pub async fn status(&self) -> Result<StatusResult, BidiError> {
        self.bidi.send(Status).await
    }

    /// `session.subscribe` — convenience over a single event name.
    pub async fn subscribe(&self, event: impl Into<String>) -> Result<SubscribeResult, BidiError> {
        self.bidi
            .send(Subscribe {
                events: vec![event.into()],
                contexts: vec![],
                user_contexts: vec![],
            })
            .await
    }

    /// `session.subscribe` — multi-event variant.
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

    /// `session.unsubscribe` by event name(s).
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

    /// `session.end`.
    pub async fn end(&self) -> Result<(), BidiError> {
        self.bidi.send(End).await?;
        Ok(())
    }
}
