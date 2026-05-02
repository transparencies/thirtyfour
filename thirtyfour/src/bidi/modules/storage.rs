//! `storage.*` BiDi module — cookies and storage partitions.
//!
//! Cookies use the crate-wide [`Cookie`] and [`SameSite`] types from
//! [`crate::cookie`], so callers don't have to learn a second cookie
//! shape. The BiDi wire format (lowercase `sameSite`,
//! `network.BytesValue`-wrapped values) is handled internally via the
//! serde adapters in `crate::common::cookie::bidi`.
//!
//! [`Cookie`]: crate::Cookie
//! [`SameSite`]: crate::SameSite

use serde::{Deserialize, Serialize};

use crate::Cookie;
use crate::SameSite;
use crate::bidi::BiDi;
use crate::bidi::command::BidiCommand;
use crate::bidi::error::BidiError;
use crate::common::cookie::bidi::{bytes_string, same_site, string_from_bytes_value};

/// Cookie partition descriptor.
///
/// `storage.PartitionDescriptor` is opaque addressing for a storage
/// partition. Use `serde_json::json!` to construct, e.g.
/// `json!({"type":"context","context": id})` or
/// `json!({"type":"storageKey","userContext":"default","sourceOrigin":"https://x"})`.
pub type PartitionDescriptor = serde_json::Value;

// ---------------------------------------------------------------------------
// Wire-shape representations of cookies on the BiDi side.
//
// Public callers see `common::cookie::Cookie`; we marshal into/out of these
// internal structs so the serde adapters can do their lowercase-sameSite /
// BytesValue work without leaking onto the user-facing type.
// ---------------------------------------------------------------------------

/// Wire shape of a `storage.Cookie` returned by `storage.getCookies`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireCookie {
    name: String,
    /// `network.BytesValue` — `{"type":"string","value":"…"}` or
    /// `{"type":"base64","value":"…"}`.
    value: serde_json::Value,
    domain: String,
    path: String,
    http_only: bool,
    secure: bool,
    #[serde(with = "same_site")]
    same_site: SameSite,
    #[serde(default)]
    #[allow(dead_code)] // surfaced only on the wire; not on `common::Cookie`.
    size: Option<u32>,
    #[serde(default)]
    expiry: Option<i64>,
}

impl From<WireCookie> for Cookie {
    fn from(w: WireCookie) -> Self {
        Cookie {
            name: w.name,
            value: string_from_bytes_value(&w.value),
            path: Some(w.path),
            domain: Some(w.domain),
            secure: Some(w.secure),
            http_only: Some(w.http_only),
            expiry: w.expiry,
            same_site: Some(w.same_site),
        }
    }
}

/// Wire shape of `storage.PartialCookie` sent to `storage.setCookie`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WirePartialCookie {
    name: String,
    value: serde_json::Value,
    domain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secure: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", with = "same_site::option")]
    same_site: Option<SameSite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expiry: Option<i64>,
}

impl WirePartialCookie {
    fn from_cookie(c: Cookie) -> Result<Self, BidiError> {
        let domain = c.domain.ok_or_else(|| BidiError {
            command: "storage.setCookie".to_string(),
            error: "invalid argument".to_string(),
            message: "BiDi storage.setCookie requires a `domain` on the cookie".to_string(),
            stacktrace: None,
        })?;
        Ok(Self {
            name: c.name,
            value: bytes_string(c.value),
            domain,
            path: c.path,
            http_only: c.http_only,
            secure: c.secure,
            same_site: c.same_site,
            expiry: c.expiry,
        })
    }
}

/// Filter for [`StorageModule::get_cookies_filtered`] /
/// [`StorageModule::delete_cookies_filtered`].
///
/// Each field, if `Some`, restricts the matched cookies. Fields that share
/// names with [`Cookie`] match the same wire field on the BiDi side.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieFilter {
    /// Match by exact name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Match by domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Match by path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Match by http-only flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    /// Match by secure flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
    /// Match by same-site.
    #[serde(skip_serializing_if = "Option::is_none", with = "same_site::option")]
    pub same_site: Option<SameSite>,
    /// Match by expiry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<i64>,
}

impl CookieFilter {
    fn by_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// `storage.getCookies`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetCookies {
    /// Optional filter — only matching cookies are returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<CookieFilter>,
    /// Optional partition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<PartitionDescriptor>,
}

impl BidiCommand for GetCookies {
    const METHOD: &'static str = "storage.getCookies";
    type Returns = WireGetCookiesResult;
}

/// Wire-shape response for [`GetCookies`]. Internal — the public API
/// returns [`GetCookiesResult`] with cookies normalised to [`Cookie`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireGetCookiesResult {
    cookies: Vec<WireCookie>,
    #[serde(default)]
    partition_key: serde_json::Value,
}

/// Response for [`StorageModule::get_cookies`].
#[derive(Debug, Clone)]
pub struct GetCookiesResult {
    /// Matching cookies, normalised to [`Cookie`] (BiDi-only fields like
    /// `size` are dropped — the response struct surfaces only what fits
    /// the unified shape).
    pub cookies: Vec<Cookie>,
    /// Resolved partition key (driver-defined shape).
    pub partition_key: serde_json::Value,
}

impl From<WireGetCookiesResult> for GetCookiesResult {
    fn from(w: WireGetCookiesResult) -> Self {
        Self {
            cookies: w.cookies.into_iter().map(Cookie::from).collect(),
            partition_key: w.partition_key,
        }
    }
}

/// `storage.setCookie`.
#[derive(Debug, Clone, Serialize)]
struct SetCookie {
    cookie: WirePartialCookie,
    #[serde(skip_serializing_if = "Option::is_none")]
    partition: Option<PartitionDescriptor>,
}

