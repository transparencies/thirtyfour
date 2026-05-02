//! `Browser` domain — top-level browser process information and control.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::common::protocol::string_enum;
use crate::error::WebDriverResult;

string_enum! {
    /// Download policy for [`SetDownloadBehavior`].
    pub enum DownloadBehavior {
        /// Block downloads.
        Deny = "deny",
        /// Allow downloads, saving with their suggested filename.
        Allow = "allow",
        /// Allow downloads but rename each file using the request URL hash.
        AllowAndName = "allowAndName",
        /// Restore the browser's default behaviour.
        Default = "default",
    }
}

/// `Browser.getVersion`
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetVersion;

/// Response for [`GetVersion`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    /// Protocol version (e.g. `"1.3"`).
    pub protocol_version: String,
    /// Product name and version (e.g. `"Chrome/121.0.0.0"`).
    pub product: String,
    /// Revision string (e.g. `"@abcdef"`).
    pub revision: String,
    /// Browser user agent.
    pub user_agent: String,
    /// V8 version.
    pub js_version: String,
}

impl CdpCommand for GetVersion {
    const METHOD: &'static str = "Browser.getVersion";
    type Returns = VersionInfo;
}

/// `Browser.close` — closes the browser gracefully.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Close;

impl CdpCommand for Close {
    const METHOD: &'static str = "Browser.close";
    type Returns = Empty;
}

/// `Browser.setDownloadBehavior` — controls how downloads are handled.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDownloadBehavior {
    /// Download behavior.
    pub behavior: DownloadBehavior,
    /// Browser context to apply the change to (omit for default context).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<String>,
    /// Directory to download files into. Required when `behavior` is
    /// [`DownloadBehavior::Allow`] or [`DownloadBehavior::AllowAndName`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
    /// Whether to emit `Browser.downloadWillBegin` events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events_enabled: Option<bool>,
}

impl CdpCommand for SetDownloadBehavior {
    const METHOD: &'static str = "Browser.setDownloadBehavior";
    type Returns = Empty;
}

/// Domain facade returned by [`Cdp::browser`].
#[derive(Debug)]
pub struct BrowserDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> BrowserDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Browser.getVersion`.
    pub async fn get_version(&self) -> WebDriverResult<VersionInfo> {
        self.cdp.send(GetVersion).await
    }

    /// `Browser.close` — close the browser gracefully.
    pub async fn close(&self) -> WebDriverResult<()> {
        self.cdp.send(Close).await?;
        Ok(())
    }

    /// `Browser.setDownloadBehavior` — control how downloads are handled.
    pub async fn set_download_behavior(
        &self,
        behavior: DownloadBehavior,
        download_path: Option<String>,
    ) -> WebDriverResult<()> {
        self.cdp
            .send(SetDownloadBehavior {
                behavior,
                browser_context_id: None,
                download_path,
                events_enabled: None,
            })
            .await?;
        Ok(())
    }
}
