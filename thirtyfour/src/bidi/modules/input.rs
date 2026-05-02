//! `input.*` BiDi module — `performActions`, `releaseActions`, `setFiles`.
//!
//! Action sources (key, pointer, wheel, none) follow the same JSON shape
//! as W3C WebDriver classic actions. Action source objects are passed
//! through as `serde_json::Value` so callers can construct them with the
//! `json!` macro or build their own typed wrappers.

use serde::Serialize;

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, NodeId};

/// `input.performActions`.
#[derive(Debug, Clone, Serialize)]
pub struct PerformActions {
    /// Browsing context the actions apply to.
    pub context: BrowsingContextId,
    /// One entry per input source. Each is a JSON object with `id`, `type`,
    /// optional `parameters`, and an `actions` array.
    pub actions: Vec<serde_json::Value>,
}

impl BidiCommand for PerformActions {
    const METHOD: &'static str = "input.performActions";
    type Returns = Empty;
}

/// `input.releaseActions`.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseActions {
    /// Browsing context.
    pub context: BrowsingContextId,
}

impl BidiCommand for ReleaseActions {
    const METHOD: &'static str = "input.releaseActions";
    type Returns = Empty;
}

/// `input.setFiles`.
#[derive(Debug, Clone, Serialize)]
pub struct SetFiles {
    /// Browsing context.
    pub context: BrowsingContextId,
    /// Element to set files on, as a BiDi `script.SharedReference`. Use
    /// [`shared_reference`] to construct.
    pub element: serde_json::Value,
    /// File paths.
    pub files: Vec<String>,
}

/// Wrap a [`NodeId`] in the `script.SharedReference` JSON shape that
/// `input.setFiles` expects (`{"sharedId":"…"}`).
pub fn shared_reference(node: &NodeId) -> serde_json::Value {
    serde_json::json!({"sharedId": node.as_str()})
}

impl BidiCommand for SetFiles {
    const METHOD: &'static str = "input.setFiles";
    type Returns = Empty;
}

/// Module facade returned by [`BiDi::input`].
#[derive(Debug)]
pub struct InputModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> InputModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// `input.performActions`.
    pub async fn perform_actions(
        &self,
        context: BrowsingContextId,
        actions: Vec<serde_json::Value>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(PerformActions {
                context,
                actions,
            })
            .await?;
        Ok(())
    }

    /// `input.releaseActions`.
    pub async fn release_actions(&self, context: BrowsingContextId) -> Result<(), BidiError> {
        self.bidi
            .send(ReleaseActions {
                context,
            })
            .await?;
        Ok(())
    }

    /// `input.setFiles`.
    pub async fn set_files(
        &self,
        context: BrowsingContextId,
        element: &NodeId,
        files: Vec<String>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetFiles {
                context,
                element: shared_reference(element),
                files,
            })
            .await?;
        Ok(())
    }
}
