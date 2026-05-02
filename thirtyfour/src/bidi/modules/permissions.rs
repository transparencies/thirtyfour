//! `permissions.*` — grant, deny, or reset a [Permissions API][permapi]
//! permission for a given origin.
//!
//! Defined by the [W3C Permissions specification's WebDriver-BiDi
//! extension][spec] (a separate document from the core BiDi spec). Driver
//! support is uneven — Chromium supports it; geckodriver currently
//! returns `unknown command`.
//!
//! [permapi]: https://www.w3.org/TR/permissions/
//! [spec]: https://w3c.github.io/permissions/#webdriver-bidi-extension

use serde::Serialize;

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::UserContextId;
use crate::common::protocol::string_enum;

string_enum! {
    /// Target state for [`SetPermission::state`]. Mirrors the spec's
    /// `permissions.PermissionState` enumeration.
    pub enum PermissionState {
        /// Allow the permission without prompting.
        Granted = "granted",
        /// Deny the permission without prompting.
        Denied = "denied",
        /// Reset to default ("ask the user").
        Prompt = "prompt",
    }
}

/// [`permissions.setPermission`][spec] — set the state of a permission
/// for a given origin.
///
/// `descriptor` is a [Permissions API descriptor][descriptor] —
/// typically `{"name": "<name>"}` for a basic permission, or with
/// extra fields for permissions like `"push"` or `"midi"`.
///
/// [spec]: https://w3c.github.io/permissions/#webdriver-bidi-command-permissions-setPermission
/// [descriptor]: https://www.w3.org/TR/permissions/#permission-descriptor
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPermission {
    /// Permission descriptor, e.g. `{"name":"geolocation"}`,
    /// `{"name":"midi","sysex":true}`. See [Permissions API].
    ///
    /// [Permissions API]: https://www.w3.org/TR/permissions/#permission-descriptor
    pub descriptor: serde_json::Value,
    /// Target state.
    pub state: PermissionState,
    /// Origin string (e.g. `"https://example.com"`). Must be an ASCII
    /// serialised origin — paths and trailing slashes are not allowed.
    pub origin: String,
    /// Restrict the change to a specific user context. `None` applies
    /// to the default user context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_context: Option<UserContextId>,
}

impl BidiCommand for SetPermission {
    const METHOD: &'static str = "permissions.setPermission";
    type Returns = Empty;
}

/// Convenience facade for the `permissions.*` module.
///
/// Returned by [`BiDi::permissions`](crate::bidi::BiDi::permissions). To
/// scope the change to a non-default user context, build a
/// [`SetPermission`] struct directly and supply
/// [`user_context`][SetPermission::user_context].
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

    /// Set a permission for `origin` to `state` via
    /// [`permissions.setPermission`][spec].
    ///
    /// `descriptor` is a [Permissions API descriptor][desc] JSON object
    /// (`{"name":"geolocation"}`, etc.).
    ///
    /// [spec]: https://w3c.github.io/permissions/#webdriver-bidi-command-permissions-setPermission
    /// [desc]: https://www.w3.org/TR/permissions/#permission-descriptor
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
