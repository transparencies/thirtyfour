//! `storage.*` — cookies and storage partition addressing.
//!
//! BiDi storage commands operate on a [partition][partition-spec] (a
//! cookie / storage jar). When `partition` is omitted the driver uses
//! the session's default partition; supply a
//! [`PartitionDescriptor`](crate::bidi::modules::storage::PartitionDescriptor)
//! to target a specific user context or origin.
//!
//! Cookie values flow through the crate-wide [`Cookie`](crate::Cookie) /
//! [`SameSite`](crate::SameSite) types — the wire shape uses
//! [`network.BytesValue`][bytes-spec] for the cookie body, which is
//! converted to a UTF-8 string when reading and re-encoded when writing.
//!
//! See the [W3C `storage` module specification][spec] for the canonical
//! definitions.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-storage
//! [partition-spec]: https://w3c.github.io/webdriver-bidi/#type-storage-PartitionKey
//! [bytes-spec]: https://w3c.github.io/webdriver-bidi/#type-network-BytesValue

use serde::{Deserialize, Serialize};

use crate::Cookie;
use crate::SameSite;
use crate::bidi::BiDi;
use crate::bidi::command::BidiCommand;
use crate::bidi::error::BidiError;
use crate::common::cookie::bidi::{bytes_string, same_site, string_from_bytes_value};

/// Opaque [`storage.PartitionDescriptor`][spec] — addresses one storage
/// partition.
///
/// Build it with `serde_json::json!` in one of the spec's two forms:
///
/// ```text
/// // Pin to a browsing context's partition.
/// json!({"type":"context","context": "<browsing-context-id>"})
///
/// // Pin to an explicit storage key.
/// json!({
///     "type":"storageKey",
///     "userContext": "<user-context-id>",
///     "sourceOrigin": "https://example.com"
/// })
/// ```
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-storage-PartitionDescriptor
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
/// [`StorageModule::delete_cookies_filtered`]. Mirrors the spec's
/// [`storage.CookieFilter`][spec] type.
///
/// Each field, if `Some`, restricts the matched cookies (multiple
/// fields are AND-combined). Fields that share names with [`Cookie`]
/// match the same field on the wire.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-storage-CookieFilter
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

/// [`storage.getCookies`][spec] — list cookies in a partition,
/// optionally filtered.
///
/// Most callers should use the
/// [`get_cookies`](StorageModule::get_cookies) /
/// [`get_cookies_by_name`](StorageModule::get_cookies_by_name) /
/// [`get_cookies_filtered`](StorageModule::get_cookies_filtered)
/// facade methods, which handle the [`Cookie`] / wire-format
/// conversion. Build [`GetCookies`] directly only when you need to
/// supply a [`PartitionDescriptor`].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-storage-getCookies
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetCookies {
    /// Optional filter — only matching cookies are returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<CookieFilter>,
    /// Optional partition. `None` uses the session default.
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
    /// Matching cookies, converted to the crate-wide [`Cookie`] type.
    pub cookies: Vec<Cookie>,
    /// Resolved [`storage.PartitionKey`][spec] of the partition the
    /// cookies were read from.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-storage-PartitionKey
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
    /// Resolved [`storage.PartitionKey`][spec] the cookie was written to.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-storage-PartitionKey
    #[serde(default)]
    pub partition_key: serde_json::Value,
}

/// [`storage.deleteCookies`][spec] — remove every cookie matching a
/// filter from a partition.
///
/// Most callers should use the
/// [`delete_cookies_by_name`](StorageModule::delete_cookies_by_name) /
/// [`delete_all_cookies`](StorageModule::delete_all_cookies) /
/// [`delete_cookies_filtered`](StorageModule::delete_cookies_filtered)
/// facade methods. Build [`DeleteCookies`] directly only when you need
/// to supply a [`PartitionDescriptor`].
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-storage-deleteCookies
#[derive(Debug, Clone, Default, Serialize)]
pub struct DeleteCookies {
    /// Filter — every matching cookie is removed. `None` clears the
    /// entire partition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<CookieFilter>,
    /// Optional partition. `None` uses the session default.
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
    /// Resolved [`storage.PartitionKey`][spec] of the affected partition.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#type-storage-PartitionKey
    #[serde(default)]
    pub partition_key: serde_json::Value,
}

/// Convenience facade for the `storage.*` module.
///
/// Returned by [`BiDi::storage`](crate::bidi::BiDi::storage). Every
/// method on this facade targets the session's default storage
/// partition. To address a specific partition, build the command struct
/// (e.g. [`GetCookies`]) directly and supply a [`PartitionDescriptor`].
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

    /// Return every cookie in the default partition via
    /// [`storage.getCookies`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-storage-getCookies
    pub async fn get_cookies(&self) -> Result<GetCookiesResult, BidiError> {
        self.get_cookies_filtered(None).await
    }

    /// Return cookies whose name matches `name` (default partition).
    pub async fn get_cookies_by_name(
        &self,
        name: impl Into<String>,
    ) -> Result<GetCookiesResult, BidiError> {
        self.get_cookies_filtered(Some(CookieFilter::by_name(name))).await
    }

    /// Return cookies matching an arbitrary [`CookieFilter`] (default
    /// partition).
    ///
    /// Wraps [`storage.getCookies`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-storage-getCookies
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

    /// Set a cookie in the default partition via
    /// [`storage.setCookie`][spec].
    ///
    /// `cookie.domain` is required by BiDi — calls without one return
    /// `invalid argument` locally without making a wire request, since
    /// the spec rejects them anyway.
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-storage-setCookie
    pub async fn set_cookie(&self, cookie: Cookie) -> Result<SetCookieResult, BidiError> {
        let cookie = WirePartialCookie::from_cookie(cookie)?;
        self.bidi
            .send(SetCookie {
                cookie,
                partition: None,
            })
            .await
    }

    /// Delete every cookie with name `name` (default partition).
    pub async fn delete_cookies_by_name(&self, name: impl Into<String>) -> Result<(), BidiError> {
        self.delete_cookies_filtered(Some(CookieFilter::by_name(name))).await
    }

    /// Delete every cookie in the default partition.
    pub async fn delete_all_cookies(&self) -> Result<(), BidiError> {
        self.delete_cookies_filtered(None).await
    }

    /// Delete every cookie matching `filter` (default partition).
    ///
    /// Wraps [`storage.deleteCookies`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-storage-deleteCookies
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
