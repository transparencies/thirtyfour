//! Unified `Cookie` and `SameSite` types used by both the W3C WebDriver
//! classic API and the BiDi `storage.*` module.
//!
//! ## Wire-shape differences
//!
//! The two protocols spell cookies slightly differently:
//!
//! - W3C classic emits `sameSite` as PascalCase (`Strict`, `Lax`, `None`)
//!   and the cookie value as a plain string.
//! - BiDi emits `sameSite` as lowercase (`strict`, `lax`, `none`,
//!   `default`) and the cookie value as a `network.BytesValue` wrapper
//!   (`{"type":"string","value":"…"}`).
//!
//! `Cookie` is the user-facing type and uses the W3C classic wire shape
//! by default. The BiDi storage module re-uses the same struct in its
//! public API and converts to/from its wire shape internally via the
//! crate-private serde adapters at the bottom of this file.

use serde::{Deserialize, Serialize};

/// The `sameSite` attribute of a cookie.
///
/// W3C WebDriver classic only defines [`SameSite::Strict`], [`SameSite::Lax`],
/// and [`SameSite::None`]. BiDi adds [`SameSite::Default`] for "let the
/// browser pick" semantics. The default `Serialize`/`Deserialize` impl uses
/// the W3C classic spelling (PascalCase); the BiDi module uses a
/// crate-private adapter for the lowercase spelling.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SameSite {
    /// Strict — only same-site requests carry the cookie.
    Strict,
    /// Lax — top-level navigations also carry the cookie.
    Lax,
    /// None — must be combined with `secure: true`.
    None,
    /// BiDi-only — let the driver pick the policy.
    Default,
}

/// A browser cookie.
///
/// Used both by `WebDriver::add_cookie` / `get_cookies` (W3C classic) and
/// by the BiDi `storage.*` module. Optional fields default to
/// driver-defined behavior; absent fields are skipped on serialization so
/// an empty `Cookie` reads as a name+value pair on either protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    /// The name of the cookie.
    pub name: String,
    /// The value of the cookie.
    pub value: String,
    /// The path of the cookie.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    /// The domain of the cookie.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub domain: Option<String>,
    /// Whether the cookie is secure.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub secure: Option<bool>,
    /// Whether the cookie is HTTP-only (inaccessible to JavaScript).
    #[serde(rename = "httpOnly", skip_serializing_if = "Option::is_none", default)]
    pub http_only: Option<bool>,
    /// The expiry date of the cookie, as seconds since epoch.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expiry: Option<i64>,
    /// The sameSite attribute of the cookie.
    #[serde(skip_serializing_if = "Option::is_none", rename = "sameSite", default)]
    pub same_site: Option<SameSite>,
}

impl Cookie {
    /// Create a new Cookie struct, specifying the name and value.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Cookie {
            name: name.into(),
            value: value.into(),
            path: None,
            domain: None,
            secure: None,
            http_only: None,
            expiry: None,
            same_site: None,
        }
    }

    /// Set the path of the cookie.
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.path = Some(path.into());
    }

    /// Set the domain of the cookie.
    pub fn set_domain(&mut self, domain: impl Into<String>) {
        self.domain = Some(domain.into());
    }

    /// Set whether the cookie is secure.
    pub fn set_secure(&mut self, secure: bool) {
        self.secure = Some(secure);
    }

    /// Set whether the cookie is HTTP-only.
    pub fn set_http_only(&mut self, http_only: bool) {
        self.http_only = Some(http_only);
    }

    /// Set the expiry date of the cookie.
    pub fn set_expiry(&mut self, expiry: i64) {
        self.expiry = Some(expiry);
    }

    /// Set the sameSite attribute of the cookie.
    pub fn set_same_site(&mut self, same_site: SameSite) {
        self.same_site = Some(same_site);
    }
}

/// Serde adapters used by the BiDi `storage.*` module to massage the
/// shared [`Cookie`] / [`SameSite`] types into BiDi's wire shape.
#[cfg(feature = "bidi")]
pub(crate) mod bidi {
    /// Wrap a plain string as a BiDi `network.BytesValue` JSON object
    /// (`{"type":"string","value":"…"}`).
    pub(crate) fn bytes_string(value: impl Into<String>) -> serde_json::Value {
        serde_json::json!({"type": "string", "value": value.into()})
    }

    /// Extract a plain string from a BiDi `network.BytesValue`. For binary
    /// (base64) or unknown types the encoded form is returned so callers
    /// don't silently get an empty string; the original JSON is still on
    /// the response struct for clients that need to handle it specially.
    pub(crate) fn string_from_bytes_value(v: &serde_json::Value) -> String {
        v.get("value").and_then(|s| s.as_str()).unwrap_or_default().to_string()
    }