impl BidiCommand for SetCookie {
    const METHOD: &'static str = "storage.setCookie";
    type Returns = SetCookieResult;
}

/// Response for [`StorageModule::set_cookie`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCookieResult {
    /// Resolved partition key.
    #[serde(default)]
    pub partition_key: serde_json::Value,
}

/// `storage.deleteCookies`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DeleteCookies {
    /// Filter — every matching cookie is removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<CookieFilter>,
    /// Optional partition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<PartitionDescriptor>,
}

impl BidiCommand for DeleteCookies {
    const METHOD: &'static str = "storage.deleteCookies";
    type Returns = DeleteCookiesResult;
}

/// Response for [`StorageModule::delete_cookies_filtered`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCookiesResult {
    /// Resolved partition key.
    #[serde(default)]
    pub partition_key: serde_json::Value,
}

/// Module facade returned by [`BiDi::storage`].
#[derive(Debug)]
pub struct StorageModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> StorageModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// `storage.getCookies` — all cookies in the default partition.
    pub async fn get_cookies(&self) -> Result<GetCookiesResult, BidiError> {
        self.get_cookies_filtered(None).await
    }

    /// `storage.getCookies` filtered by name.
    pub async fn get_cookies_by_name(
        &self,
        name: impl Into<String>,
    ) -> Result<GetCookiesResult, BidiError> {
        self.get_cookies_filtered(Some(CookieFilter::by_name(name))).await
    }

    /// `storage.getCookies` with an arbitrary [`CookieFilter`].
    pub async fn get_cookies_filtered(
        &self,
        filter: Option<CookieFilter>,
    ) -> Result<GetCookiesResult, BidiError> {
        let raw = self
            .bidi
            .send(GetCookies {
                filter,
                partition: None,
            })
            .await?;
        Ok(raw.into())
    }

    /// `storage.setCookie`.
    ///
    /// BiDi requires the cookie's `domain` to be set. If `cookie.domain` is
    /// `None`, this returns an error without making a network call.
    pub async fn set_cookie(&self, cookie: Cookie) -> Result<SetCookieResult, BidiError> {
        let cookie = WirePartialCookie::from_cookie(cookie)?;
        self.bidi
            .send(SetCookie {
                cookie,
                partition: None,
            })
            .await
    }

    /// `storage.deleteCookies` — by exact name.
    pub async fn delete_cookies_by_name(&self, name: impl Into<String>) -> Result<(), BidiError> {
        self.delete_cookies_filtered(Some(CookieFilter::by_name(name))).await
    }

    /// `storage.deleteCookies` — every cookie in the default partition.
    pub async fn delete_all_cookies(&self) -> Result<(), BidiError> {
        self.delete_cookies_filtered(None).await
    }

    /// `storage.deleteCookies` with an arbitrary [`CookieFilter`].
    pub async fn delete_cookies_filtered(
        &self,
        filter: Option<CookieFilter>,
    ) -> Result<(), BidiError> {
        self.bidi
            .send(DeleteCookies {
                filter,
                partition: None,
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
    fn wire_cookie_deserialises_from_bidi_shape() {
        let v = json!({
            "name": "k",
            "value": {"type": "string", "value": "v"},
            "domain": "example.com",
            "path": "/",
            "httpOnly": false,
            "secure": true,
            "sameSite": "lax",
            "size": 7,
            "expiry": 1_700_000_000_i64
        });
        let w: WireCookie = serde_json::from_value(v).unwrap();
        let c: Cookie = w.into();
        assert_eq!(c.name, "k");
        assert_eq!(c.value, "v");
        assert_eq!(c.same_site, Some(SameSite::Lax));
        assert_eq!(c.http_only, Some(false));
        assert_eq!(c.secure, Some(true));
        assert_eq!(c.expiry, Some(1_700_000_000));
        assert_eq!(c.domain.as_deref(), Some("example.com"));
        assert_eq!(c.path.as_deref(), Some("/"));
    }

    #[test]
    fn wire_partial_cookie_uses_lowercase_same_site_and_bytes_value() {
        let mut c = Cookie::new("k", "v");
        c.set_domain("example.com");
        c.set_path("/");
        c.set_same_site(SameSite::Lax);
        c.set_http_only(true);
        let w = WirePartialCookie::from_cookie(c).unwrap();
        let v = serde_json::to_value(&w).unwrap();
        assert_eq!(v["name"], "k");
        assert_eq!(v["value"]["type"], "string");
        assert_eq!(v["value"]["value"], "v");
        assert_eq!(v["domain"], "example.com");
        assert_eq!(v["path"], "/");
        assert_eq!(v["httpOnly"], true);
        assert_eq!(v["sameSite"], "lax");
    }

    #[test]
    fn wire_partial_cookie_skips_unset_optionals() {
        let mut c = Cookie::new("k", "v");
        c.set_domain("example.com");
        let w = WirePartialCookie::from_cookie(c).unwrap();
        let v = serde_json::to_value(&w).unwrap();
        assert!(v.get("path").is_none());
        assert!(v.get("httpOnly").is_none());
        assert!(v.get("secure").is_none());
        assert!(v.get("sameSite").is_none());
        assert!(v.get("expiry").is_none());
    }

    #[test]
    fn set_cookie_without_domain_errors_without_sending() {
        let c = Cookie::new("k", "v");
        let err = WirePartialCookie::from_cookie(c).unwrap_err();
        assert_eq!(err.command, "storage.setCookie");
        assert!(err.message.contains("domain"));
    }
}
