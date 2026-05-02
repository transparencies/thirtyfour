//! `browser.*` — browser-wide control: shutdown, user contexts, OS-level
//! windows, and download behavior.
//!
//! "Browser" here means the running browser instance, not an individual
//! tab. Use this module to:
//!
//! - End the entire browser ([`Close`][close]).
//! - Manage [user contexts][user-context-spec] — BiDi's incognito-like
//!   isolation primitive ([`CreateUserContext`][cuc],
//!   [`GetUserContexts`][guc], [`RemoveUserContext`][ruc]).
//! - Inspect and manipulate OS-level browser windows
//!   ([`GetClientWindows`][gcw], [`SetClientWindowState`][scws]).
//! - Control how downloads are handled
//!   ([`SetDownloadBehavior`][sdb]).
//!
//! See the [W3C `browser` module specification][spec] for the canonical
//! definitions.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-browser
//! [user-context-spec]: https://w3c.github.io/webdriver-bidi/#user-contexts
//! [close]: crate::bidi::modules::browser::Close
//! [cuc]: crate::bidi::modules::browser::CreateUserContext
//! [guc]: crate::bidi::modules::browser::GetUserContexts
//! [ruc]: crate::bidi::modules::browser::RemoveUserContext
//! [gcw]: crate::bidi::modules::browser::GetClientWindows
//! [scws]: crate::bidi::modules::browser::SetClientWindowState
//! [sdb]: crate::bidi::modules::browser::SetDownloadBehavior

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{ClientWindowId, UserContextId};
use crate::common::protocol::string_enum;

/// [`browser.close`][spec] — terminate every WebDriver session and shut
/// the browser process down.
///
/// After this command returns, the browser process exits and all
/// associated tabs/windows close without prompting to unload. The
/// behaviour when multiple WebDriver sessions are connected to the same
/// browser is implementation-defined — see the spec.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-close
#[derive(Debug, Clone, Default, Serialize)]
pub struct Close;

impl BidiCommand for Close {
    const METHOD: &'static str = "browser.close";
    type Returns = Empty;
}

/// [`browser.createUserContext`][spec] — open a new
/// [user context][user-context-spec] (an isolated cookie / storage jar,
/// roughly equivalent to a Chrome profile or a private-browsing window).
///
/// Per-user-context overrides for `acceptInsecureCerts`, proxy
/// configuration, and unhandled-prompt behavior may be supplied; if
/// omitted the session-level defaults apply.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-createUserContext
/// [user-context-spec]: https://w3c.github.io/webdriver-bidi/#user-contexts
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserContext {
    /// Override the session's accept-insecure-certs flag for this user
    /// context. Returns `unsupported operation` on drivers that can't
    /// scope TLS handling per user context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_insecure_certs: Option<bool>,
    /// Per-user-context [`session.ProxyConfiguration`][proxy] (passed
    /// through as JSON). Returns `unsupported operation` when not
    /// supported by the driver.
    ///
    /// [proxy]: https://w3c.github.io/webdriver-bidi/#type-session-ProxyConfiguration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<serde_json::Value>,
    /// Per-user-context [`session.UserPromptHandler`][handler] override.
    ///
    /// [handler]: https://w3c.github.io/webdriver-bidi/#type-session-UserPromptHandler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unhandled_prompt_behavior: Option<serde_json::Value>,
}

impl BidiCommand for CreateUserContext {
    const METHOD: &'static str = "browser.createUserContext";
    type Returns = UserContextInfo;
}

/// One user context. See [`type-browser-UserContextInfo`][spec].
///
/// Returned by [`CreateUserContext`] and as elements of
/// [`GetUserContextsResult::user_contexts`].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-browser-UserContextInfo
#[derive(Debug, Clone, Deserialize)]
pub struct UserContextInfo {
    /// User context id. The default user context is always called
    /// `"default"` and cannot be removed.
    #[serde(rename = "userContext")]
    pub user_context: UserContextId,
}

/// [`browser.getUserContexts`][spec] — list every known user context.
///
/// The default user context is always present in the result.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-getUserContexts
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetUserContexts;