    /// `serde(with = "...")` adapter that emits and accepts the BiDi
    /// lowercase spelling for [`super::SameSite`]. Drop-in replacement for
    /// the default PascalCase serde impl on the BiDi-side wire structs.
    pub(crate) mod same_site {
        use super::super::SameSite;
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        pub(crate) fn serialize<S>(value: &SameSite, ser: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let s = match value {
                SameSite::Strict => "strict",
                SameSite::Lax => "lax",
                SameSite::None => "none",
                SameSite::Default => "default",
            };
            s.serialize(ser)
        }

        pub(crate) fn deserialize<'de, D>(de: D) -> Result<SameSite, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(de)?;
            match s.as_str() {
                "strict" => Ok(SameSite::Strict),
                "lax" => Ok(SameSite::Lax),
                "none" => Ok(SameSite::None),
                "default" => Ok(SameSite::Default),
                other => Err(serde::de::Error::unknown_variant(
                    other,
                    &["strict", "lax", "none", "default"],
                )),
            }
        }

        /// Same as the module-level (de)serializers but lifted to
        /// `Option<SameSite>` so the BiDi structs that have an optional
        /// `sameSite` field can use `serde(with = "...::option")`.
        ///
        /// `deserialize` is currently only used by the BiDi `CookieFilter`
        /// path; if no caller ever round-trips an optional sameSite filter
        /// through deserialise, dead-code analysis can flag it. Allow it.
        #[allow(dead_code)]
        pub(crate) mod option {
            use super::SameSite;
            use serde::{Deserialize, Deserializer, Serializer};

            pub(crate) fn serialize<S>(value: &Option<SameSite>, ser: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                match value {
                    Some(v) => super::serialize(v, ser),
                    None => ser.serialize_none(),
                }
            }

            pub(crate) fn deserialize<'de, D>(de: D) -> Result<Option<SameSite>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let opt = Option::<String>::deserialize(de)?;
                opt.map(|s| match s.as_str() {
                    "strict" => Ok(SameSite::Strict),
                    "lax" => Ok(SameSite::Lax),
                    "none" => Ok(SameSite::None),
                    "default" => Ok(SameSite::Default),
                    other => Err(serde::de::Error::unknown_variant(
                        other,
                        &["strict", "lax", "none", "default"],
                    )),
                })
                .transpose()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classic_cookie_round_trips_pascal_case_same_site() {
        let mut c = Cookie::new("k", "v");
        c.set_same_site(SameSite::Lax);
        c.set_http_only(true);
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["name"], "k");
        assert_eq!(v["value"], "v");
        assert_eq!(v["sameSite"], "Lax");
        assert_eq!(v["httpOnly"], true);
        let back: Cookie = serde_json::from_value(v).unwrap();
        assert_eq!(back.same_site, Some(SameSite::Lax));
        assert_eq!(back.http_only, Some(true));
    }

    #[test]
    fn classic_cookie_skips_unset_fields() {
        let c = Cookie::new("k", "v");
        let v = serde_json::to_value(&c).unwrap();
        assert!(v.get("path").is_none());
        assert!(v.get("domain").is_none());
        assert!(v.get("sameSite").is_none());
        assert!(v.get("httpOnly").is_none());
    }

    #[test]
    fn classic_cookie_deserialises_w3c_shape() {
        let v = json!({
            "name": "k",
            "value": "v",
            "sameSite": "Strict",
            "httpOnly": false,
            "expiry": 1_700_000_000_i64,
        });
        let c: Cookie = serde_json::from_value(v).unwrap();
        assert_eq!(c.name, "k");
        assert_eq!(c.same_site, Some(SameSite::Strict));
        assert_eq!(c.http_only, Some(false));
        assert_eq!(c.expiry, Some(1_700_000_000));
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn bidi_same_site_adapter_uses_lowercase() {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        struct Wire {
            #[serde(with = "super::bidi::same_site")]
            v: SameSite,
        }

        for (variant, wire) in [
            (SameSite::Strict, "strict"),
            (SameSite::Lax, "lax"),
            (SameSite::None, "none"),
            (SameSite::Default, "default"),
        ] {
            let w = Wire {
                v: variant,
            };
            let json = serde_json::to_value(&w).unwrap();
            assert_eq!(json["v"], wire);
            let back: Wire = serde_json::from_value(json).unwrap();
            assert_eq!(back.v, variant);
        }
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn bidi_bytes_string_wraps_into_object() {
        let v = bidi::bytes_string("hello");
        assert_eq!(v["type"], "string");
        assert_eq!(v["value"], "hello");
        assert_eq!(bidi::string_from_bytes_value(&v), "hello");
    }
}
