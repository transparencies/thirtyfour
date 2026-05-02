//! Cookie and SameSite types.

use serde::{Deserialize, Serialize};

/// The `sameSite` attribute of a cookie.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SameSite {
    /// Only same-site requests carry the cookie.
    Strict,
    /// Top-level navigations also carry the cookie.
    Lax,
    /// No SameSite restriction. Must be combined with `secure: true`.
    None,
    /// Let the driver pick the policy. BiDi only.
    Default,
}

/// A browser cookie.
///
/// Used by `WebDriver::add_cookie` / `WebDriver::get_cookies` and by the
/// BiDi `storage.*` module. Unset optional fields are skipped on
/// serialization, so a freshly constructed `Cookie::new` is just a
/// name + value on the wire.
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

// Serde adapters that translate `Cookie` / `SameSite` to and from
// BiDi's wire shape (lowercase `sameSite`, value wrapped in a
// `network.BytesValue`).
#[cfg(feature = "bidi")]
pub(crate) mod bidi {
    /// Wrap a string as a BiDi `network.BytesValue` JSON object.
    pub(crate) fn bytes_string(value: impl Into<String>) -> serde_json::Value {
        serde_json::json!({"type": "string", "value": value.into()})
    }

    /// Extract the encoded string from a BiDi `network.BytesValue`.
    /// Base64-typed values are returned as their encoded form rather
    /// than decoded.
    pub(crate) fn string_from_bytes_value(v: &serde_json::Value) -> String {
        v.get("value").and_then(|s| s.as_str()).unwrap_or_default().to_string()
    }

    /// `serde(with = "...")` adapter for BiDi's lowercase `sameSite`.
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

        /// `Option<SameSite>` variant for fields that skip when unset.
        #[allow(dead_code)] // `deserialize` may be unused depending on call sites.
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
