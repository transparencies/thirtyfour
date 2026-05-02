//! `permissions.*` BiDi module — `setPermission`.

use serde::Serialize;

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::UserContextId;
use crate::bidi::macros::string_enum;

string_enum! {
    /// Permission state for [`SetPermission`].
    pub enum PermissionState {
        /// Allow.
        Granted = "granted",
        /// Deny.
        Denied = "denied",
        /// Reset to default ("ask").
        Prompt = "prompt",
    }
}

/// `permissions.setPermission`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPermission {
    /// Permission descriptor (`{"name":"geolocation"}`, etc.) — see
    /// [Permissions API].
    ///
    /// [Permissions API]: https://www.w3.org/TR/permissions/#permission-descriptor
    pub descriptor: serde_json::Value,
    /// New state.
    pub state: PermissionState,
    /// Origin string (e.g. `"https://example.com"`).
    pub origin: String,
    /// Restrict to a user context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_context: Option<UserContextId>,
}

impl BidiCommand for SetPermission {
    const METHOD: &'static str = "permissions.setPermission";
    type Returns = Empty;
}

/// Module facade returned by [`BiDi::permissions`].
#[derive(Debug)]
pub struct PermissionsModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> PermissionsModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// `permissions.setPermission`.
    pub async fn set_permission(
        &self,
        descriptor: serde_json::Value,
        state: PermissionState,
        origin: impl Into<String>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetPermission {
                descriptor,
                state,
                origin: origin.into(),
                user_context: None,
            })
            .await?;
        Ok(())
    }
}
