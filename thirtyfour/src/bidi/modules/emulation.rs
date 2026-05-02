//! `emulation.*` — override browser-exposed environmental APIs
//! (geolocation, locale, timezone, screen, user agent, scripting,
//! touch, scrollbars, forced-colors theme, network conditions).
//!
//! Each command follows the same scoping pattern:
//!
//! - **No `contexts` and no `user_contexts`** — the override applies as
//!   the new default (every existing and future top-level traversable).
//! - **`contexts` only** — restrict to the given top-level traversables.
//! - **`user_contexts` only** — restrict to the given user contexts
//!   (and every traversable inside them).
//! - **Both** — invalid; the spec returns `invalid argument`.
//!
//! Pass `None` (or the variant that the spec defines as "clear", e.g.
//! `coordinates: None` for geolocation) to remove a previously-set
//! override.
//!
//! Driver support varies. Chromium implements most commands; Firefox
//! lags. Calls return `unsupported operation` when the driver hasn't
//! implemented a particular override yet.
//!
//! See the [W3C `emulation` module specification][spec] for the
//! canonical definitions.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-emulation

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::{BrowsingContextId, UserContextId};
use crate::common::protocol::string_enum;

string_enum! {
    /// Theme value for [`SetForcedColorsModeThemeOverride::theme`].
    pub enum ForcedColorsModeTheme {
        /// Light theme.
        Light = "light",
        /// Dark theme.
        Dark = "dark",
    }
}

/// [`emulation.setForcedColorsModeThemeOverride`][spec] — override the
/// CSS forced-colors theme.
///
/// Affects the value returned by `matchMedia('(prefers-color-scheme:
/// dark)')` and the in-page rendering. Pass `theme: None` to clear.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setForcedColorsModeThemeOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetForcedColorsModeThemeOverride {
    /// New theme, or `None` to clear.
    pub theme: Option<ForcedColorsModeTheme>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetForcedColorsModeThemeOverride {
    const METHOD: &'static str = "emulation.setForcedColorsModeThemeOverride";
    type Returns = Empty;
}

/// Geolocation coordinates for [`SetGeolocationOverride`]. Mirrors the
/// spec's [`emulation.GeolocationCoordinates`][spec] type.
///
/// Lat/long are required; the rest mirror the
/// [Geolocation API's `Coordinates`][coords] interface and are
/// optional. Note `altitude_accuracy` requires `altitude`.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-emulation-GeolocationCoordinates
/// [coords]: https://www.w3.org/TR/geolocation/#coordinates_interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeolocationCoordinates {
    /// Latitude in degrees, -90.0..=90.0.
    pub latitude: f64,
    /// Longitude in degrees, -180.0..=180.0.
    pub longitude: f64,
    /// Accuracy in meters (default 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
    /// Altitude in meters above the WGS-84 ellipsoid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>,
    /// Altitude accuracy in meters. Requires `altitude`.
    #[serde(rename = "altitudeAccuracy", skip_serializing_if = "Option::is_none")]
    pub altitude_accuracy: Option<f64>,
    /// Heading (bearing) in degrees, 0.0..=360.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<f64>,
    /// Speed in m/s.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
}

/// Geolocation override variant. Mirrors the spec's
/// `emulation.SetGeolocationOverrideParameters` discriminated body.
///
/// Use [`Coordinates`][Self::Coordinates] (with `Some(coords)` to set or
/// `None` to clear) to override with a position, or [`Error`][Self::Error]
/// to make the Geolocation API report a `GeolocationPositionError`.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum GeolocationOverride {
    /// Override with explicit coordinates, or clear by passing `None`.
    Coordinates {
        /// Coordinates payload. `None` clears the override.
        #[serde(skip_serializing_if = "Option::is_none")]
        coordinates: Option<GeolocationCoordinates>,
    },
    /// Override with a position-error. Drives the
    /// [`GeolocationPositionError`][api] callback in JS.
    ///
    /// [api]: https://www.w3.org/TR/geolocation/#position_error_interface
    Error {
        /// Error payload — currently the spec only defines
        /// [`PositionUnavailable`][GeolocationPositionError::PositionUnavailable].
        error: GeolocationPositionError,
    },
}

/// Position error variant for [`GeolocationOverride::Error`]. Mirrors
/// the spec's `emulation.GeolocationPositionError` type.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GeolocationPositionError {
    /// Browser reports `POSITION_UNAVAILABLE` to JS.
    PositionUnavailable,
}

