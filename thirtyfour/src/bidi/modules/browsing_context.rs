//! `browsingContext.*` BiDi module — tabs, frames, navigation, screenshots.

use serde::{Deserialize, Deserializer, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, BidiEvent, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, NavigationId, UserContextId};
use crate::common::protocol::string_enum;

string_enum! {
    /// Wait condition for [`Navigate`] (`browsingContext.navigate`).
    pub enum ReadinessState {
        /// Resolve as soon as the navigation has been initiated.
        None = "none",
        /// Resolve when the document has been parsed.
        Interactive = "interactive",
        /// Resolve when the load event has fired.
        Complete = "complete",
    }
}

string_enum! {
    /// Lifecycle phase for [`events::DomContentLoaded`]-style events.
    pub enum LifecyclePhase {
        /// Document has loaded.
        Load = "load",
        /// `DOMContentLoaded` fired.
        DomContentLoaded = "DOMContentLoaded",
    }
}

string_enum! {
    /// New-context creation type.
    pub enum CreateType {
        /// Open as a new tab in the existing window.
        Tab = "tab",
        /// Open as a new top-level window.
        Window = "window",
    }
}

string_enum! {
    /// Image format for [`CaptureScreenshot`].
    pub enum ImageFormatType {
        /// PNG.
        Png = "png",
        /// JPEG (optionally lossy).
        Jpeg = "jpeg",
    }
}

string_enum! {
    /// History traversal direction in [`TraverseHistory`].
    pub enum TraversalDirection {
        /// Equivalent to clicking the browser back button.
        Back = "back",
        /// Equivalent to clicking the browser forward button.
        Forward = "forward",
    }
}

/// `browsingContext.getTree`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTree {
    /// Limit traversal depth. `Some(0)` returns just the matched roots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    /// Restrict the tree to descendants of this context. `None` returns all
    /// top-level contexts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<BrowsingContextId>,
}

impl BidiCommand for GetTree {
    const METHOD: &'static str = "browsingContext.getTree";
    type Returns = GetTreeResult;
}

/// Response for [`GetTree`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetTreeResult {
    /// One node per top-level context (or one node if [`GetTree::root`] was
    /// set). Children populate [`BrowsingContextInfo::children`].
    pub contexts: Vec<BrowsingContextInfo>,
}

/// A node in the browsing-context tree.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowsingContextInfo {
    /// Identifier for this context.
    pub context: BrowsingContextId,
    /// Top-level context id this node belongs to.
    pub parent: Option<BrowsingContextId>,
    /// User context (incognito-like partition) the context lives in.
    #[serde(default)]
    pub user_context: Option<UserContextId>,
    /// Descendants when traversal depth allows. Absent or `null` on the
    /// wire — chromedriver sends `children: null` for the
    /// `contextCreated` event when there are no descendants yet.
    #[serde(default, deserialize_with = "null_to_default")]
    pub children: Vec<BrowsingContextInfo>,
    /// Current document URL.
    pub url: String,
    /// Whether this context is the active OS-level window.
    #[serde(default)]
    pub original_opener: Option<BrowsingContextId>,
    /// Reflects `client_window` on newer drivers.
    #[serde(default, rename = "clientWindow")]
    pub client_window: Option<String>,
}

/// Deserializer that accepts both an absent field and a literal `null` and
/// produces `T::default()`.
fn null_to_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// `browsingContext.navigate`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Navigate {
    /// Browsing context to navigate.
    pub context: BrowsingContextId,
    /// URL to load.
    pub url: String,
    /// Wait condition. Defaults to `Complete`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<ReadinessState>,
}

impl BidiCommand for Navigate {
    const METHOD: &'static str = "browsingContext.navigate";
    type Returns = NavigateResult;
}

/// Response for [`Navigate`].
#[derive(Debug, Clone, Deserialize)]
pub struct NavigateResult {
    /// Server-assigned navigation id (`None` if `wait: None`).
    pub navigation: Option<NavigationId>,
    /// Final URL after redirects.
    pub url: String,
}

