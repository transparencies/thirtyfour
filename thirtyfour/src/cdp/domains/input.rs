//! `Input` domain — synthetic keyboard, mouse, and text input events.

use serde::Serialize;

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::cdp::macros::string_enum;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods() {
        assert_eq!(DispatchKeyEvent::METHOD, "Input.dispatchKeyEvent");
        assert_eq!(DispatchMouseEvent::METHOD, "Input.dispatchMouseEvent");
        assert_eq!(InsertText::METHOD, "Input.insertText");
    }

    #[test]
    fn insert_text_serialises() {
        let v = serde_json::to_value(InsertText {
            text: "hi".to_string(),
        })
        .unwrap();
        assert_eq!(v["text"], "hi");
    }

    #[test]
    fn dispatch_key_event_skips_optional_fields() {
        let v = serde_json::to_value(DispatchKeyEvent {
            r#type: KeyEventType::KeyDown,
            modifiers: None,
            text: None,
            unmodified_text: None,
            key: None,
            code: None,
            native_virtual_key_code: None,
            windows_virtual_key_code: None,
            auto_repeat: None,
            is_keypad: None,
            is_system_key: None,
        })
        .unwrap();
        assert_eq!(v["type"], "keyDown");
        // All optional fields skipped
        for field in [
            "modifiers",
            "text",
            "unmodifiedText",
            "key",
            "code",
            "nativeVirtualKeyCode",
            "windowsVirtualKeyCode",
            "autoRepeat",
            "isKeypad",
            "isSystemKey",
        ] {
            assert!(v.get(field).is_none(), "{field} should be omitted");
        }
    }

    #[test]
    fn dispatch_key_event_full() {
        let v = serde_json::to_value(DispatchKeyEvent {
            r#type: KeyEventType::KeyDown,
            modifiers: Some(8),
            text: Some("A".to_string()),
            unmodified_text: Some("a".to_string()),
            key: Some("a".to_string()),
            code: Some("KeyA".to_string()),
            native_virtual_key_code: Some(65),
            windows_virtual_key_code: Some(65),
            auto_repeat: Some(false),
            is_keypad: Some(false),
            is_system_key: Some(true),
        })
        .unwrap();
        assert_eq!(v["modifiers"], 8);
        assert_eq!(v["unmodifiedText"], "a");
        assert_eq!(v["nativeVirtualKeyCode"], 65);
        assert_eq!(v["windowsVirtualKeyCode"], 65);
        assert_eq!(v["isSystemKey"], true);
    }

    #[test]
    fn dispatch_mouse_event_required_fields() {
        let v = serde_json::to_value(DispatchMouseEvent {
            r#type: MouseEventType::MouseMoved,
            x: 10.5,
            y: 20.5,
            modifiers: None,
            button: None,
            click_count: None,
            delta_x: None,
            delta_y: None,
        })
        .unwrap();
        assert_eq!(v["type"], "mouseMoved");
        assert_eq!(v["x"], 10.5);
        assert_eq!(v["y"], 20.5);
        assert!(v.get("clickCount").is_none());
    }

    #[test]
    fn dispatch_mouse_event_wheel() {
        let v = serde_json::to_value(DispatchMouseEvent {
            r#type: MouseEventType::MouseWheel,
            x: 0.0,
            y: 0.0,
            modifiers: Some(0),
            button: Some(MouseButton::None),
            click_count: Some(0),
            delta_x: Some(0.0),
            delta_y: Some(120.0),
        })
        .unwrap();
        assert_eq!(v["button"], "none");
        assert_eq!(v["clickCount"], 0);
        assert_eq!(v["deltaY"], 120.0);
    }

    #[test]
    fn key_event_type_round_trip() {
        for (variant, wire) in [
            (KeyEventType::KeyDown, "keyDown"),
            (KeyEventType::KeyUp, "keyUp"),
            (KeyEventType::RawKeyDown, "rawKeyDown"),
            (KeyEventType::Char, "char"),
        ] {
            assert_eq!(serde_json::to_value(&variant).unwrap(), serde_json::json!(wire));
        }
    }

    #[test]
    fn mouse_event_type_round_trip() {
        for (variant, wire) in [
            (MouseEventType::MousePressed, "mousePressed"),
            (MouseEventType::MouseReleased, "mouseReleased"),
            (MouseEventType::MouseMoved, "mouseMoved"),
            (MouseEventType::MouseWheel, "mouseWheel"),
        ] {
            assert_eq!(serde_json::to_value(&variant).unwrap(), serde_json::json!(wire));
        }
    }

    #[test]
    fn mouse_button_round_trip_all_variants() {
        for (variant, wire) in [
            (MouseButton::None, "none"),
            (MouseButton::Left, "left"),
            (MouseButton::Middle, "middle"),
            (MouseButton::Right, "right"),
            (MouseButton::Back, "back"),
            (MouseButton::Forward, "forward"),
        ] {
            assert_eq!(serde_json::to_value(&variant).unwrap(), serde_json::json!(wire));
        }
    }

    #[test]
    fn unknown_variants_round_trip() {
        let k: KeyEventType = serde_json::from_value(serde_json::json!("future")).unwrap();
        assert_eq!(k, KeyEventType::Unknown("future".to_string()));
    }
}
