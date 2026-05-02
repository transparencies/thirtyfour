//! `input.*` — synthesise pointer / key / wheel input and drive
//! `<input type=file>` dialogs.
//!
//! The action format is identical to W3C WebDriver Classic's
//! [Actions API][actions] — each entry in
//! [`PerformActions::actions`][pa] describes one input source (a
//! keyboard, a pointer, or a wheel) and its sequence of actions. The
//! Rust API treats each source as a [`serde_json::Value`] so callers
//! can build them with the `json!` macro, deserialize them from JSON
//! test fixtures, or wrap them in their own typed helpers.
//!
//! [pa]: crate::bidi::modules::input::PerformActions::actions
//!
//! See the [W3C `input` module specification][spec] for the canonical
//! definitions, including the
//! [`input.SourceActions`][source-actions-spec] union the
//! action-source entries follow.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-input
//! [actions]: https://www.w3.org/TR/webdriver2/#actions
//! [source-actions-spec]: https://w3c.github.io/webdriver-bidi/#type-input-SourceActions

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, BidiEvent, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, NodeId, UserContextId};

/// [`input.performActions`][spec] — execute a batch of input actions.
///
/// `actions` is an array of [`input.SourceActions`][src] JSON objects.
/// Each source has an `id`, a `type` (`"key"`, `"pointer"`, `"wheel"`,
/// or `"none"`), optional per-source `parameters`, and an array of
/// individual actions to perform on that source.
///
/// Example: type "hi" through a synthesized keyboard.
///
/// ```no_run
/// # use thirtyfour::prelude::*;
/// # use thirtyfour::bidi::modules::input::PerformActions;
/// # use serde_json::json;
/// # async fn run(bidi: thirtyfour::bidi::BiDi, ctx: thirtyfour::bidi::BrowsingContextId)
/// # -> WebDriverResult<()> {
/// bidi.send(PerformActions {
///     context: ctx,
///     actions: vec![json!({
///         "id": "kbd",
///         "type": "key",
///         "actions": [
///             {"type": "keyDown", "value": "h"},
///             {"type": "keyUp",   "value": "h"},
///             {"type": "keyDown", "value": "i"},
///             {"type": "keyUp",   "value": "i"},
///         ]
///     })],
/// }).await?;
/// # Ok(()) }
/// ```
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-input-performActions
/// [src]: https://w3c.github.io/webdriver-bidi/#type-input-SourceActions
#[derive(Debug, Clone, Serialize)]
pub struct PerformActions {
    /// Browsing context the actions apply to.
    pub context: BrowsingContextId,
    /// One entry per input source. See [`input.SourceActions`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-input-SourceActions
    pub actions: Vec<serde_json::Value>,
}

impl BidiCommand for PerformActions {
    const METHOD: &'static str = "input.performActions";
    type Returns = Empty;
}

/// [`input.releaseActions`][spec] — release any sticky input state
/// (held modifier keys, depressed buttons) for `context`.
///
/// Useful as cleanup between tests so that a leftover keydown from one
/// scenario doesn't bleed into the next.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-input-releaseActions
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseActions {
    /// Browsing context to release.
    pub context: BrowsingContextId,
}

impl BidiCommand for ReleaseActions {
    const METHOD: &'static str = "input.releaseActions";
    type Returns = Empty;
}

/// [`input.setFiles`][spec] — set the file list on an
/// `<input type=file>` element.
///
/// Construct the [`element`][Self::element] reference with
/// [`shared_reference`] from a [`NodeId`] returned by
/// [`script.callFunction`](crate::bidi::modules::script::CallFunction)
/// or
/// [`browsingContext.locateNodes`](crate::bidi::modules::browsing_context::LocateNodes).
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-input-setFiles
#[derive(Debug, Clone, Serialize)]
pub struct SetFiles {
    /// Browsing context.
    pub context: BrowsingContextId,
    /// Element to set files on — a [`script.SharedReference`][spec]
    /// JSON value (`{"sharedId":"…"}`). Use [`shared_reference`] to
    /// build one from a [`NodeId`].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-SharedReference
    pub element: serde_json::Value,
    /// File paths to attach. Paths must be absolute on the host
    /// filesystem; the spec doesn't define remote-driver upload — for
    /// Selenium Grid scenarios upload the file to the grid first via
    /// the WebDriver Classic file-upload endpoint.
    pub files: Vec<String>,
}

/// Wrap a [`NodeId`] in the [`script.SharedReference`][spec] JSON shape
/// expected by [`SetFiles::element`].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-SharedReference
pub fn shared_reference(node: &NodeId) -> serde_json::Value {
    serde_json::json!({"sharedId": node.as_str()})
}

impl BidiCommand for SetFiles {
    const METHOD: &'static str = "input.setFiles";
    type Returns = Empty;
}

/// Convenience facade for the `input.*` module.
///
/// Returned by [`BiDi::input`](crate::bidi::BiDi::input).
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

    /// Execute a batch of input actions via
    /// [`input.performActions`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-input-performActions
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

    /// Release any sticky input state via
    /// [`input.releaseActions`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-input-releaseActions
    pub async fn release_actions(&self, context: BrowsingContextId) -> Result<(), BidiError> {
        self.bidi
            .send(ReleaseActions {
                context,
            })
            .await?;
        Ok(())
    }

    /// Set the files on an `<input type=file>` element via
    /// [`input.setFiles`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-input-setFiles
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

/// Events surfaced by the `input.*` module.
pub(crate) mod events {
    use super::*;

    /// [`input.fileDialogOpened`][spec] — fires when the browser opens
    /// a native file picker (e.g. when the user clicks an
    /// `<input type=file>`).
    ///
    /// Pair with [`SetFiles`] to programmatically supply files —
    /// natively-opened pickers can't be driven by simulated clicks
    /// alone.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#event-input-fileDialogOpened
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FileDialogOpened {
        /// Browsing context that triggered the dialog.
        pub context: BrowsingContextId,
        /// Whether the input accepts multiple files (the `multiple`
        /// HTML attribute).
        pub multiple: bool,
        /// User context id of the originating context.
        #[serde(default)]
        pub user_context: Option<UserContextId>,
        /// The `<input>` element that opened the dialog, as a
        /// [`script.SharedReference`][spec] JSON value. May be absent
        /// when the dialog wasn't directly triggered by a DOM element.
        ///
        /// [spec]: https://w3c.github.io/webdriver-bidi/#type-script-SharedReference
        #[serde(default)]
        pub element: Option<serde_json::Value>,
    }

    impl BidiEvent for FileDialogOpened {
        const METHOD: &'static str = "input.fileDialogOpened";
    }
}