/// `browsingContext.reload`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reload {
    /// Browsing context to reload.
    pub context: BrowsingContextId,
    /// If true, bypass HTTP caches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_cache: Option<bool>,
    /// Wait condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<ReadinessState>,
}

impl BidiCommand for Reload {
    const METHOD: &'static str = "browsingContext.reload";
    type Returns = NavigateResult;
}

/// `browsingContext.create`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    /// Whether to create a tab or window.
    pub r#type: CreateType,
    /// Optional reference context (used as opener / parent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_context: Option<BrowsingContextId>,
    /// Whether to bring the new context to the foreground.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    /// User context to put it in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_context: Option<UserContextId>,
}

impl BidiCommand for Create {
    const METHOD: &'static str = "browsingContext.create";
    type Returns = CreateResult;
}

/// Response for [`Create`].
#[derive(Debug, Clone, Deserialize)]
pub struct CreateResult {
    /// Identifier for the newly created context.
    pub context: BrowsingContextId,
}

/// `browsingContext.close`.
#[derive(Debug, Clone, Serialize)]
pub struct Close {
    /// Browsing context to close.
    pub context: BrowsingContextId,
    /// If true, bypass any `beforeunload` prompt.
    #[serde(rename = "promptUnload", skip_serializing_if = "Option::is_none")]
    pub prompt_unload: Option<bool>,
}

impl BidiCommand for Close {
    const METHOD: &'static str = "browsingContext.close";
    type Returns = Empty;
}

/// `browsingContext.activate`.
#[derive(Debug, Clone, Serialize)]
pub struct Activate {
    /// Browsing context to activate (focus the OS-level window).
    pub context: BrowsingContextId,
}

impl BidiCommand for Activate {
    const METHOD: &'static str = "browsingContext.activate";
    type Returns = Empty;
}

/// `browsingContext.captureScreenshot`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshot {
    /// Browsing context to capture.
    pub context: BrowsingContextId,
    /// `viewport` (default) or `document` for a full-document screenshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<ScreenshotOrigin>,
    /// Output format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ImageFormat>,
}

string_enum! {
    /// Origin coordinate system for [`CaptureScreenshot::origin`].
    pub enum ScreenshotOrigin {
        /// Capture only the visible viewport.
        Viewport = "viewport",
        /// Capture the entire scrolled document.
        Document = "document",
    }
}

/// Image format options for [`CaptureScreenshot::format`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFormat {
    /// Image type — PNG or JPEG.
    pub r#type: ImageFormatType,
    /// JPEG quality, 0.0..=1.0. Ignored for PNG.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<f64>,
}

impl BidiCommand for CaptureScreenshot {
    const METHOD: &'static str = "browsingContext.captureScreenshot";
    type Returns = CaptureScreenshotResult;
}

/// Response for [`CaptureScreenshot`].
#[derive(Debug, Clone, Deserialize)]
pub struct CaptureScreenshotResult {
    /// Base64-encoded image bytes.
    pub data: String,
}

/// `browsingContext.setViewport`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetViewport {
    /// Browsing context to resize.
    pub context: BrowsingContextId,
    /// Pixel viewport. `None` resets to driver default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
    /// Device pixel ratio. `None` keeps current.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_pixel_ratio: Option<f64>,
}

/// Viewport shape used by [`SetViewport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
}

impl BidiCommand for SetViewport {
    const METHOD: &'static str = "browsingContext.setViewport";
    type Returns = Empty;
}

/// `browsingContext.traverseHistory`.
#[derive(Debug, Clone, Serialize)]
pub struct TraverseHistory {
    /// Browsing context to traverse.
    pub context: BrowsingContextId,
    /// Number of steps. Negative goes back; positive goes forward.
    pub delta: i32,
}

impl BidiCommand for TraverseHistory {
    const METHOD: &'static str = "browsingContext.traverseHistory";
    type Returns = Empty;
}