/// [`emulation.setGeolocationOverride`][spec] — override the page's
/// geolocation reading.
///
/// Use [`GeolocationOverride::Coordinates`] with `Some(coords)` to fake
/// a location, `None` to clear, or [`GeolocationOverride::Error`] to
/// make the API report failure.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setGeolocationOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGeolocationOverride {
    /// Override (coordinates or error).
    #[serde(flatten)]
    pub geolocation: GeolocationOverride,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetGeolocationOverride {
    const METHOD: &'static str = "emulation.setGeolocationOverride";
    type Returns = Empty;
}

/// [`emulation.setLocaleOverride`][spec] — override the navigator
/// locale.
///
/// `locale` must be a structurally-valid BCP-47 language tag
/// (`"en-GB"`, `"ja"`, `"de-AT"`, …); `None` clears the override.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setLocaleOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLocaleOverride {
    /// BCP-47 locale tag, or `None` to clear.
    pub locale: Option<String>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetLocaleOverride {
    const METHOD: &'static str = "emulation.setLocaleOverride";
    type Returns = Empty;
}

/// Network conditions for [`SetNetworkConditions`]. Mirrors the spec's
/// `emulation.NetworkConditions` union — currently only the `offline`
/// variant is defined.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NetworkConditions {
    /// Force the network stack offline. The browser fails every fetch
    /// and refuses new WebSocket / WebTransport connections.
    Offline,
}

/// [`emulation.setNetworkConditions`][spec] — override per-context
/// network conditions.
///
/// Pass `network_conditions: None` to clear. The spec currently only
/// defines an "offline" mode; bandwidth/latency throttling is
/// implementation-defined for now.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setNetworkConditions
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNetworkConditions {
    /// New conditions, or `None` to clear the override.
    pub network_conditions: Option<NetworkConditions>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetNetworkConditions {
    const METHOD: &'static str = "emulation.setNetworkConditions";
    type Returns = Empty;
}

/// Screen area in CSS pixels for [`SetScreenSettingsOverride`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenArea {
    /// Width in CSS pixels.
    pub width: u32,
    /// Height in CSS pixels.
    pub height: u32,
}

/// [`emulation.setScreenSettingsOverride`][spec] — emulate a different
/// screen size for the page (`screen.width`, `screen.height`,
/// `screen.availWidth`, `screen.availHeight`).
///
/// Pass `screen_area: None` to clear.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScreenSettingsOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetScreenSettingsOverride {
    /// Override, or `None` to clear.
    pub screen_area: Option<ScreenArea>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetScreenSettingsOverride {
    const METHOD: &'static str = "emulation.setScreenSettingsOverride";
    type Returns = Empty;
}

string_enum! {
    /// Natural orientation of the emulated device for
    /// [`ScreenOrientation::natural`].
    pub enum ScreenOrientationNatural {
        /// Natural portrait (taller than wide). Default for phones.
        Portrait = "portrait",
        /// Natural landscape (wider than tall). Default for tablets in
        /// landscape mode.
        Landscape = "landscape",
    }
}

string_enum! {
    /// Current orientation for [`ScreenOrientation::orientation_type`].
    /// Maps to the values exposed by the [Screen Orientation API][api].
    ///
    /// [api]: https://www.w3.org/TR/screen-orientation/
    pub enum ScreenOrientationType {
        /// Portrait, primary direction.
        PortraitPrimary = "portrait-primary",
        /// Portrait, secondary (180° from primary).
        PortraitSecondary = "portrait-secondary",
        /// Landscape, primary direction.
        LandscapePrimary = "landscape-primary",
        /// Landscape, secondary (180° from primary).
        LandscapeSecondary = "landscape-secondary",
    }
}

/// Screen orientation for [`SetScreenOrientationOverride`]. Pairs the
/// device's natural orientation with the current orientation type.
#[derive(Debug, Clone, Serialize)]
pub struct ScreenOrientation {
    /// Natural orientation of the (emulated) device.
    pub natural: ScreenOrientationNatural,
    /// Current orientation type.
    #[serde(rename = "type")]
    pub orientation_type: ScreenOrientationType,
}

