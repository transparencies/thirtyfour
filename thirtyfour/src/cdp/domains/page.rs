//! `Page` domain — navigation, screenshots, lifecycle events.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, CdpEvent, Empty};
use crate::cdp::ids::{FrameId, LoaderId, ScriptId};
use crate::error::WebDriverResult;

/// `Page.enable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Enable;
impl CdpCommand for Enable {
    const METHOD: &'static str = "Page.enable";
    type Returns = Empty;
}

/// `Page.disable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Disable;
impl CdpCommand for Disable {
    const METHOD: &'static str = "Page.disable";
    type Returns = Empty;
}

/// `Page.navigate` params.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Navigate {
    /// URL to navigate to.
    pub url: String,
    /// Referrer URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    /// Intended transition type (e.g. `"link"`, `"typed"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_type: Option<String>,
    /// Frame id to navigate; defaults to the main frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
}

impl Navigate {
    /// Construct a navigate command for the main frame.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            referrer: None,
            transition_type: None,
            frame_id: None,
        }
    }
}

/// Response for [`Navigate`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateResult {
    /// Frame id that was navigated.
    pub frame_id: FrameId,
    /// Loader id of the navigation. `None` for fragment navigations.
    #[serde(default)]
    pub loader_id: Option<LoaderId>,
    /// User-friendly error message if navigation failed.
    #[serde(default)]
    pub error_text: Option<String>,
}

impl CdpCommand for Navigate {
    const METHOD: &'static str = "Page.navigate";
    type Returns = NavigateResult;
}

/// `Page.reload`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reload {
    /// If true, ignores the cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_cache: Option<bool>,
    /// If set, this script will be injected into all frames at load time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_to_evaluate_on_load: Option<String>,
}
impl CdpCommand for Reload {
    const METHOD: &'static str = "Page.reload";
    type Returns = Empty;
}

/// `Page.captureScreenshot`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshot {
    /// `"png"` (default) or `"jpeg"` or `"webp"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// JPEG/WebP quality in `[0, 100]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<u32>,
    /// `true` to capture beyond the viewport.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_beyond_viewport: Option<bool>,
}

/// Response for [`CaptureScreenshot`].
#[derive(Debug, Clone, Deserialize)]
pub struct ScreenshotData {
    /// Base64-encoded image data.
    pub data: String,
}

impl CdpCommand for CaptureScreenshot {
    const METHOD: &'static str = "Page.captureScreenshot";
    type Returns = ScreenshotData;
}

/// `Page.printToPDF` (subset of params).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintToPdf {
    /// Paper width, in inches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_width: Option<f64>,
    /// Paper height, in inches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_height: Option<f64>,
    /// Print landscape orientation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landscape: Option<bool>,
    /// Display header and footer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_header_footer: Option<bool>,
    /// Print background graphics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_background: Option<bool>,
    /// Scale of the webpage rendering. Default is 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
}

/// Response for [`PrintToPdf`].
#[derive(Debug, Clone, Deserialize)]
pub struct PdfData {
    /// Base64-encoded PDF bytes.
    pub data: String,
}

impl CdpCommand for PrintToPdf {
    const METHOD: &'static str = "Page.printToPDF";
    type Returns = PdfData;
}

/// `Page.addScriptToEvaluateOnNewDocument` — inject a script that runs on every
/// new document before any page scripts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnNewDocument {
    /// Script source.
    pub source: String,
    /// World name to inject into. Defaults to the main world.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    /// Whether to inject into all frames.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
}

/// Response for [`AddScriptToEvaluateOnNewDocument`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedScript {
    /// Identifier of the script.
    pub identifier: ScriptId,
}

impl CdpCommand for AddScriptToEvaluateOnNewDocument {
    const METHOD: &'static str = "Page.addScriptToEvaluateOnNewDocument";
    type Returns = AddedScript;
}

/// `Page.removeScriptToEvaluateOnNewDocument`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveScriptToEvaluateOnNewDocument {
    /// Identifier returned by [`AddScriptToEvaluateOnNewDocument`].
    pub identifier: ScriptId,
}
impl CdpCommand for RemoveScriptToEvaluateOnNewDocument {
    const METHOD: &'static str = "Page.removeScriptToEvaluateOnNewDocument";
    type Returns = Empty;
}

/// `Page.setLifecycleEventsEnabled`.
#[derive(Debug, Clone, Serialize)]
pub struct SetLifecycleEventsEnabled {
    /// Whether to emit `Page.lifecycleEvent`.
    pub enabled: bool,
}
impl CdpCommand for SetLifecycleEventsEnabled {
    const METHOD: &'static str = "Page.setLifecycleEventsEnabled";
    type Returns = Empty;
}

/// `Page.frameNavigated` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameNavigated {
    /// Frame description after navigation.
    pub frame: serde_json::Value,
    /// Type of navigation: `"Navigation"` or `"BackForwardCacheRestore"`.
    #[serde(default)]
    pub r#type: Option<String>,
}
impl CdpEvent for FrameNavigated {
    const METHOD: &'static str = "Page.frameNavigated";
}