/// `browsingContext.handleUserPrompt`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleUserPrompt {
    /// Browsing context whose dialog should be handled.
    pub context: BrowsingContextId,
    /// Accept the dialog (`true`) or dismiss (`false`). `None` lets the
    /// driver pick the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<bool>,
    /// Text to type for `prompt()` dialogs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_text: Option<String>,
}

impl BidiCommand for HandleUserPrompt {
    const METHOD: &'static str = "browsingContext.handleUserPrompt";
    type Returns = Empty;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events surfaced by the `browsingContext.*` module.
pub(crate) mod events {
    use super::*;

    /// `browsingContext.contextCreated`.
    #[derive(Debug, Clone, Deserialize)]
    pub struct ContextCreated(pub BrowsingContextInfo);

    impl BidiEvent for ContextCreated {
        const METHOD: &'static str = "browsingContext.contextCreated";
    }

    /// `browsingContext.contextDestroyed`.
    #[derive(Debug, Clone, Deserialize)]
    pub struct ContextDestroyed(pub BrowsingContextInfo);

    impl BidiEvent for ContextDestroyed {
        const METHOD: &'static str = "browsingContext.contextDestroyed";
    }

    /// `browsingContext.navigationStarted`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NavigationStarted {
        /// Context the navigation belongs to.
        pub context: BrowsingContextId,
        /// Navigation id (matches [`NavigateResult::navigation`]).
        pub navigation: Option<NavigationId>,
        /// Target URL.
        pub url: String,
        /// Driver-assigned timestamp (ms since unix epoch).
        pub timestamp: u64,
    }

    impl BidiEvent for NavigationStarted {
        const METHOD: &'static str = "browsingContext.navigationStarted";
    }

    /// `browsingContext.load` — page load event.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Load {
        /// Context that finished loading.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// Final URL.
        pub url: String,
        /// Timestamp (ms since unix epoch).
        pub timestamp: u64,
    }

    impl BidiEvent for Load {
        const METHOD: &'static str = "browsingContext.load";
    }

    /// `browsingContext.domContentLoaded`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DomContentLoaded {
        /// Context the event belongs to.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// Final URL.
        pub url: String,
        /// Timestamp (ms since unix epoch).
        pub timestamp: u64,
    }

    impl BidiEvent for DomContentLoaded {
        const METHOD: &'static str = "browsingContext.domContentLoaded";
    }

    /// `browsingContext.fragmentNavigated`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FragmentNavigated {
        /// Context.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// New URL.
        pub url: String,
        /// Timestamp.
        pub timestamp: u64,
    }

    impl BidiEvent for FragmentNavigated {
        const METHOD: &'static str = "browsingContext.fragmentNavigated";
    }

    /// `browsingContext.userPromptOpened`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UserPromptOpened {
        /// Context that opened the prompt.
        pub context: BrowsingContextId,
        /// Dialog kind (`alert`, `confirm`, `prompt`, `beforeunload`).
        #[serde(rename = "type")]
        pub prompt_type: String,
        /// Prompt message.
        pub message: String,
        /// Default value for `prompt()` dialogs.
        #[serde(default)]
        pub default_value: Option<String>,
    }

    impl BidiEvent for UserPromptOpened {
        const METHOD: &'static str = "browsingContext.userPromptOpened";
    }

    /// `browsingContext.userPromptClosed`.
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UserPromptClosed {
        /// Context the prompt was attached to.
        pub context: BrowsingContextId,
        /// Whether the prompt was accepted.
        pub accepted: bool,
        /// Text the user typed (for prompt dialogs).
        #[serde(default)]
        pub user_text: Option<String>,
    }

    impl BidiEvent for UserPromptClosed {
        const METHOD: &'static str = "browsingContext.userPromptClosed";
    }
}

