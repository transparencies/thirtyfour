//! `Emulation` domain — device, locale, timezone, and media emulation.

use serde::Serialize;

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::error::WebDriverResult;

/// `Emulation.setDeviceMetricsOverride`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDeviceMetricsOverride {
    /// Override width in CSS pixels.
    pub width: u32,
    /// Override height in CSS pixels.
    pub height: u32,
    /// Override `device-pixel-ratio`. `0` disables.
    pub device_scale_factor: f64,
    /// Whether to emulate mobile.
    pub mobile: bool,
    /// Whether to scale according to a fixed ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Screen orientation override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_orientation: Option<serde_json::Value>,
}
impl CdpCommand for SetDeviceMetricsOverride {
    const METHOD: &'static str = "Emulation.setDeviceMetricsOverride";
    type Returns = Empty;
}

/// `Emulation.clearDeviceMetricsOverride`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ClearDeviceMetricsOverride;
impl CdpCommand for ClearDeviceMetricsOverride {
    const METHOD: &'static str = "Emulation.clearDeviceMetricsOverride";
    type Returns = Empty;
}

/// `Emulation.setUserAgentOverride`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserAgentOverride {
    /// User agent string to use.
    pub user_agent: String,
    /// Browser language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_language: Option<String>,
    /// Platform string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}
impl CdpCommand for SetUserAgentOverride {
    const METHOD: &'static str = "Emulation.setUserAgentOverride";
    type Returns = Empty;
}

/// `Emulation.setGeolocationOverride`. Pass `None` for all fields to clear.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetGeolocationOverride {
    /// Mock latitude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// Mock longitude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// Mock accuracy in meters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
}
impl CdpCommand for SetGeolocationOverride {
    const METHOD: &'static str = "Emulation.setGeolocationOverride";
    type Returns = Empty;
}

/// `Emulation.setLocaleOverride`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SetLocaleOverride {
    /// ICU locale name (e.g. `"en-US"`). Empty/omitted resets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}
impl CdpCommand for SetLocaleOverride {
    const METHOD: &'static str = "Emulation.setLocaleOverride";
    type Returns = Empty;
}

/// `Emulation.setTimezoneOverride`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTimezoneOverride {
    /// IANA timezone (e.g. `"Europe/Berlin"`). Empty resets.
    pub timezone_id: String,
}
impl CdpCommand for SetTimezoneOverride {
    const METHOD: &'static str = "Emulation.setTimezoneOverride";
    type Returns = Empty;
}

/// One media feature to override (e.g. `prefers-color-scheme: dark`).
#[derive(Debug, Clone, Serialize)]
pub struct MediaFeature {
    /// Feature name.
    pub name: String,
    /// Feature value.
    pub value: String,
}

/// `Emulation.setEmulatedMedia`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEmulatedMedia {
    /// Media type to emulate (`"screen"`, `"print"`, or empty to reset).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Media features to override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<MediaFeature>>,
}
impl CdpCommand for SetEmulatedMedia {
    const METHOD: &'static str = "Emulation.setEmulatedMedia";
    type Returns = Empty;
}