/// `Page.lifecycleEvent` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEvent {
    /// Frame the event applies to.
    pub frame_id: FrameId,
    /// Loader id.
    pub loader_id: LoaderId,
    /// Event name (e.g. `"init"`, `"DOMContentLoaded"`, `"load"`).
    pub name: String,
    /// `Network.MonotonicTime` of the event.
    pub timestamp: f64,
}
impl CdpEvent for LifecycleEvent {
    const METHOD: &'static str = "Page.lifecycleEvent";
}

/// `Page.loadEventFired` event.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadEventFired {
    /// `Network.MonotonicTime` of the event.
    pub timestamp: f64,
}
impl CdpEvent for LoadEventFired {
    const METHOD: &'static str = "Page.loadEventFired";
}

/// Domain facade returned by [`Cdp::page`].
#[derive(Debug)]
pub struct PageDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> PageDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Page.enable`.
    pub async fn enable(&self) -> WebDriverResult<()> {
        self.cdp.send(Enable).await?;
        Ok(())
    }

    /// `Page.disable`.
    pub async fn disable(&self) -> WebDriverResult<()> {
        self.cdp.send(Disable).await?;
        Ok(())
    }

    /// `Page.navigate`.
    pub async fn navigate(&self, url: impl Into<String>) -> WebDriverResult<NavigateResult> {
        self.cdp.send(Navigate::new(url)).await
    }

    /// `Page.reload`.
    pub async fn reload(&self) -> WebDriverResult<()> {
        self.cdp.send(Reload::default()).await?;
        Ok(())
    }

    /// `Page.captureScreenshot` — returns the raw base64-encoded image data.
    pub async fn capture_screenshot_base64(&self) -> WebDriverResult<String> {
        Ok(self.cdp.send(CaptureScreenshot::default()).await?.data)
    }

    /// `Page.printToPDF` — returns the raw base64-encoded PDF bytes.
    pub async fn print_to_pdf_base64(&self) -> WebDriverResult<String> {
        Ok(self.cdp.send(PrintToPdf::default()).await?.data)
    }

    /// `Page.addScriptToEvaluateOnNewDocument`.
    pub async fn add_script_to_evaluate_on_new_document(
        &self,
        source: impl Into<String>,
    ) -> WebDriverResult<ScriptId> {
        Ok(self
            .cdp
            .send(AddScriptToEvaluateOnNewDocument {
                source: source.into(),
                world_name: None,
                include_command_line_api: None,
            })
            .await?
            .identifier)
    }

    /// `Page.removeScriptToEvaluateOnNewDocument`.
    pub async fn remove_script_to_evaluate_on_new_document(
        &self,
        identifier: ScriptId,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(RemoveScriptToEvaluateOnNewDocument {
                identifier,
            })
            .await?;
        Ok(())
    }

    /// `Page.setLifecycleEventsEnabled`.
    pub async fn set_lifecycle_events_enabled(&self, enabled: bool) -> WebDriverResult<()> {
        self.cdp
            .send(SetLifecycleEventsEnabled {
                enabled,
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
        assert_eq!(Enable::METHOD, "Page.enable");
        assert_eq!(Disable::METHOD, "Page.disable");
        assert_eq!(Navigate::METHOD, "Page.navigate");
        assert_eq!(Reload::METHOD, "Page.reload");
        assert_eq!(CaptureScreenshot::METHOD, "Page.captureScreenshot");
        assert_eq!(PrintToPdf::METHOD, "Page.printToPDF");
        assert_eq!(
            AddScriptToEvaluateOnNewDocument::METHOD,
            "Page.addScriptToEvaluateOnNewDocument"
        );
        assert_eq!(
            RemoveScriptToEvaluateOnNewDocument::METHOD,
            "Page.removeScriptToEvaluateOnNewDocument"
        );
        assert_eq!(SetLifecycleEventsEnabled::METHOD, "Page.setLifecycleEventsEnabled");
    }

    #[test]
    fn navigate_minimal() {
        let cmd = Navigate::new("https://example.com/");
        let v = serde_json::to_value(&cmd).unwrap();
        assert_eq!(v["url"], "https://example.com/");
        assert!(v.get("referrer").is_none());
        assert!(v.get("frameId").is_none());
        assert!(v.get("transitionType").is_none());
    }

    #[test]
    fn navigate_with_frame_and_referrer() {
        let cmd = Navigate {
            url: "https://example.com/".to_string(),
            referrer: Some("https://ref.com/".to_string()),
            transition_type: Some("link".to_string()),
            frame_id: Some(FrameId::from("F1")),
        };
        let v = serde_json::to_value(&cmd).unwrap();
        assert_eq!(v["referrer"], "https://ref.com/");
        assert_eq!(v["transitionType"], "link");
        assert_eq!(v["frameId"], "F1");
    }

    #[test]
    fn navigate_result_full() {
        let body = json!({
            "frameId": "F1",
            "loaderId": "L1",
            "errorText": null
        });
        let r: NavigateResult = serde_json::from_value(body).unwrap();
        assert_eq!(r.frame_id.as_str(), "F1");
        assert_eq!(r.loader_id.as_ref().unwrap().as_str(), "L1");
        assert!(r.error_text.is_none());
    }

    #[test]
    fn navigate_result_with_error() {
        let body = json!({
            "frameId": "F1",
            "errorText": "net::ERR_NAME_NOT_RESOLVED"
        });
        let r: NavigateResult = serde_json::from_value(body).unwrap();
        assert!(r.loader_id.is_none());
        assert_eq!(r.error_text.as_deref(), Some("net::ERR_NAME_NOT_RESOLVED"));
    }

    #[test]
    fn reload_skips_optional_fields_when_default() {
        let v = serde_json::to_value(Reload::default()).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn reload_emits_ignore_cache() {
        let v = serde_json::to_value(Reload {
            ignore_cache: Some(true),
            script_to_evaluate_on_load: Some("console.log(1)".to_string()),
        })
        .unwrap();
        assert_eq!(v["ignoreCache"], true);
        assert_eq!(v["scriptToEvaluateOnLoad"], "console.log(1)");
    }

    #[test]
    fn screenshot_data_parse() {
        let r: ScreenshotData = serde_json::from_value(json!({"data": "BASE64"})).unwrap();
        assert_eq!(r.data, "BASE64");
    }

    #[test]
    fn capture_screenshot_format_and_quality() {
        let v = serde_json::to_value(CaptureScreenshot {
            format: Some("jpeg".to_string()),
            quality: Some(80),
            capture_beyond_viewport: Some(true),
        })
        .unwrap();
        assert_eq!(v["format"], "jpeg");
        assert_eq!(v["quality"], 80);
        assert_eq!(v["captureBeyondViewport"], true);
    }

    #[test]
    fn print_to_pdf_field_names() {
        let v = serde_json::to_value(PrintToPdf {
            paper_width: Some(8.5),
            paper_height: Some(11.0),
            landscape: Some(true),
            display_header_footer: Some(false),
            print_background: Some(true),
            scale: Some(1.5),
        })
        .unwrap();
        assert_eq!(v["paperWidth"], 8.5);
        assert_eq!(v["paperHeight"], 11.0);
        assert_eq!(v["landscape"], true);
        assert_eq!(v["displayHeaderFooter"], false);
        assert_eq!(v["printBackground"], true);
        assert_eq!(v["scale"], 1.5);
    }

    #[test]
    fn pdf_data_parse() {
        let r: PdfData = serde_json::from_value(json!({"data": "BASE64"})).unwrap();
        assert_eq!(r.data, "BASE64");
    }

    #[test]
    fn add_script_serialises_world_name() {
        let v = serde_json::to_value(AddScriptToEvaluateOnNewDocument {
            source: "console.log(1)".to_string(),
            world_name: Some("isolated".to_string()),
            include_command_line_api: Some(false),
        })
        .unwrap();
        assert_eq!(v["source"], "console.log(1)");
        assert_eq!(v["worldName"], "isolated");
        assert_eq!(v["includeCommandLineApi"], false);
    }

    #[test]
    fn add_script_returns_identifier() {
        let r: AddedScript = serde_json::from_value(json!({"identifier": 42})).unwrap();
        assert_eq!(r.identifier.get(), 42);
    }

    #[test]
    fn remove_script_serialises_identifier() {
        let v = serde_json::to_value(RemoveScriptToEvaluateOnNewDocument {
            identifier: ScriptId::new(7),
        })
        .unwrap();
        assert_eq!(v["identifier"], 7);
    }

    #[test]
    fn set_lifecycle_enables_flag() {
        let v = serde_json::to_value(SetLifecycleEventsEnabled {
            enabled: true,
        })
        .unwrap();
        assert_eq!(v["enabled"], true);
    }

    #[test]
    fn frame_navigated_event_methods() {
        assert_eq!(FrameNavigated::METHOD, "Page.frameNavigated");
        assert_eq!(LifecycleEvent::METHOD, "Page.lifecycleEvent");
        assert_eq!(LoadEventFired::METHOD, "Page.loadEventFired");
    }

    #[test]
    fn frame_navigated_event_parse() {
        let body = json!({
            "frame": {"id": "F1", "url": "about:blank"},
            "type": "Navigation"
        });
        let evt: FrameNavigated = serde_json::from_value(body).unwrap();
        assert_eq!(evt.r#type.as_deref(), Some("Navigation"));
    }

    #[test]
    fn lifecycle_event_parse() {
        let body = json!({
            "frameId": "F1",
            "loaderId": "L1",
            "name": "DOMContentLoaded",
            "timestamp": 12345.0
        });
        let evt: LifecycleEvent = serde_json::from_value(body).unwrap();
        assert_eq!(evt.frame_id.as_str(), "F1");
        assert_eq!(evt.loader_id.as_str(), "L1");
        assert_eq!(evt.name, "DOMContentLoaded");
        assert!((evt.timestamp - 12345.0).abs() < f64::EPSILON);
    }

    #[test]
    fn load_event_fired_parse() {
        let evt: LoadEventFired = serde_json::from_value(json!({"timestamp": 1.0})).unwrap();
        assert!((evt.timestamp - 1.0).abs() < f64::EPSILON);
    }
}