/// Module facade returned by [`BiDi::browsing_context`].
#[derive(Debug)]
pub struct BrowsingContextModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> BrowsingContextModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// `browsingContext.getTree` — full tree of all top-level contexts.
    pub async fn get_tree(
        &self,
        root: Option<BrowsingContextId>,
    ) -> Result<GetTreeResult, BidiError> {
        self.bidi
            .send(GetTree {
                max_depth: None,
                root,
            })
            .await
    }

    /// Convenience: id of the first top-level browsing context.
    ///
    /// Wraps `getTree` and returns `tree.contexts[0].context`. Errors if
    /// the driver reports no top-level contexts (a fresh session always
    /// has at least one). Use this when you just want "the active tab"
    /// — most automation scripts want the first top-level context.
    pub async fn top_level(&self) -> Result<BrowsingContextId, BidiError> {
        let tree = self.get_tree(None).await?;
        tree.contexts.into_iter().next().map(|c| c.context).ok_or_else(|| BidiError {
            command: GetTree::METHOD.to_string(),
            error: "unknown error".to_string(),
            message: "browsingContext.getTree returned no top-level contexts".to_string(),
            stacktrace: None,
        })
    }

    /// Convenience: ids of every top-level browsing context.
    ///
    /// Wraps `getTree` and returns each `tree.contexts[*].context`.
    pub async fn top_levels(&self) -> Result<Vec<BrowsingContextId>, BidiError> {
        let tree = self.get_tree(None).await?;
        Ok(tree.contexts.into_iter().map(|c| c.context).collect())
    }

    /// `browsingContext.navigate`.
    pub async fn navigate(
        &self,
        context: BrowsingContextId,
        url: impl Into<String>,
        wait: Option<ReadinessState>,
    ) -> Result<NavigateResult, BidiError> {
        self.bidi
            .send(Navigate {
                context,
                url: url.into(),
                wait,
            })
            .await
    }

    /// `browsingContext.reload`.
    pub async fn reload(
        &self,
        context: BrowsingContextId,
        ignore_cache: bool,
        wait: Option<ReadinessState>,
    ) -> Result<NavigateResult, BidiError> {
        self.bidi
            .send(Reload {
                context,
                ignore_cache: Some(ignore_cache),
                wait,
            })
            .await
    }

    /// `browsingContext.create` — open a new tab or window.
    pub async fn create(&self, kind: CreateType) -> Result<CreateResult, BidiError> {
        self.bidi
            .send(Create {
                r#type: kind,
                reference_context: None,
                background: None,
                user_context: None,
            })
            .await
    }

    /// `browsingContext.close`.
    pub async fn close(&self, context: BrowsingContextId) -> Result<(), BidiError> {
        self.bidi
            .send(Close {
                context,
                prompt_unload: None,
            })
            .await?;
        Ok(())
    }

    /// `browsingContext.activate`.
    pub async fn activate(&self, context: BrowsingContextId) -> Result<(), BidiError> {
        self.bidi
            .send(Activate {
                context,
            })
            .await?;
        Ok(())
    }

    /// `browsingContext.captureScreenshot` — viewport, PNG. Returns
    /// base64-encoded PNG bytes.
    pub async fn capture_screenshot(
        &self,
        context: BrowsingContextId,
    ) -> Result<CaptureScreenshotResult, BidiError> {
        self.bidi
            .send(CaptureScreenshot {
                context,
                origin: Some(ScreenshotOrigin::Viewport),
                format: None,
            })
            .await
    }

    /// `browsingContext.setViewport`.
    pub async fn set_viewport(
        &self,
        context: BrowsingContextId,
        viewport: Option<Viewport>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetViewport {
                context,
                viewport,
                device_pixel_ratio: None,
            })
            .await?;
        Ok(())
    }

    /// `browsingContext.traverseHistory`.
    pub async fn traverse_history(
        &self,
        context: BrowsingContextId,
        delta: i32,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(TraverseHistory {
                context,
                delta,
            })
            .await?;
        Ok(())
    }

    /// `browsingContext.handleUserPrompt`.
    pub async fn handle_user_prompt(
        &self,
        context: BrowsingContextId,
        accept: Option<bool>,
        user_text: Option<String>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(HandleUserPrompt {
                context,
                accept,
                user_text,
            })
            .await?;
        Ok(())
    }
}