/// [`emulation.setScreenOrientationOverride`][spec] — override the
/// reported screen orientation.
///
/// Pass `screen_orientation: None` to clear.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScreenOrientationOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetScreenOrientationOverride {
    /// Override, or `None` to clear.
    pub screen_orientation: Option<ScreenOrientation>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetScreenOrientationOverride {
    const METHOD: &'static str = "emulation.setScreenOrientationOverride";
    type Returns = Empty;
}

/// [`emulation.setUserAgentOverride`][spec] — override `navigator.userAgent`.
///
/// `user_agent: None` clears the override. Note this affects the
/// outgoing `User-Agent` header as well as `navigator.userAgent`, but
/// does **not** override Client Hint headers — for those you'll need
/// driver-specific extensions.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setUserAgentOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserAgentOverride {
    /// New user-agent string, or `None` to clear.
    pub user_agent: Option<String>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetUserAgentOverride {
    const METHOD: &'static str = "emulation.setUserAgentOverride";
    type Returns = Empty;
}

/// [`emulation.setScriptingEnabled`][spec] — disable JavaScript
/// execution.
///
/// The spec only defines two wire values: `false` (disable) and `null`
/// (clear the override). There is intentionally no way to selectively
/// _re-enable_ scripting — the spec calls this out, since untrusted
/// re-enable would defeat the security purpose.
///
/// To preserve that constraint while keeping a familiar `Option<bool>`
/// API, this struct silently omits the field if `enabled == Some(true)`
/// (which is equivalent to "no override"). Use `Some(false)` to
/// disable, `None` to clear.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScriptingEnabled
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetScriptingEnabled {
    /// `Some(false)` to disable scripting, `None` to clear.
    /// `Some(true)` is silently dropped (the wire shape forbids it).
    #[serde(serialize_with = "serialize_disable_only")]
    pub enabled: Option<bool>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

fn serialize_disable_only<S: serde::Serializer>(
    value: &Option<bool>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(false) => serializer.serialize_bool(false),
        _ => serializer.serialize_none(),
    }
}

impl BidiCommand for SetScriptingEnabled {
    const METHOD: &'static str = "emulation.setScriptingEnabled";
    type Returns = Empty;
}

string_enum! {
    /// Scrollbar style for [`SetScrollbarTypeOverride::scrollbar_type`].
    pub enum ScrollbarType {
        /// Classic scrollbars — reserve space alongside content.
        Classic = "classic",
        /// Overlay scrollbars — drawn on top of content.
        Overlay = "overlay",
    }
}

/// [`emulation.setScrollbarTypeOverride`][spec] — override the page's
/// scrollbar style.
///
/// `scrollbar_type: None` clears the override.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScrollbarTypeOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetScrollbarTypeOverride {
    /// New style, or `None` to clear.
    pub scrollbar_type: Option<ScrollbarType>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetScrollbarTypeOverride {
    const METHOD: &'static str = "emulation.setScrollbarTypeOverride";
    type Returns = Empty;
}

/// [`emulation.setTimezoneOverride`][spec] — override the system
/// timezone for the page.
///
/// `timezone` accepts either an IANA timezone name (`"Australia/Sydney"`,
/// `"Europe/Berlin"`) or a UTC offset string (`"+10:00"`, `"-05:00"`).
/// `None` clears the override. Invalid values are rejected with
/// `invalid argument`.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setTimezoneOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTimezoneOverride {
    /// IANA timezone name or UTC-offset string. `None` clears.
    pub timezone: Option<String>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetTimezoneOverride {
    const METHOD: &'static str = "emulation.setTimezoneOverride";
    type Returns = Empty;
}

/// [`emulation.setTouchOverride`][spec] — emulate
/// `navigator.maxTouchPoints` (drives feature-detection of touch
/// support, e.g. `'ontouchstart' in window`).
///
/// `max_touch_points: None` clears the override; values must be `>= 1`.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setTouchOverride
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTouchOverride {
    /// New `maxTouchPoints` value (`>= 1`), or `None` to clear.
    pub max_touch_points: Option<u32>,
    /// Browsing-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<BrowsingContextId>>,
    /// User-context scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_contexts: Option<Vec<UserContextId>>,
}

impl BidiCommand for SetTouchOverride {
    const METHOD: &'static str = "emulation.setTouchOverride";
    type Returns = Empty;
}

