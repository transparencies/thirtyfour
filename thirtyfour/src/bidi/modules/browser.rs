//! `browser.*` BiDi module — top-level browser control and user contexts.

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::UserContextId;

/// `browser.close`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Close;

impl BidiCommand for Close {
    const METHOD: &'static str = "browser.close";
    type Returns = Empty;
}

/// `browser.createUserContext`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateUserContext;

impl BidiCommand for CreateUserContext {
    const METHOD: &'static str = "browser.createUserContext";
    type Returns = UserContextInfo;
}

/// One user context (incognito-like partition).
#[derive(Debug, Clone, Deserialize)]
pub struct UserContextInfo {
    /// User context id.
    #[serde(rename = "userContext")]
    pub user_context: UserContextId,
}

/// `browser.getUserContexts`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetUserContexts;

impl BidiCommand for GetUserContexts {
    const METHOD: &'static str = "browser.getUserContexts";
    type Returns = GetUserContextsResult;
}

/// Response for [`GetUserContexts`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetUserContextsResult {
    /// All known user contexts (always includes `"default"`).
    #[serde(rename = "userContexts")]
    pub user_contexts: Vec<UserContextInfo>,
}

/// `browser.removeUserContext`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveUserContext {
    /// User context to remove. The default user context cannot be removed.
    pub user_context: UserContextId,
}

impl BidiCommand for RemoveUserContext {
    const METHOD: &'static str = "browser.removeUserContext";
    type Returns = Empty;
}

/// Module facade returned by [`BiDi::browser`].
#[derive(Debug)]
pub struct BrowserModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> BrowserModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// `browser.createUserContext` — opens an incognito-like partition.
    pub async fn create_user_context(&self) -> Result<UserContextInfo, BidiError> {
        self.bidi.send(CreateUserContext).await
    }

    /// `browser.getUserContexts` — list all current user contexts.
    pub async fn get_user_contexts(&self) -> Result<GetUserContextsResult, BidiError> {
        self.bidi.send(GetUserContexts).await
    }

    /// `browser.removeUserContext` — close every context inside the partition.
    pub async fn remove_user_context(&self, user_context: UserContextId) -> Result<(), BidiError> {
        self.bidi
            .send(RemoveUserContext {
                user_context,
            })
            .await?;
        Ok(())
    }
}
