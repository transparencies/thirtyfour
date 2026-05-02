//! `browsingContext.*` — tabs, frames, navigation, history, viewport,
//! screenshots, printing, and locating DOM nodes.
//!
//! A "browsing context" is the spec's name for an HTML navigable: a
//! top-level tab/window or a child frame inside one. Most automation
//! workflows touch this module first — it covers everything from
//! "navigate to this URL" to "render the page as a PDF" to "find the
//! `<button>` matching this CSS selector".
//!
//! See the [W3C `browsingContext` module specification][spec] for the
//! canonical definitions.
//!
//! # Common patterns
//!
//! - Get the active tab id with the [`top_level`][top_level] helper.
//! - Wait for a navigation to complete by passing
//!   [`ReadinessState::Complete`][rs] to [`navigate`][navigate].
//! - Open a new tab with [`create`][create].
//! - Subscribe to lifecycle events
//!   ([`Load`][load], [`DomContentLoaded`][dcl],
//!   [`NavigationStarted`][nav], …) via
//!   [`BiDi::subscribe`](crate::bidi::BiDi::subscribe).
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-browsingContext
//! [top_level]: crate::bidi::modules::browsing_context::BrowsingContextModule::top_level
//! [navigate]: crate::bidi::modules::browsing_context::BrowsingContextModule::navigate
//! [create]: crate::bidi::modules::browsing_context::BrowsingContextModule::create
//! [rs]: crate::bidi::modules::browsing_context::ReadinessState::Complete
//! [load]: crate::bidi::modules::browsing_context::events::Load
//! [dcl]: crate::bidi::modules::browsing_context::events::DomContentLoaded
//! [nav]: crate::bidi::modules::browsing_context::events::NavigationStarted

use serde::{Deserialize, Deserializer, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, BidiEvent, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, NavigationId, UserContextId};
use crate::common::protocol::string_enum;

string_enum! {
    /// Stage of document loading at which a navigation command will return.
    ///
    /// Used by [`Navigate::wait`] and [`Reload::wait`]. Mirrors the spec's
    /// [`browsingContext.ReadinessState`][spec] type.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-ReadinessState
    pub enum ReadinessState {
        /// Resolve as soon as the navigation has been initiated.
        None = "none",
        /// Resolve when the document has been parsed (`document.readyState
        /// == "interactive"`).
        Interactive = "interactive",
        /// Resolve when the `load` event has fired.
        Complete = "complete",
    }
}

string_enum! {
    /// Lifecycle phase used by some events. Mirrors the W3C HTML spec's
    /// document-lifecycle naming.
    pub enum LifecyclePhase {
        /// `load` fired.
        Load = "load",
        /// `DOMContentLoaded` fired.
        DomContentLoaded = "DOMContentLoaded",
    }
}

string_enum! {
    /// New-context creation type — the `type` field on [`Create`].
    pub enum CreateType {
        /// Open as a new tab in the existing window.
        Tab = "tab",
        /// Open as a new top-level window.
        Window = "window",
    }
}

string_enum! {
    /// Image format type — the `type` field on [`ImageFormat`].
    pub enum ImageFormatType {
        /// PNG (lossless).
        Png = "png",
        /// JPEG (lossy; pair with [`ImageFormat::quality`]).
        Jpeg = "jpeg",
    }
}

string_enum! {
    /// History traversal direction. Reserved for future use — current
    /// `traverseHistory` callers pass a signed delta directly.
    pub enum TraversalDirection {
        /// Equivalent to clicking the browser back button.
        Back = "back",
        /// Equivalent to clicking the browser forward button.
        Forward = "forward",
    }
}

/// [`browsingContext.getTree`][spec] — return the tree of browsing contexts.
///
/// With no parameters this returns every top-level context as a separate
/// root. Set [`root`][Self::root] to limit the result to one subtree, or
/// [`max_depth`][Self::max_depth] to limit recursion.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-getTree
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTree {
    /// Maximum traversal depth. `Some(0)` returns just the matched roots
    /// without children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    /// Restrict the tree to descendants of this context. `None` returns
    /// every top-level context.
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
    /// set). Descendants are populated via [`BrowsingContextInfo::children`].
    pub contexts: Vec<BrowsingContextInfo>,
}

