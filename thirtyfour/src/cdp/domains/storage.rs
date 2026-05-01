//! `Storage` domain — origin-scoped storage controls.

use serde::Serialize;

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::error::WebDriverResult;

/// `Storage.clearDataForOrigin`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearDataForOrigin {
    /// Origin URL.
    pub origin: String,
    /// Comma-separated list of storage types to clear: `"appcache"`, `"cookies"`,
    /// `"file_systems"`, `"indexeddb"`, `"local_storage"`, `"shader_cache"`,
    /// `"websql"`, `"service_workers"`, `"cache_storage"`, `"all"`.
    pub storage_types: String,
}
impl CdpCommand for ClearDataForOrigin {
    const METHOD: &'static str = "Storage.clearDataForOrigin";
    type Returns = Empty;
}

/// `Storage.clearCookies` for a given browser context.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearCookies {
    /// Browser context id (omit for default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<String>,
}
impl CdpCommand for ClearCookies {
    const METHOD: &'static str = "Storage.clearCookies";
    type Returns = Empty;
}

/// Domain facade returned by [`Cdp::storage`].
#[derive(Debug)]
pub struct StorageDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> StorageDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Storage.clearDataForOrigin` for all storage types.
    pub async fn clear_all_data_for_origin(
        &self,
        origin: impl Into<String>,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(ClearDataForOrigin {
                origin: origin.into(),
                storage_types: "all".to_string(),
            })
            .await?;
        Ok(())
    }

    /// `Storage.clearCookies` for the default browser context.
    pub async fn clear_cookies(&self) -> WebDriverResult<()> {
        self.cdp.send(ClearCookies::default()).await?;
        Ok(())
    }
}