/// Domain facade returned by [`Cdp::emulation`].
#[derive(Debug)]
pub struct EmulationDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> EmulationDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Emulation.setDeviceMetricsOverride`.
    pub async fn set_device_metrics_override(
        &self,
        width: u32,
        height: u32,
        device_scale_factor: f64,
        mobile: bool,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(SetDeviceMetricsOverride {
                width,
                height,
                device_scale_factor,
                mobile,
                scale: None,
                screen_orientation: None,
            })
            .await?;
        Ok(())
    }

    /// `Emulation.clearDeviceMetricsOverride`.
    pub async fn clear_device_metrics_override(&self) -> WebDriverResult<()> {
        self.cdp.send(ClearDeviceMetricsOverride).await?;
        Ok(())
    }

    /// `Emulation.setUserAgentOverride`.
    pub async fn set_user_agent_override(
        &self,
        user_agent: impl Into<String>,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(SetUserAgentOverride {
                user_agent: user_agent.into(),
                accept_language: None,
                platform: None,
            })
            .await?;
        Ok(())
    }

    /// `Emulation.setGeolocationOverride` to a specific location.
    pub async fn set_geolocation_override(
        &self,
        latitude: f64,
        longitude: f64,
        accuracy: f64,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(SetGeolocationOverride {
                latitude: Some(latitude),
                longitude: Some(longitude),
                accuracy: Some(accuracy),
            })
            .await?;
        Ok(())
    }

    /// Clear `Emulation.setGeolocationOverride`.
    pub async fn clear_geolocation_override(&self) -> WebDriverResult<()> {
        self.cdp.send(SetGeolocationOverride::default()).await?;
        Ok(())
    }

    /// `Emulation.setLocaleOverride`.
    pub async fn set_locale_override(&self, locale: impl Into<String>) -> WebDriverResult<()> {
        self.cdp
            .send(SetLocaleOverride {
                locale: Some(locale.into()),
            })
            .await?;
        Ok(())
    }

    /// `Emulation.setTimezoneOverride`.
    pub async fn set_timezone_override(
        &self,
        timezone_id: impl Into<String>,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(SetTimezoneOverride {
                timezone_id: timezone_id.into(),
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
        assert_eq!(SetDeviceMetricsOverride::METHOD, "Emulation.setDeviceMetricsOverride");
        assert_eq!(ClearDeviceMetricsOverride::METHOD, "Emulation.clearDeviceMetricsOverride");
        assert_eq!(SetUserAgentOverride::METHOD, "Emulation.setUserAgentOverride");
        assert_eq!(SetGeolocationOverride::METHOD, "Emulation.setGeolocationOverride");
        assert_eq!(SetLocaleOverride::METHOD, "Emulation.setLocaleOverride");
        assert_eq!(SetTimezoneOverride::METHOD, "Emulation.setTimezoneOverride");
        assert_eq!(SetEmulatedMedia::METHOD, "Emulation.setEmulatedMedia");
    }

    #[test]
    fn set_device_metrics_required_fields() {
        let v = serde_json::to_value(SetDeviceMetricsOverride {
            width: 360,
            height: 640,
            device_scale_factor: 2.0,
            mobile: true,
            scale: None,
            screen_orientation: None,
        })
        .unwrap();
        assert_eq!(v["width"], 360);
        assert_eq!(v["height"], 640);
        assert_eq!(v["deviceScaleFactor"], 2.0);
        assert_eq!(v["mobile"], true);
        assert!(v.get("scale").is_none());
        assert!(v.get("screenOrientation").is_none());
    }

    #[test]
    fn set_device_metrics_with_screen_orientation() {
        let v = serde_json::to_value(SetDeviceMetricsOverride {
            width: 100,
            height: 100,
            device_scale_factor: 1.0,
            mobile: false,
            scale: Some(0.5),
            screen_orientation: Some(json!({"type": "portraitPrimary", "angle": 0})),
        })
        .unwrap();
        assert_eq!(v["scale"], 0.5);
        assert_eq!(v["screenOrientation"]["type"], "portraitPrimary");
    }

    #[test]
    fn set_geolocation_default_clears() {
        // `Default::default()` is the documented "clear" form: all None,
        // serialises to an empty object.
        let v = serde_json::to_value(SetGeolocationOverride::default()).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn set_geolocation_full() {
        let v = serde_json::to_value(SetGeolocationOverride {
            latitude: Some(48.85),
            longitude: Some(2.35),
            accuracy: Some(50.0),
        })
        .unwrap();
        assert_eq!(v["latitude"], 48.85);
        assert_eq!(v["longitude"], 2.35);
        assert_eq!(v["accuracy"], 50.0);
    }

    #[test]
    fn set_locale_override_serialises_locale() {
        let v = serde_json::to_value(SetLocaleOverride {
            locale: Some("en-US".to_string()),
        })
        .unwrap();
        assert_eq!(v["locale"], "en-US");
    }

    #[test]
    fn set_timezone_override_serialises_timezone_id() {
        let v = serde_json::to_value(SetTimezoneOverride {
            timezone_id: "Europe/Berlin".to_string(),
        })
        .unwrap();
        assert_eq!(v["timezoneId"], "Europe/Berlin");
    }

    #[test]
    fn set_emulated_media_default_skips_all() {
        let v = serde_json::to_value(SetEmulatedMedia::default()).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn set_emulated_media_with_features() {
        let v = serde_json::to_value(SetEmulatedMedia {
            media: Some("screen".to_string()),
            features: Some(vec![MediaFeature {
                name: "prefers-color-scheme".to_string(),
                value: "dark".to_string(),
            }]),
        })
        .unwrap();
        assert_eq!(v["media"], "screen");
        assert_eq!(v["features"][0]["name"], "prefers-color-scheme");
        assert_eq!(v["features"][0]["value"], "dark");
    }

    #[test]
    fn set_user_agent_override_field_names() {
        let v = serde_json::to_value(SetUserAgentOverride {
            user_agent: "ua".to_string(),
            accept_language: Some("en".to_string()),
            platform: Some("Linux".to_string()),
        })
        .unwrap();
        assert_eq!(v["userAgent"], "ua");
        assert_eq!(v["acceptLanguage"], "en");
        assert_eq!(v["platform"], "Linux");
    }
}