impl BidiCommand for GetUserContexts {
    const METHOD: &'static str = "browser.getUserContexts";
    type Returns = GetUserContextsResult;
}

/// Response for [`GetUserContexts`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetUserContextsResult {
    /// All user contexts, including `"default"`.
    #[serde(rename = "userContexts")]
    pub user_contexts: Vec<UserContextInfo>,
}

/// [`browser.removeUserContext`][spec] — close a user context, including
/// every navigable inside it (without firing `beforeunload`).
///
/// The default user context cannot be removed; passing it returns
/// `invalid argument`.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-removeUserContext
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveUserContext {
    /// User context to remove.
    pub user_context: UserContextId,
}

impl BidiCommand for RemoveUserContext {
    const METHOD: &'static str = "browser.removeUserContext";
    type Returns = Empty;
}

string_enum! {
    /// Window state. Used by [`ClientWindowInfo::state`] and as a
    /// parameter to [`SetClientWindowState::state`].
    ///
    /// See [`type-browser-ClientWindowInfo`][spec] in the spec.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-browser-ClientWindowInfo
    pub enum ClientWindowState {
        /// Fills the entire screen with no chrome.
        Fullscreen = "fullscreen",
        /// Maximised within the OS desktop.
        Maximized = "maximized",
        /// Minimised to the dock/taskbar.
        Minimized = "minimized",
        /// Normal (windowed) state. Allows custom width/height/x/y.
        Normal = "normal",
    }
}

/// Properties of an OS-level browser window.
///
/// Mirrors the spec's [`browser.ClientWindowInfo`][spec] type.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-browser-ClientWindowInfo
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientWindowInfo {
    /// Whether this window currently receives keyboard input from the OS.
    /// Note that this is OS focus, not BiDi document focus — the active
    /// document of an inactive window can still be queried.
    pub active: bool,
    /// Stable window id. Use it with [`SetClientWindowState`].
    pub client_window: ClientWindowId,
    /// Window height in CSS pixels.
    pub height: u32,
    /// Window width in CSS pixels.
    pub width: u32,
    /// Window x-coordinate in screen pixels.
    pub x: i32,
    /// Window y-coordinate in screen pixels.
    pub y: i32,
    /// Current window state.
    pub state: ClientWindowState,
}

/// [`browser.getClientWindows`][spec] — enumerate every OS-level browser
/// window.
///
/// One window can host many tabs (top-level browsing contexts); use
/// [`browsing_context::GetTree`](crate::bidi::modules::browsing_context::GetTree)
/// to list those.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-getClientWindows
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetClientWindows;

impl BidiCommand for GetClientWindows {
    const METHOD: &'static str = "browser.getClientWindows";
    type Returns = GetClientWindowsResult;
}

/// Response for [`GetClientWindows`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetClientWindowsResult {
    /// All client windows.
    #[serde(rename = "clientWindows")]
    pub client_windows: Vec<ClientWindowInfo>,
}

/// [`browser.setClientWindowState`][spec] — change a window's state and
/// (when `state == Normal`) optionally its position/size.
///
/// `width`/`height`/`x`/`y` are only respected when `state` is
/// [`ClientWindowState::Normal`]; for `Maximized`, `Minimized`, or
/// `Fullscreen` they are ignored. The driver returns the (possibly
/// adjusted) post-transition window info as a [`ClientWindowInfo`].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-setClientWindowState
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetClientWindowState {
    /// Window to update.
    pub client_window: ClientWindowId,
    /// New state.
    pub state: ClientWindowState,
    /// New width in CSS pixels (only respected when `state == Normal`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// New height in CSS pixels (only respected when `state == Normal`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// New x-coordinate (only respected when `state == Normal`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// New y-coordinate (only respected when `state == Normal`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
}

impl BidiCommand for SetClientWindowState {
    const METHOD: &'static str = "browser.setClientWindowState";
    type Returns = ClientWindowInfo;
}

/// Download behaviour for [`SetDownloadBehavior::download_behavior`].
///
/// Matches the spec's `browser.DownloadBehavior` union of `"allowed"` and
/// `"denied"`.
///
/// See [`SetDownloadBehavior`] for usage.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DownloadBehavior {
    /// Allow downloads, saving each file to `destination_folder`.
    Allowed {
        /// Destination folder on the host filesystem.
        #[serde(rename = "destinationFolder")]
        destination_folder: String,
    },
    /// Refuse all downloads. The browser cancels each one.
    Denied,
}

