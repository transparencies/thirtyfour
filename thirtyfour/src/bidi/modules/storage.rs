//! `storage.*` BiDi module — cookies and storage partitions.
//!
//! Cookies use the crate-wide [`Cookie`](crate::Cookie) and
//! [`SameSite`](crate::SameSite) types.

use serde::{Deserialize, Serialize};

use crate::Cookie;
use crate::SameSite;
use crate::bidi::BiDi;
use crate::bidi::command::BidiCommand;
use crate::bidi::error::BidiError;
use crate::common::cookie::bidi::{bytes_string, same_site, string_from_bytes_value};

/// `storage.PartitionDescriptor` — opaque addressing for a storage
/// partition. Construct with `serde_json::json!`, e.g.
/// `json!({"type":"context","context": id})` or
/// `json!({"type":"storageKey","userContext":"default","sourceOrigin":"https://x"})`.
pub type PartitionDescriptor = serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireCookie {
    name: String,
    value: serde_json::Value,
    domain: String,
    path: String,
    http_only: bool,
    secure: bool,
    #[serde(with = "same_site")]
    same_site: SameSite,
    #[serde(default)]
    #[allow(dead_code)] // BiDi-only field; not surfaced on `Cookie`.
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc(hidden)]
pub struct WireGetCookiesResult {
    cookies: Vec<WireCookie>,
    #[serde(default)]
    partition_key: serde_json::Value,
}

/// Response for [`StorageModule::get_cookies`].
#[derive(Debug, Clone)]
pub struct GetCookiesResult {
    /// Matching cookies.
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
    /// `cookie.domain` is required by BiDi; an empty `domain` returns
    /// `invalid argument` without making a network call.
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

    #[test]
    fn set_cookie_without_domain_errors_locally() {
        let err = WirePartialCookie::from_cookie(Cookie::new("k", "v")).unwrap_err();
        assert_eq!(err.command, "storage.setCookie");
        assert!(err.message.contains("domain"));
    }
}
