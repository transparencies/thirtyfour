//! `Target` domain — pages, workers, and CDP session attachment.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::cdp::ids::{BrowserContextId, SessionId, TargetId};
use crate::error::WebDriverResult;

/// `Target.attachToTarget`. Use `flatten: true`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTarget {
    /// Target id to attach to.
    pub target_id: TargetId,
    /// If true, multiplex all sessions on this connection by `sessionId`.
    pub flatten: bool,
}
impl AttachToTarget {
    /// Construct in flat-mode (the modern default).
    pub fn flat(target_id: TargetId) -> Self {
        Self {
            target_id,
            flatten: true,
        }
    }
}
/// Response for [`AttachToTarget`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetResult {
    /// Session id assigned to the new attachment.
    pub session_id: SessionId,
}
impl CdpCommand for AttachToTarget {
    const METHOD: &'static str = "Target.attachToTarget";
    type Returns = AttachToTargetResult;
}

/// `Target.detachFromTarget`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachFromTarget {
    /// Session id to detach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}
impl CdpCommand for DetachFromTarget {
    const METHOD: &'static str = "Target.detachFromTarget";
    type Returns = Empty;
}

/// `Target.setAutoAttach` — auto-attach to all new targets.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAutoAttach {
    /// Whether to auto-attach.
    pub auto_attach: bool,
    /// Pause new targets at debugger statements.
    pub wait_for_debugger_on_start: bool,
    /// Use flat session mode.
    pub flatten: bool,
}
impl CdpCommand for SetAutoAttach {
    const METHOD: &'static str = "Target.setAutoAttach";
    type Returns = Empty;
}

/// `Target.getTargets`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetTargets;
/// One entry returned by [`GetTargets`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    /// Target id.
    pub target_id: TargetId,
    /// Target type (e.g. `"page"`, `"iframe"`, `"worker"`).
    pub r#type: String,
    /// Title.
    pub title: String,
    /// URL.
    pub url: String,
    /// Whether the target is currently attached.
    pub attached: bool,
    /// Browser context id.
    pub browser_context_id: Option<BrowserContextId>,
    /// Opener id.
    pub opener_id: Option<TargetId>,
}
/// Response for [`GetTargets`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetsResult {
    /// All targets.
    pub target_infos: Vec<TargetInfo>,
}
impl CdpCommand for GetTargets {
    const METHOD: &'static str = "Target.getTargets";
    type Returns = GetTargetsResult;
}

/// `Target.createTarget`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTarget {
    /// URL to open in the new tab.
    pub url: String,
    /// Width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Browser context to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextId>,
    /// Whether to create a background tab (Chrome only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
}
/// Response for [`CreateTarget`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetResult {
    /// Id of the newly created target.
    pub target_id: TargetId,
}
impl CdpCommand for CreateTarget {
    const METHOD: &'static str = "Target.createTarget";
    type Returns = CreateTargetResult;
}

/// `Target.closeTarget`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseTarget {
    /// Target id to close.
    pub target_id: TargetId,
}
/// Response for [`CloseTarget`]. CDP includes a deprecated `success`
/// boolean that is always `true` for compatibility.
#[derive(Debug, Clone, Deserialize)]
pub struct CloseTargetResult {
    /// Always `true` — CDP keeps the field for backwards compatibility.
    pub success: bool,
}
impl CdpCommand for CloseTarget {
    const METHOD: &'static str = "Target.closeTarget";
    type Returns = CloseTargetResult;
}

/// `Target.attachedToTarget` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedToTarget {
    /// Session id assigned to this attachment.
    pub session_id: SessionId,
    /// Description of the new target.
    pub target_info: TargetInfo,
    /// Whether the target was paused at the debugger.
    pub waiting_for_debugger: bool,
}
impl CdpEvent for AttachedToTarget {
    const METHOD: &'static str = "Target.attachedToTarget";
}

/// `Target.detachedFromTarget` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedFromTarget {
    /// Session id of the detachment.
    pub session_id: SessionId,
    /// Target id of the detached target.
    pub target_id: Option<TargetId>,
}
impl CdpEvent for DetachedFromTarget {
    const METHOD: &'static str = "Target.detachedFromTarget";
}

/// Domain facade returned by [`Cdp::target`].
#[derive(Debug)]
pub struct TargetDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> TargetDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Target.getTargets`.
    pub async fn get_targets(&self) -> WebDriverResult<Vec<TargetInfo>> {
        let r = self.cdp.send(GetTargets).await?;
        Ok(r.target_infos)
    }

    /// `Target.createTarget`.
    pub async fn create_target(&self, url: impl Into<String>) -> WebDriverResult<TargetId> {
        let r = self
            .cdp
            .send(CreateTarget {
                url: url.into(),
                width: None,
                height: None,
                browser_context_id: None,
                background: None,
            })
            .await?;
        Ok(r.target_id)
    }

    /// `Target.closeTarget`. Returns `true` on success — CDP only ever
    /// returns `true` here, so most callers can ignore the result.
    pub async fn close_target(&self, target_id: TargetId) -> WebDriverResult<bool> {
        Ok(self
            .cdp
            .send(CloseTarget {
                target_id,
            })
            .await?
            .success)
    }
}