/// [`browser.setDownloadBehavior`][spec] — configure how the browser
/// handles downloads, either globally or per user context.
///
/// Pass `download_behavior: None` to clear any previously-set override.
/// `user_contexts: None` (or omitted) sets the default behaviour;
/// `user_contexts: Some(ids)` scopes the override to those contexts.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-setDownloadBehavior
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDownloadBehavior {
    /// New behaviour, or `None` to clear any prior override.
    pub download_behavior: Option<DownloadBehavior>,
    /// Restrict to these user contexts. `None` / empty → set the global
    /// default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetDownloadBehavior {
    const METHOD: &'static str = "browser.setDownloadBehavior";
    type Returns = Empty;
}

/// Convenience facade for the `browser.*` module.
///
/// Returned by [`BiDi::browser`](crate::bidi::BiDi::browser). Methods on
/// this facade cover the common, unscoped form of each command. For
/// scoping (e.g. setting download behaviour for one user context only)
/// build the command struct directly and send it via
/// [`BiDi::send`](crate::bidi::BiDi::send).
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

    /// Run [`browser.close`][spec] — terminate every WebDriver session
    /// and shut the browser process down.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-close
    pub async fn close(&self) -> Result<(), BidiError> {
        self.bidi.send(Close).await?;
        Ok(())
    }

    /// Create a new user context with default settings via
    /// [`browser.createUserContext`][spec].
    ///
    /// For per-context proxy / TLS / prompt-handler overrides, build the
    /// [`CreateUserContext`] struct directly.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-createUserContext
    pub async fn create_user_context(&self) -> Result<UserContextInfo, BidiError> {
        self.bidi.send(CreateUserContext::default()).await
    }

    /// List every user context via [`browser.getUserContexts`][spec].
    ///
    /// The default user context is always included in the result.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-getUserContexts
    pub async fn get_user_contexts(&self) -> Result<GetUserContextsResult, BidiError> {
        self.bidi.send(GetUserContexts).await
    }

    /// Remove a user context via [`browser.removeUserContext`][spec].
    ///
    /// All top-level traversables inside the context are closed without
    /// firing `beforeunload`. Returns `invalid argument` if `user_context`
    /// is the default user context, and `no such user context` if it
    /// doesn't exist.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-removeUserContext
    pub async fn remove_user_context(&self, user_context: UserContextId) -> Result<(), BidiError> {
        self.bidi
            .send(RemoveUserContext {
                user_context,
            })
            .await?;
        Ok(())
    }

    /// List every OS-level browser window via
    /// [`browser.getClientWindows`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-getClientWindows
    pub async fn get_client_windows(&self) -> Result<GetClientWindowsResult, BidiError> {
        self.bidi.send(GetClientWindows).await
    }

    /// Change a window's state via
    /// [`browser.setClientWindowState`][spec], without changing position
    /// or size.
    ///
    /// To resize or reposition a window in [`ClientWindowState::Normal`],
    /// build the [`SetClientWindowState`] struct directly and supply
    /// `width` / `height` / `x` / `y`.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-setClientWindowState
    pub async fn set_client_window_state(
        &self,
        client_window: ClientWindowId,
        state: ClientWindowState,
    ) -> Result<ClientWindowInfo, BidiError> {
        self.bidi
            .send(SetClientWindowState {
                client_window,
                state,
                width: None,
                height: None,
                x: None,
                y: None,
            })
            .await
    }

    /// Configure the global download behaviour via
    /// [`browser.setDownloadBehavior`][spec].
    ///
    /// Pass `None` to clear any prior override. To scope the override to
    /// specific user contexts, build the [`SetDownloadBehavior`] struct
    /// directly and supply [`user_contexts`][SetDownloadBehavior::user_contexts].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-browser-setDownloadBehavior
    pub async fn set_download_behavior(
        &self,
        download_behavior: Option<DownloadBehavior>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetDownloadBehavior {
                download_behavior,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }
}
