//! `Input` domain — synthetic keyboard, mouse, and text input events.

use serde::Serialize;

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::common::protocol::string_enum;
use crate::error::WebDriverResult;

string_enum! {
    /// Type of synthetic keyboard event for [`DispatchKeyEvent`].
    pub enum KeyEventType {
        /// Key pressed down.
        KeyDown = "keyDown",
        /// Key released.
        KeyUp = "keyUp",
        /// Raw key down (skips the OS-level translation step).
        RawKeyDown = "rawKeyDown",
        /// Inserts a character (e.g. for IME).
        Char = "char",
    }
}

string_enum! {
    /// Type of synthetic mouse event for [`DispatchMouseEvent`].
    pub enum MouseEventType {
        /// Mouse button pressed.
        MousePressed = "mousePressed",
        /// Mouse button released.
        MouseReleased = "mouseReleased",
        /// Mouse moved.
        MouseMoved = "mouseMoved",
        /// Mouse wheel scrolled.
        MouseWheel = "mouseWheel",
    }
}

string_enum! {
    /// Mouse button identifier for [`DispatchMouseEvent`].
    pub enum MouseButton {
        /// No button.
        None = "none",
        /// Left button.
        Left = "left",
        /// Middle button.
        Middle = "middle",
        /// Right button.
        Right = "right",
        /// Back button.
        Back = "back",
        /// Forward button.
        Forward = "forward",
    }
}

/// `Input.dispatchKeyEvent`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchKeyEvent {
    /// Event phase.
    pub r#type: KeyEventType,
    /// Bit mask of [Modifiers](https://chromedevtools.github.io/devtools-protocol/tot/Input/#method-dispatchKeyEvent).
    /// 1=Alt, 2=Ctrl, 4=Meta, 8=Shift.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<u32>,
    /// Text the key represents (for `"char"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Unmodified text (e.g. `"a"` instead of `"A"` if shift is pressed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmodified_text: Option<String>,
    /// Key identifier (e.g. `"Enter"`, `"a"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// `code` value per <https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values> (e.g. `"KeyA"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Native virtual key code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_virtual_key_code: Option<i32>,
    /// Windows virtual key code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub windows_virtual_key_code: Option<i32>,
    /// Whether this is an auto-repeat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_repeat: Option<bool>,
    /// Whether this is a keypad event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_keypad: Option<bool>,
    /// Whether the event is a system key (Alt/Meta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_system_key: Option<bool>,
}
impl CdpCommand for DispatchKeyEvent {
    const METHOD: &'static str = "Input.dispatchKeyEvent";
    type Returns = Empty;
}

/// `Input.dispatchMouseEvent`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchMouseEvent {
    /// Event phase.
    pub r#type: MouseEventType,
    /// X coordinate of the event relative to the main frame's viewport in CSS pixels.
    pub x: f64,
    /// Y coordinate of the event relative to the main frame's viewport in CSS pixels.
    pub y: f64,
    /// Bit mask of modifiers (see [`DispatchKeyEvent::modifiers`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<u32>,
    /// Mouse button identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<MouseButton>,
    /// Number of times the mouse button was clicked. Defaults to 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_count: Option<u32>,
    /// X-axis delta for `"mouseWheel"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_x: Option<f64>,
    /// Y-axis delta for `"mouseWheel"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_y: Option<f64>,
}
impl CdpCommand for DispatchMouseEvent {
    const METHOD: &'static str = "Input.dispatchMouseEvent";
    type Returns = Empty;
}

/// `Input.insertText` — types text into the focused element via IME.
#[derive(Debug, Clone, Serialize)]
pub struct InsertText {
    /// Text to insert.
    pub text: String,
}
impl CdpCommand for InsertText {
    const METHOD: &'static str = "Input.insertText";
    type Returns = Empty;
}

/// Domain facade returned by [`Cdp::input`].
#[derive(Debug)]
pub struct InputDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> InputDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Input.insertText` — types text into the focused element.
    pub async fn insert_text(&self, text: impl Into<String>) -> WebDriverResult<()> {
        self.cdp
            .send(InsertText {
                text: text.into(),
            })
            .await?;
        Ok(())
    }
}