/// Convenience facade for the `emulation.*` module.
///
/// Returned by [`BiDi::emulation`](crate::bidi::BiDi::emulation). Every
/// method here applies the override globally. To scope an override to
/// specific browsing contexts or user contexts, build the command
/// struct directly and supply [`contexts`][SetUserAgentOverride::contexts]
/// or [`user_contexts`][SetUserAgentOverride::user_contexts].
#[derive(Debug)]
pub struct EmulationModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> EmulationModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// Override the forced-colors theme globally via
    /// [`emulation.setForcedColorsModeThemeOverride`][spec].
    ///
    /// Pass `None` to clear.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setForcedColorsModeThemeOverride
    pub async fn set_forced_colors_mode_theme_override(
        &self,
        theme: Option<ForcedColorsModeTheme>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetForcedColorsModeThemeOverride {
                theme,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override geolocation globally via
    /// [`emulation.setGeolocationOverride`][spec], using explicit
    /// coordinates (or `None` to clear).
    ///
    /// To make the API report a position-error instead, build a
    /// [`SetGeolocationOverride`] with [`GeolocationOverride::Error`].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setGeolocationOverride
    pub async fn set_geolocation_override(
        &self,
        coordinates: Option<GeolocationCoordinates>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetGeolocationOverride {
                geolocation: GeolocationOverride::Coordinates {
                    coordinates,
                },
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override the navigator locale globally via
    /// [`emulation.setLocaleOverride`][spec].
    ///
    /// `locale` is a BCP-47 tag (`"en-GB"`, etc.); `None` clears.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setLocaleOverride
    pub async fn set_locale_override(&self, locale: Option<String>) -> Result<(), BidiError> {
        self.bidi
            .send(SetLocaleOverride {
                locale,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override network conditions globally via
    /// [`emulation.setNetworkConditions`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setNetworkConditions
    pub async fn set_network_conditions(
        &self,
        conditions: Option<NetworkConditions>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetNetworkConditions {
                network_conditions: conditions,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override the reported screen size globally via
    /// [`emulation.setScreenSettingsOverride`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScreenSettingsOverride
    pub async fn set_screen_settings_override(
        &self,
        screen_area: Option<ScreenArea>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetScreenSettingsOverride {
                screen_area,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override screen orientation globally via
    /// [`emulation.setScreenOrientationOverride`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScreenOrientationOverride
    pub async fn set_screen_orientation_override(
        &self,
        orientation: Option<ScreenOrientation>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetScreenOrientationOverride {
                screen_orientation: orientation,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override the user-agent string globally via
    /// [`emulation.setUserAgentOverride`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setUserAgentOverride
    pub async fn set_user_agent_override(
        &self,
        user_agent: Option<String>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetUserAgentOverride {
                user_agent,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Disable JavaScript globally via
    /// [`emulation.setScriptingEnabled`][spec].
    ///
    /// Pass `Some(false)` to disable, `None` to clear.
    /// `Some(true)` is silently dropped — see [`SetScriptingEnabled`].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScriptingEnabled
    pub async fn set_scripting_enabled(&self, enabled: Option<bool>) -> Result<(), BidiError> {
        self.bidi
            .send(SetScriptingEnabled {
                enabled,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override scrollbar style globally via
    /// [`emulation.setScrollbarTypeOverride`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setScrollbarTypeOverride
    pub async fn set_scrollbar_type_override(
        &self,
        scrollbar_type: Option<ScrollbarType>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(SetScrollbarTypeOverride {
                scrollbar_type,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override timezone globally via
    /// [`emulation.setTimezoneOverride`][spec].
    ///
    /// `timezone` is an IANA name (`"Australia/Sydney"`) or a UTC
    /// offset (`"+10:00"`); `None` clears.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setTimezoneOverride
    pub async fn set_timezone_override(&self, timezone: Option<String>) -> Result<(), BidiError> {
        self.bidi
            .send(SetTimezoneOverride {
                timezone,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }

    /// Override `navigator.maxTouchPoints` globally via
    /// [`emulation.setTouchOverride`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-emulation-setTouchOverride
    pub async fn set_touch_override(&self, max_touch_points: Option<u32>) -> Result<(), BidiError> {
        self.bidi
            .send(SetTouchOverride {
                max_touch_points,
                contexts: None,
                user_contexts: None,
            })
            .await?;
        Ok(())
    }
}
