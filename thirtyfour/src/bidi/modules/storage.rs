//! `storage.*` BiDi module — cookies and storage partitions.

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::BidiCommand;
use crate::bidi::error::BidiError;
use crate::bidi::macros::string_enum;

string_enum! {
    /// Same-site policy for [`Cookie::same_site`] / [`PartialCookie::same_site`].
    pub enum SameSite {
        /// Strict — only same-site requests carry the cookie.
        Strict = "strict",
        /// Lax — top-level navigations also carry the cookie.
        Lax = "lax",
        /// None — must be `secure: true`.
        None = "none",
        /// Default — driver-defined.
        Default = "default",
    }
}

/// `storage.Cookie` returned by [`GetCookies`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value (BiDi `network.BytesValue`).
    pub value: serde_json::Value,
    /// Cookie domain.
    pub domain: String,
    /// Cookie path.
    pub path: String,
    /// HTTP-only flag.
    pub http_only: bool,
    /// Secure flag.
    pub secure: bool,
    /// Same-site policy.
    pub same_site: SameSite,
    /// Cookie size on the wire.
    #[serde(default)]
    pub size: Option<u32>,
    /// Expiry as epoch milliseconds; absent for session cookies.
    #[serde(default)]
    pub expiry: Option<i64>,
}

/// `storage.PartialCookie` used by [`SetCookie`] / [`DeleteCookies`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialCookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value as a `network.BytesValue`. Use [`bytes_string`] to wrap a
    /// plain string.
    pub value: serde_json::Value,
    /// Cookie domain.
    pub domain: String,
    /// Cookie path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// HTTP-only flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    /// Secure flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
    /// Same-site policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<SameSite>,
    /// Expiry as epoch milliseconds; omit for session cookies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<i64>,
}

/// Wrap a plain string into a BiDi `network.BytesValue` JSON object
/// (`{"type":"string","value":"…"}`).
pub fn bytes_string(value: impl Into<String>) -> serde_json::Value {
    serde_json::json!({"type": "string", "value": value.into()})
}

/// `storage.CookieFilter` used by [`GetCookies`] / [`DeleteCookies`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieFilter {
    /// Match by exact name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Match by value (`network.BytesValue`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<SameSite>,
    /// Match by size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    /// Match by expiry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<i64>,
}

/// `storage.PartitionDescriptor` — opaque addressing for a storage partition.
/// Use `serde_json::json!` to construct, e.g.
/// `json!({"type":"context","context": id})` or
/// `json!({"type":"storageKey","userContext":"default","sourceOrigin":"https://x"})`.
pub type PartitionDescriptor = serde_json::Value;

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
    type Returns = GetCookiesResult;
}

/// Response for [`GetCookies`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCookiesResult {
    /// Matching cookies.
    pub cookies: Vec<Cookie>,
    /// Resolved partition key (driver-defined shape).
    #[serde(default)]
    pub partition_key: serde_json::Value,
}

/// `storage.setCookie`.
#[derive(Debug, Clone, Serialize)]
pub struct SetCookie {
    /// Cookie to set.
    pub cookie: PartialCookie,
    /// Optional partition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<PartitionDescriptor>,
}

impl BidiCommand for SetCookie {
    const METHOD: &'static str = "storage.setCookie";
    type Returns = SetCookieResult;
}

/// Response for [`SetCookie`].
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

/// Response for [`DeleteCookies`].
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
        self.bidi
            .send(GetCookies {
                filter: None,
                partition: None,
            })
            .await
    }

    /// `storage.getCookies` filtered by name.
    pub async fn get_cookies_by_name(
        &self,
        name: impl Into<String>,
    ) -> Result<GetCookiesResult, BidiError> {
        self.bidi
            .send(GetCookies {
                filter: Some(CookieFilter {
                    name: Some(name.into()),
                    ..CookieFilter::default()
                }),
                partition: None,
            })
            .await
    }

    /// `storage.setCookie`.
    pub async fn set_cookie(&self, cookie: PartialCookie) -> Result<SetCookieResult, BidiError> {
        self.bidi
            .send(SetCookie {
                cookie,
                partition: None,
            })
            .await
    }

    /// `storage.deleteCookies` — by exact name.
    pub async fn delete_cookies_by_name(&self, name: impl Into<String>) -> Result<(), BidiError> {
        self.bidi
            .send(DeleteCookies {
                filter: Some(CookieFilter {
                    name: Some(name.into()),
                    ..CookieFilter::default()
                }),
                partition: None,
            })
            .await?;
        Ok(())
    }

    /// `storage.deleteCookies` — every cookie in the default partition.
    pub async fn delete_all_cookies(&self) -> Result<(), BidiError> {
        self.bidi
            .send(DeleteCookies {
                filter: None,
                partition: None,
            })
            .await?;
        Ok(())
    }
}
