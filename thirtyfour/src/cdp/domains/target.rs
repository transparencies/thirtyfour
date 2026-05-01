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
impl CdpCommand for CloseTarget {
    const METHOD: &'static str = "Target.closeTarget";
    type Returns = Empty;
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

    /// `Target.closeTarget`.
    pub async fn close_target(&self, target_id: TargetId) -> WebDriverResult<()> {
        self.cdp
            .send(CloseTarget {
                target_id,
            })
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn methods() {
        assert_eq!(AttachToTarget::METHOD, "Target.attachToTarget");
        assert_eq!(DetachFromTarget::METHOD, "Target.detachFromTarget");
        assert_eq!(SetAutoAttach::METHOD, "Target.setAutoAttach");
        assert_eq!(GetTargets::METHOD, "Target.getTargets");
        assert_eq!(CreateTarget::METHOD, "Target.createTarget");
        assert_eq!(CloseTarget::METHOD, "Target.closeTarget");
    }

    #[test]
    fn event_methods() {
        assert_eq!(AttachedToTarget::METHOD, "Target.attachedToTarget");
        assert_eq!(DetachedFromTarget::METHOD, "Target.detachedFromTarget");
    }

    #[test]
    fn attach_to_target_flat_mode() {
        let v = serde_json::to_value(AttachToTarget::flat(TargetId::from("T1"))).unwrap();
        assert_eq!(v["targetId"], "T1");
        assert_eq!(v["flatten"], true);
    }

    #[test]
    fn attach_to_target_result_parses() {
        let r: AttachToTargetResult = serde_json::from_value(json!({"sessionId": "S1"})).unwrap();
        assert_eq!(r.session_id.as_str(), "S1");
    }

    #[test]
    fn detach_from_target_with_session_id() {
        let v = serde_json::to_value(DetachFromTarget {
            session_id: Some(SessionId::from("S1")),
        })
        .unwrap();
        assert_eq!(v["sessionId"], "S1");
    }

    #[test]
    fn detach_from_target_default_skips() {
        let v = serde_json::to_value(DetachFromTarget::default()).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn set_auto_attach_required_fields() {
        let v = serde_json::to_value(SetAutoAttach {
            auto_attach: true,
            wait_for_debugger_on_start: false,
            flatten: true,
        })
        .unwrap();
        assert_eq!(v["autoAttach"], true);
        assert_eq!(v["waitForDebuggerOnStart"], false);
        assert_eq!(v["flatten"], true);
    }

    #[test]
    fn get_targets_response_uses_camel_case() {
        let body = json!({
            "targetInfos": [
                {
                    "targetId": "T-1",
                    "type": "page",
                    "title": "",
                    "url": "about:blank",
                    "attached": true
                }
            ]
        });
        let r: GetTargetsResult = serde_json::from_value(body).unwrap();
        assert_eq!(r.target_infos.len(), 1);
        assert_eq!(r.target_infos[0].target_id.as_str(), "T-1");
        assert_eq!(r.target_infos[0].r#type, "page");
        assert!(r.target_infos[0].attached);
    }

    #[test]
    fn target_info_with_browser_context_and_opener() {
        let body = json!({
            "targetId": "T-1",
            "type": "page",
            "title": "child",
            "url": "https://example.com",
            "attached": false,
            "browserContextId": "CTX",
            "openerId": "T-0"
        });
        let info: TargetInfo = serde_json::from_value(body).unwrap();
        assert_eq!(info.browser_context_id.as_ref().unwrap().as_str(), "CTX");
        assert_eq!(info.opener_id.as_ref().unwrap().as_str(), "T-0");
    }

    #[test]
    fn create_target_minimal() {
        let v = serde_json::to_value(CreateTarget {
            url: "about:blank".to_string(),
            width: None,
            height: None,
            browser_context_id: None,
            background: None,
        })
        .unwrap();
        assert_eq!(v["url"], "about:blank");
        for f in ["width", "height", "browserContextId", "background"] {
            assert!(v.get(f).is_none(), "{f} should be omitted");
        }
    }

    #[test]
    fn create_target_full() {
        let v = serde_json::to_value(CreateTarget {
            url: "https://example.com/".to_string(),
            width: Some(1024),
            height: Some(768),
            browser_context_id: Some(BrowserContextId::from("CTX")),
            background: Some(true),
        })
        .unwrap();
        assert_eq!(v["width"], 1024);
        assert_eq!(v["height"], 768);
        assert_eq!(v["browserContextId"], "CTX");
        assert_eq!(v["background"], true);
    }

    #[test]
    fn create_target_result_parses() {
        let r: CreateTargetResult = serde_json::from_value(json!({"targetId": "T-9"})).unwrap();
        assert_eq!(r.target_id.as_str(), "T-9");
    }

    #[test]
    fn close_target_serialises_target_id() {
        let v = serde_json::to_value(CloseTarget {
            target_id: TargetId::from("T-1"),
        })
        .unwrap();
        assert_eq!(v["targetId"], "T-1");
    }

    #[test]
    fn attached_to_target_event_parses() {
        let body = json!({
            "sessionId": "S1",
            "targetInfo": {
                "targetId": "T1",
                "type": "page",
                "title": "",
                "url": "",
                "attached": true
            },
            "waitingForDebugger": false
        });
        let evt: AttachedToTarget = serde_json::from_value(body).unwrap();
        assert_eq!(evt.session_id.as_str(), "S1");
        assert!(!evt.waiting_for_debugger);
    }

    #[test]
    fn detached_from_target_event_parses() {
        let body = json!({"sessionId": "S1", "targetId": "T1"});
        let evt: DetachedFromTarget = serde_json::from_value(body).unwrap();
        assert_eq!(evt.session_id.as_str(), "S1");
        assert_eq!(evt.target_id.as_ref().unwrap().as_str(), "T1");
    }
}