/// A node in the browsing-context tree.
///
/// Mirrors the spec's [`browsingContext.Info`][spec] type.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-Info
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowsingContextInfo {
    /// Identifier for this context.
    pub context: BrowsingContextId,
    /// Parent context id. `None` for top-level contexts.
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
    /// The browsing context that opened this one (`window.opener`),
    /// preserved across navigations. `None` when there is no opener.
    #[serde(default)]
    pub original_opener: Option<BrowsingContextId>,
    /// Id of the OS-level client window hosting this context. Match
    /// against [`browser::ClientWindowInfo::client_window`](crate::bidi::modules::browser::ClientWindowInfo::client_window).
    /// May be omitted by older drivers.
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

/// [`browsingContext.navigate`][spec] — navigate the given context to a URL.
///
/// `wait` controls when the call resolves: `None` returns as soon as the
/// navigation has been initiated, while [`ReadinessState::Complete`]
/// waits for the `load` event. The default if omitted is the spec's
/// `committed` state (resolves once the document has been replaced) —
/// some drivers behave like `Complete` instead.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-navigate
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Navigate {
    /// Browsing context to navigate.
    pub context: BrowsingContextId,
    /// Destination URL. May be relative — the spec resolves it against
    /// the document's base URL.
    pub url: String,
    /// Wait condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<ReadinessState>,
}

impl BidiCommand for Navigate {
    const METHOD: &'static str = "browsingContext.navigate";
    type Returns = NavigateResult;
}

/// Response for [`Navigate`] / [`Reload`].
#[derive(Debug, Clone, Deserialize)]
pub struct NavigateResult {
    /// Server-assigned navigation id. `None` if [`Navigate::wait`] was
    /// [`ReadinessState::None`] (no navigation tracked).
    pub navigation: Option<NavigationId>,
    /// Final URL after any redirects.
    pub url: String,
}

/// [`browsingContext.reload`][spec] — reload the active document.
///
/// Behaves identically to [`Navigate`] except that the destination is the
/// current document's URL and the navigation history-handling is `reload`.
/// `ignore_cache: Some(true)` requests a cache bypass (Ctrl+Shift+R
/// equivalent); some drivers don't yet implement that and respond with
/// `unsupported operation`.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-reload
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reload {
    /// Browsing context to reload.
    pub context: BrowsingContextId,
    /// If `true`, bypass HTTP caches.
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

/// [`browsingContext.create`][spec] — open a new tab or window.
///
/// Optional fields:
/// - [`reference_context`][Self::reference_context] sets the opener of
///   the new context (so `window.opener` and the new context's user
///   context inherit from this one).
/// - [`background`][Self::background] keeps the new tab/window behind
///   the current one.
/// - [`user_context`][Self::user_context] places it in a specific user
///   context (incompatible with `reference_context`).
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-create
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    /// Whether to create a tab or a top-level window.
    pub r#type: CreateType,
    /// Reference context — used as opener / parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_context: Option<BrowsingContextId>,
    /// `Some(true)` to keep the new context behind; default is to bring
    /// it to the foreground.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    /// User context to place the new context in. Mutually exclusive with
    /// [`reference_context`][Self::reference_context].
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

/// [`browsingContext.close`][spec] — close a context.
///
/// Closing a top-level context closes its tab (and the window if it was
/// the last tab). Set [`prompt_unload`][Self::prompt_unload] to `true` to
/// honour any `beforeunload` handler; the default is to skip it.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-close
#[derive(Debug, Clone, Serialize)]
pub struct Close {
    /// Browsing context to close.
    pub context: BrowsingContextId,
    /// If `true`, run the `beforeunload` prompt before closing.
    #[serde(rename = "promptUnload", skip_serializing_if = "Option::is_none")]
    pub prompt_unload: Option<bool>,
}

impl BidiCommand for Close {
    const METHOD: &'static str = "browsingContext.close";
    type Returns = Empty;
}

/// [`browsingContext.activate`][spec] — focus a top-level context (its
/// OS-level window).
///
/// Used for tests that depend on the focused state — most automation
/// works without it.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-activate
#[derive(Debug, Clone, Serialize)]
pub struct Activate {
    /// Browsing context to activate.
    pub context: BrowsingContextId,
}

impl BidiCommand for Activate {
    const METHOD: &'static str = "browsingContext.activate";
    type Returns = Empty;
}

/// [`browsingContext.captureScreenshot`][spec] — render the page to a
/// base64-encoded image.
///
/// Defaults to a viewport-sized PNG. For a full-page screenshot pass
/// `origin: Some(ScreenshotOrigin::Document)`. JPEG quality is set via
/// [`ImageFormat::quality`].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-captureScreenshot
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshot {
    /// Browsing context to capture.
    pub context: BrowsingContextId,
    /// `Viewport` (default) — only the visible portion.
    /// `Document` — the entire scrollable document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<ScreenshotOrigin>,
    /// Output format. Defaults to PNG.
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
///
/// `quality` is ignored for PNG and is interpreted as a 0.0..=1.0
/// quality factor for JPEG.
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
    /// Base64-encoded image bytes (PNG bytes start with `iVBOR…`, JPEG
    /// with `/9j/…`).
    pub data: String,
}

/// [`browsingContext.setViewport`][spec] — set viewport dimensions and/or
/// device pixel ratio for a context.
///
/// Pass `viewport: None` to reset to the driver default; the same goes
/// for `device_pixel_ratio`.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-setViewport
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetViewport {
    /// Browsing context to resize.
    pub context: BrowsingContextId,
    /// New viewport size. `None` resets to driver default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
    /// New device pixel ratio. `None` keeps the current value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_pixel_ratio: Option<f64>,
}

/// Viewport size in CSS pixels for [`SetViewport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    /// Width in CSS pixels.
    pub width: u32,
    /// Height in CSS pixels.
    pub height: u32,
}

impl BidiCommand for SetViewport {
    const METHOD: &'static str = "browsingContext.setViewport";
    type Returns = Empty;
}

/// [`browsingContext.traverseHistory`][spec] — back/forward through the
/// session history.
///
/// `delta` is signed: negative goes backwards, positive forwards. Zero
/// reloads the current entry.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-traverseHistory
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

/// [`browsingContext.handleUserPrompt`][spec] — accept or dismiss a
/// JavaScript modal dialog (`alert` / `confirm` / `prompt` /
/// `beforeunload`).
///
/// Pass `accept: Some(true)` to accept, `Some(false)` to dismiss, or
/// `None` to use the driver's default. `user_text` is only meaningful for
/// `prompt()` dialogs.
///
/// Returns `no such alert` if there is no open dialog.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-handleUserPrompt
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleUserPrompt {
    /// Browsing context whose dialog should be handled.
    pub context: BrowsingContextId,
    /// `true` to accept, `false` to dismiss, `None` for driver default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<bool>,
    /// Text to type into a `prompt()` dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_text: Option<String>,
}

impl BidiCommand for HandleUserPrompt {
    const METHOD: &'static str = "browsingContext.handleUserPrompt";
    type Returns = Empty;
}

string_enum! {
    /// Locator strategy for [`Locator::locator_type`]. Mirrors the spec's
    /// [`browsingContext.Locator`][spec] union.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-Locator
    pub enum LocatorType {
        /// Match by accessibility role and/or name.
        Accessibility = "accessibility",
        /// CSS selector.
        Css = "css",
        /// Frame/context locator: find the element hosting another navigable.
        Context = "context",
        /// Inner-text match (case-sensitive by default).
        InnerText = "innerText",
        /// XPath expression.
        XPath = "xpath",
    }
}

string_enum! {
    /// Match type for [`Locator::match_type`] (inner-text locator).
    pub enum InnerTextMatchType {
        /// Whole inner text equals the search value.
        Full = "full",
        /// Search value appears as a substring of inner text.
        Partial = "partial",
    }
}

/// Selector spec passed to [`LocateNodes`]. Mirrors the spec's
/// [`browsingContext.Locator`][spec] union — the constructor functions
/// ([`Locator::css`], [`Locator::xpath`], …) are usually more
/// ergonomic than building the struct by hand.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-Locator
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Locator {
    /// Strategy.
    #[serde(rename = "type")]
    pub locator_type: LocatorType,
    /// Selector value. Shape varies by strategy:
    /// - `Css`, `XPath`, `InnerText` — a JSON string.
    /// - `Accessibility` — an object with optional `role` / `name` keys.
    /// - `Context` — `{ "context": "<browsing-context-id>" }`.
    pub value: serde_json::Value,
    /// Inner-text only — perform a case-insensitive match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_case: Option<bool>,
    /// Inner-text only — full vs partial match (default `Full`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_type: Option<InnerTextMatchType>,
    /// Inner-text only — recursion depth limit (`None` = unbounded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
}

impl Locator {
    /// CSS selector locator.
    pub fn css(selector: impl Into<String>) -> Self {
        Self {
            locator_type: LocatorType::Css,
            value: serde_json::Value::String(selector.into()),
            ignore_case: None,
            match_type: None,
            max_depth: None,
        }
    }

    /// XPath expression locator.
    pub fn xpath(expression: impl Into<String>) -> Self {
        Self {
            locator_type: LocatorType::XPath,
            value: serde_json::Value::String(expression.into()),
            ignore_case: None,
            match_type: None,
            max_depth: None,
        }
    }

    /// Inner-text locator.
    pub fn inner_text(text: impl Into<String>) -> Self {
        Self {
            locator_type: LocatorType::InnerText,
            value: serde_json::Value::String(text.into()),
            ignore_case: None,
            match_type: None,
            max_depth: None,
        }
    }

    /// Accessibility locator (role and/or name).
    pub fn accessibility(role: Option<String>, name: Option<String>) -> Self {
        let mut value = serde_json::Map::new();
        if let Some(role) = role {
            value.insert("role".to_string(), serde_json::Value::String(role));
        }
        if let Some(name) = name {
            value.insert("name".to_string(), serde_json::Value::String(name));
        }
        Self {
            locator_type: LocatorType::Accessibility,
            value: serde_json::Value::Object(value),
            ignore_case: None,
            match_type: None,
            max_depth: None,
        }
    }

    /// Context locator — find the host element for another navigable.
    pub fn context(context: BrowsingContextId) -> Self {
        let mut value = serde_json::Map::new();
        value.insert("context".to_string(), serde_json::Value::String(context.0));
        Self {
            locator_type: LocatorType::Context,
            value: serde_json::Value::Object(value),
            ignore_case: None,
            match_type: None,
            max_depth: None,
        }
    }
}

/// [`browsingContext.locateNodes`][spec] — find every node matching a
/// locator.
///
/// Returns the nodes as serialized [`script.NodeRemoteValue`][node-spec]
/// JSON values (each contains a `sharedId` you can use to address the
/// node from [`script.callFunction`][script-spec] or
/// [`input.setFiles`](crate::bidi::modules::input::SetFiles)).
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-locateNodes
/// [node-spec]: https://w3c.github.io/webdriver-bidi/#type-script-NodeRemoteValue
/// [script-spec]: https://w3c.github.io/webdriver-bidi/#command-script-callFunction
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateNodes {
    /// Browsing context to search.
    pub context: BrowsingContextId,
    /// Locator describing the strategy.
    pub locator: Locator,
    /// Maximum number of nodes to return (`>= 1`). `None` returns every
    /// match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_node_count: Option<u32>,
    /// Custom [`script.SerializationOptions`][opts] for the returned
    /// nodes (controls depth, max length, ownership, …).
    ///
    /// [opts]: https://w3c.github.io/webdriver-bidi/#type-script-SerializationOptions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serialization_options: Option<serde_json::Value>,
    /// Restrict the search to descendants of these
    /// [`script.SharedReference`][shared] nodes.
    ///
    /// [shared]: https://w3c.github.io/webdriver-bidi/#type-script-SharedReference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_nodes: Option<Vec<serde_json::Value>>,
}

impl BidiCommand for LocateNodes {
    const METHOD: &'static str = "browsingContext.locateNodes";
    type Returns = LocateNodesResult;
}

/// Response for [`LocateNodes`].
#[derive(Debug, Clone, Deserialize)]
pub struct LocateNodesResult {
    /// Serialized matching nodes — each is a
    /// [`script.NodeRemoteValue`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-NodeRemoteValue
    pub nodes: Vec<serde_json::Value>,
}

string_enum! {
    /// Page orientation for [`Print::orientation`].
    pub enum PrintOrientation {
        /// Portrait (taller than wide). Default.
        Portrait = "portrait",
        /// Landscape (wider than tall).
        Landscape = "landscape",
    }
}

/// Print margins (in centimeters). Mirrors
/// [`browsingContext.PrintMarginParameters`][spec].
///
/// All four fields default to 1.0 cm if omitted.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-PrintMarginParameters
#[derive(Debug, Clone, Serialize)]
pub struct PrintMargin {
    /// Bottom margin (cm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<f64>,
    /// Left margin (cm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<f64>,
    /// Right margin (cm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<f64>,
    /// Top margin (cm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<f64>,
}

/// Page size in centimeters. Mirrors
/// [`browsingContext.PrintPageParameters`][spec]; defaults are 21.59 cm
/// (8.5 in) wide by 27.94 cm (11 in) tall.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-browsingContext-PrintPageParameters
#[derive(Debug, Clone, Serialize)]
pub struct PrintPage {
    /// Page height (cm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// Page width (cm).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
}

/// [`browsingContext.print`][spec] — render the page as a base64-encoded
/// PDF.
///
/// All formatting fields are optional; the spec defaults are portrait
/// orientation, 1 cm margins, US Letter page size, scale 1.0, no
/// background colors, and shrink-to-fit enabled.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-print
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Print {
    /// Browsing context to print.
    pub context: BrowsingContextId,
    /// Include background colors and images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    /// Page margins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<PrintMargin>,
    /// Page orientation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<PrintOrientation>,
    /// Page size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<PrintPage>,
    /// Page ranges to include. Each entry is either a 1-based page
    /// number (JSON integer) or a `"start-end"` string (JSON string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_ranges: Option<Vec<serde_json::Value>>,
    /// Zoom factor (0.1..=2.0; default 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Shrink content to fit the page width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shrink_to_fit: Option<bool>,
}

impl BidiCommand for Print {
    const METHOD: &'static str = "browsingContext.print";
    type Returns = PrintResult;
}

/// Response for [`Print`].
#[derive(Debug, Clone, Deserialize)]
pub struct PrintResult {
    /// Base64-encoded PDF bytes (start with `JVBERi…`).
    pub data: String,
}

/// [`browsingContext.setBypassCSP`][spec] — toggle Content-Security-Policy
/// enforcement.
///
/// When CSP bypass is on, `eval()`, `new Function()`, inline scripts, and
/// resource loads that CSP would normally block are allowed. The wire
/// shape only accepts `true` (enable bypass) or `null` (clear the
/// override) — we model that as `Option<bool>` and silently omit `false`.
///
/// Use [`contexts`][Self::contexts] / [`user_contexts`][Self::user_contexts]
/// to scope the override; an unscoped call applies the default.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-setBypassCSP
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBypassCsp {
    /// `Some(true)` to bypass CSP, `None` to clear the override. The
    /// wire only accepts `true` or `null`; we always omit `false`.
    #[serde(serialize_with = "serialize_bypass")]
    pub bypass: Option<bool>,
    /// Restrict to specific browsing contexts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// Restrict to specific user contexts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

fn serialize_bypass<S: serde::Serializer>(
    value: &Option<bool>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(true) => serializer.serialize_bool(true),
        _ => serializer.serialize_none(),
    }
}

impl BidiCommand for SetBypassCsp {
    const METHOD: &'static str = "browsingContext.setBypassCSP";
    type Returns = Empty;
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events surfaced by the `browsingContext.*` module.
pub(crate) mod events {
    use super::*;

    /// [`browsingContext.contextCreated`][spec] — a new tab, window, or
    /// frame has been created.
    ///
    /// The wrapped [`BrowsingContextInfo`] carries the new context's id,
    /// parent (for iframes), URL, and user context.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-contextCreated
    #[derive(Debug, Clone, Deserialize)]
    pub struct ContextCreated(pub BrowsingContextInfo);

    impl BidiEvent for ContextCreated {
        const METHOD: &'static str = "browsingContext.contextCreated";
    }

    /// [`browsingContext.contextDestroyed`][spec] — a context has been
    /// torn down (tab closed, frame removed, etc.).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-contextDestroyed
    #[derive(Debug, Clone, Deserialize)]
    pub struct ContextDestroyed(pub BrowsingContextInfo);

    impl BidiEvent for ContextDestroyed {
        const METHOD: &'static str = "browsingContext.contextDestroyed";
    }

    /// [`browsingContext.navigationStarted`][spec] — a navigation has
    /// been initiated (before any network traffic).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-navigationStarted
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

    /// [`browsingContext.load`][spec] — the document's `load` event has
    /// fired.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-load
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

    /// [`browsingContext.domContentLoaded`][spec] — the document's
    /// `DOMContentLoaded` event has fired.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-domContentLoaded
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

    /// [`browsingContext.fragmentNavigated`][spec] — the URL fragment
    /// (`#…`) changed without a full navigation.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-fragmentNavigated
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FragmentNavigated {
        /// Context.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// New URL (including the new fragment).
        pub url: String,
        /// Timestamp.
        pub timestamp: u64,
    }

    impl BidiEvent for FragmentNavigated {
        const METHOD: &'static str = "browsingContext.fragmentNavigated";
    }

    /// [`browsingContext.userPromptOpened`][spec] — a JS dialog
    /// (`alert`/`confirm`/`prompt`/`beforeunload`) has opened.
    ///
    /// Pair with [`HandleUserPrompt`] (or
    /// [`BrowsingContextModule::handle_user_prompt`]) to drive it.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-userPromptOpened
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

    /// [`browsingContext.userPromptClosed`][spec] — a JS dialog has been
    /// dismissed or accepted.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-userPromptClosed
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UserPromptClosed {
        /// Context the prompt was attached to.
        pub context: BrowsingContextId,
        /// Whether the prompt was accepted.
        pub accepted: bool,
        /// Text the user typed (for `prompt()` dialogs only).
        #[serde(default)]
        pub user_text: Option<String>,
    }

    impl BidiEvent for UserPromptClosed {
        const METHOD: &'static str = "browsingContext.userPromptClosed";
    }

    /// [`browsingContext.historyUpdated`][spec] — `history.pushState` or
    /// `history.replaceState` has run.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-historyUpdated
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HistoryUpdated {
        /// Context.
        pub context: BrowsingContextId,
        /// Updated URL.
        pub url: String,
        /// Driver timestamp (ms since epoch).
        pub timestamp: u64,
        /// User context.
        #[serde(default)]
        pub user_context: Option<UserContextId>,
    }

    impl BidiEvent for HistoryUpdated {
        const METHOD: &'static str = "browsingContext.historyUpdated";
    }

    /// [`browsingContext.downloadWillBegin`][spec] — fired just before a
    /// download starts.
    ///
    /// Pair with
    /// [`browser.setDownloadBehavior`](crate::bidi::modules::browser::SetDownloadBehavior)
    /// to control whether the download proceeds and where it's saved.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-downloadWillBegin
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DownloadWillBegin {
        /// Context that triggered the download.
        pub context: BrowsingContextId,
        /// Navigation id (the download is associated with the navigation
        /// request).
        pub navigation: Option<NavigationId>,
        /// Source URL.
        pub url: String,
        /// Driver timestamp (ms since epoch).
        pub timestamp: u64,
        /// User context.
        #[serde(default)]
        pub user_context: Option<UserContextId>,
        /// Browser-suggested filename.
        pub suggested_filename: String,
    }

    impl BidiEvent for DownloadWillBegin {
        const METHOD: &'static str = "browsingContext.downloadWillBegin";
    }

    /// [`browsingContext.downloadEnd`][spec] — fired when a download
    /// finishes (`Complete`) or is canceled (`Canceled`).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-downloadEnd
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DownloadEnd {
        /// Final status.
        pub status: DownloadEndStatus,
        /// Where the file was saved. Only meaningful when
        /// `status == Complete`; even then it can be `None` if the
        /// driver doesn't expose the path.
        #[serde(default)]
        pub filepath: Option<String>,
        /// Context that triggered the download.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// Source URL.
        pub url: String,
        /// Driver timestamp (ms since epoch).
        pub timestamp: u64,
        /// User context.
        #[serde(default)]
        pub user_context: Option<UserContextId>,
    }

    impl BidiEvent for DownloadEnd {
        const METHOD: &'static str = "browsingContext.downloadEnd";
    }

    /// [`browsingContext.navigationAborted`][spec] — the navigation was
    /// abandoned (e.g. another navigation took over).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-navigationAborted
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NavigationAborted {
        /// Context.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// URL.
        pub url: String,
        /// Timestamp.
        pub timestamp: u64,
        /// User context.
        #[serde(default)]
        pub user_context: Option<UserContextId>,
    }

    impl BidiEvent for NavigationAborted {
        const METHOD: &'static str = "browsingContext.navigationAborted";
    }

    /// [`browsingContext.navigationCommitted`][spec] — the navigation
    /// has reached the "committed" state (the new document has been
    /// installed; equivalent to the spec's `committed` ready state).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-navigationCommitted
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NavigationCommitted {
        /// Context.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// URL.
        pub url: String,
        /// Timestamp.
        pub timestamp: u64,
        /// User context.
        #[serde(default)]
        pub user_context: Option<UserContextId>,
    }

    impl BidiEvent for NavigationCommitted {
        const METHOD: &'static str = "browsingContext.navigationCommitted";
    }

    /// [`browsingContext.navigationFailed`][spec] — the navigation
    /// failed (network error, blocked, etc.).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-navigationFailed
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NavigationFailed {
        /// Context.
        pub context: BrowsingContextId,
        /// Navigation id.
        pub navigation: Option<NavigationId>,
        /// URL.
        pub url: String,
        /// Timestamp.
        pub timestamp: u64,
        /// User context.
        #[serde(default)]
        pub user_context: Option<UserContextId>,
    }

    impl BidiEvent for NavigationFailed {
        const METHOD: &'static str = "browsingContext.navigationFailed";
    }
}

string_enum! {
    /// Status field of [`events::DownloadEnd`].
    pub enum DownloadEndStatus {
        /// The download finished successfully.
        Complete = "complete",
        /// The download was canceled.
        Canceled = "canceled",
    }
}

/// Convenience facade for the `browsingContext.*` module.
///
/// Returned by [`BiDi::browsing_context`](crate::bidi::BiDi::browsing_context).
/// Methods cover the common, unscoped form of each command — for finer
/// control (per-context CSP bypass, multi-page-range printing, etc.)
/// build the command struct directly and send it via
/// [`BiDi::send`](crate::bidi::BiDi::send).
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

    /// Run [`browsingContext.getTree`][spec] — return every top-level
    /// context (and their descendants), or just the subtree rooted at
    /// `root` if supplied.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-getTree
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

    /// Return the id of the first top-level browsing context — i.e. "the
    /// active tab".
    ///
    /// Implemented in terms of [`get_tree`](Self::get_tree). Errors with
    /// a synthetic [`BidiError`] if the driver reports no top-level
    /// contexts (which never happens in a healthy session).
    pub async fn top_level(&self) -> Result<BrowsingContextId, BidiError> {
        let tree = self.get_tree(None).await?;
        tree.contexts.into_iter().next().map(|c| c.context).ok_or_else(|| BidiError {
            command: GetTree::METHOD.to_string(),
            error: "unknown error".to_string(),
            message: "browsingContext.getTree returned no top-level contexts".to_string(),
            stacktrace: None,
        })
    }

    /// Return the ids of every top-level browsing context.
    ///
    /// Implemented in terms of [`get_tree`](Self::get_tree).
    pub async fn top_levels(&self) -> Result<Vec<BrowsingContextId>, BidiError> {
        let tree = self.get_tree(None).await?;
        Ok(tree.contexts.into_iter().map(|c| c.context).collect())
    }

    /// Navigate `context` to `url` via [`browsingContext.navigate`][spec].
    ///
    /// `wait` controls when the call resolves; see [`ReadinessState`].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-navigate
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

    /// Reload `context` via [`browsingContext.reload`][spec].
    ///
    /// `ignore_cache: true` requests a cache bypass — some drivers
    /// (currently geckodriver) respond with `unsupported operation`.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-reload
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

    /// Open a new tab or window via [`browsingContext.create`][spec].
    ///
    /// For per-create options (opener, user context, background) build a
    /// [`Create`] directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-create
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

    /// Close a context via [`browsingContext.close`][spec], skipping any
    /// `beforeunload` prompt.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-close
    pub async fn close(&self, context: BrowsingContextId) -> Result<(), BidiError> {
        self.bidi
            .send(Close {
                context,
                prompt_unload: None,
            })
            .await?;
        Ok(())
    }

    /// Focus a context via [`browsingContext.activate`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-activate
    pub async fn activate(&self, context: BrowsingContextId) -> Result<(), BidiError> {
        self.bidi
            .send(Activate {
                context,
            })
            .await?;
        Ok(())
    }

    /// Capture a viewport-sized PNG screenshot of `context`.
    ///
    /// Wraps [`browsingContext.captureScreenshot`][spec]; see
    /// [`CaptureScreenshot`] to capture the entire scrolled document or
    /// to render JPEG.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-captureScreenshot
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

    /// Set the viewport of `context` via
    /// [`browsingContext.setViewport`][spec]. Pass `None` to reset to
    /// the driver default.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-setViewport
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

    /// Step through `context`'s session history via
    /// [`browsingContext.traverseHistory`][spec]. `delta` is signed.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-traverseHistory
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

    /// Accept or dismiss a JS dialog via
    /// [`browsingContext.handleUserPrompt`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-handleUserPrompt
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

    /// Find every node in `context` matching `locator` via
    /// [`browsingContext.locateNodes`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-locateNodes
    pub async fn locate_nodes(
        &self,
        context: BrowsingContextId,
        locator: Locator,
    ) -> Result<LocateNodesResult, BidiError> {
        self.bidi
            .send(LocateNodes {
                context,
                locator,
                max_node_count: None,
                serialization_options: None,
                start_nodes: None,
            })
            .await
    }

    /// Render `context` as a base64-encoded PDF using the driver's
    /// default page settings. Wraps [`browsingContext.print`][spec].
    ///
    /// For custom page size, margins, scale, or page ranges build a
    /// [`Print`] directly and send it via
    /// [`BiDi::send`](crate::bidi::BiDi::send).
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-print
    pub async fn print(&self, context: BrowsingContextId) -> Result<PrintResult, BidiError> {
        self.bidi
            .send(Print {
                context,
                background: None,
                margin: None,
                orientation: None,
                page: None,
                page_ranges: None,
                scale: None,
                shrink_to_fit: None,
            })
            .await
    }

    /// Toggle global Content-Security-Policy bypass via
    /// [`browsingContext.setBypassCSP`][spec].
    ///
    /// `Some(true)` enables bypass, `None` clears the override. To scope
    /// the override to specific contexts/user-contexts, build the
    /// [`SetBypassCsp`] struct directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-setBypassCSP
    pub async fn set_bypass_csp(&self, bypass: Option<bool>) -> Result<(), BidiError> {
        self.bidi
            .send(SetBypassCsp {
                bypass,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }
}
