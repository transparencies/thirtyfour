//! `webExtension.*` — install and uninstall web extensions in the
//! running browser.
//!
//! Driver support is currently Firefox-only (geckodriver implements
//! both commands; Chromium drivers respond with `unknown error` /
//! `Method not available`). Some browsers install extensions only
//! temporarily — they're removed at the next browser shutdown — so
//! treat the lifetime of the resulting
//! [`ExtensionId`](crate::bidi::ExtensionId) as scoped to the current
//! session unless your driver explicitly persists the install.
//!
//! Extensions can be supplied three ways: an unpacked directory on the
//! local filesystem, a packed archive on the local filesystem (`.crx`,
//! `.xpi`, or plain `.zip`), or base64-encoded archive bytes. See
//! [`ExtensionData`](crate::bidi::modules::web_extension::ExtensionData).
//!
//! See the [W3C `webExtension` module specification][spec] for the
//! canonical definitions.
//!
//! [spec]: https://w3c.github.io/webdriver-bidi/#module-webExtension

use serde::{Deserialize, Serialize};

use crate::bidi::BiDi;
use crate::bidi::command::{BidiCommand, Empty};
use crate::bidi::error::BidiError;
use crate::bidi::ids::ExtensionId;

/// Source of an extension to install. Mirrors the spec's
/// [`webExtension.ExtensionData`][spec] union.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#type-webExtension-ExtensionData
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExtensionData {
    /// Path to an unpacked extension directory on the host filesystem.
    Path {
        /// Filesystem path.
        path: String,
    },
    /// Path to a packed archive (`.crx`, `.xpi`, or plain `.zip`) on
    /// the host filesystem.
    ArchivePath {
        /// Filesystem path.
        path: String,
    },
    /// Base64-encoded archive bytes (same format as `archivePath` but
    /// transmitted inline).
    Base64 {
        /// Base64 payload.
        value: String,
    },
}

/// [`webExtension.install`][spec] — install an extension from the given
/// source.
///
/// Returns the assigned [`ExtensionId`], which can be used with
/// [`Uninstall`] later. Errors:
///
/// - `unsupported operation` — driver doesn't implement
///   `webExtension.install` or doesn't support this `type`.
/// - `invalid web extension` — bytes weren't a valid extension /
///   archive, or the path doesn't exist.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-webExtension-install
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Install {
    /// Where to install the extension from.
    pub extension_data: ExtensionData,
}

impl BidiCommand for Install {
    const METHOD: &'static str = "webExtension.install";
    type Returns = InstallResult;
}

/// Response for [`Install`].
#[derive(Debug, Clone, Deserialize)]
pub struct InstallResult {
    /// Server-assigned extension id. Pass to [`Uninstall`] to remove
    /// the extension later.
    pub extension: ExtensionId,
}

/// [`webExtension.uninstall`][spec] — remove a previously-installed
/// extension.
///
/// Returns `no such web extension` if `extension` is unknown.
///
/// [spec]: https://w3c.github.io/webdriver-bidi/#command-webExtension-uninstall
#[derive(Debug, Clone, Serialize)]
pub struct Uninstall {
    /// Extension id to remove.
    pub extension: ExtensionId,
}

impl BidiCommand for Uninstall {
    const METHOD: &'static str = "webExtension.uninstall";
    type Returns = Empty;
}

/// Convenience facade for the `webExtension.*` module.
///
/// Returned by [`BiDi::web_extension`](crate::bidi::BiDi::web_extension).
/// The three `install_*` shortcuts cover the common cases; for the
/// generic form pass [`ExtensionData`] to [`install`](Self::install).
#[derive(Debug)]
pub struct WebExtensionModule<'a> {
    bidi: &'a BiDi,
}

impl<'a> WebExtensionModule<'a> {
    pub(crate) fn new(bidi: &'a BiDi) -> Self {
        Self {
            bidi,
        }
    }

    /// Install an extension from any [`ExtensionData`] source via
    /// [`webExtension.install`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-webExtension-install
    pub async fn install(&self, extension_data: ExtensionData) -> Result<InstallResult, BidiError> {
        self.bidi
            .send(Install {
                extension_data,
            })
            .await
    }

    /// Install an unpacked extension directory at `path`.
    ///
    /// Convenience for [`install`](Self::install) with
    /// [`ExtensionData::Path`].
    pub async fn install_path(&self, path: impl Into<String>) -> Result<InstallResult, BidiError> {
        self.install(ExtensionData::Path {
            path: path.into(),
        })
        .await
    }

    /// Install a packed extension archive at `path` (`.crx`, `.xpi`, or
    /// `.zip`).
    ///
    /// Convenience for [`install`](Self::install) with
    /// [`ExtensionData::ArchivePath`].
    pub async fn install_archive(
        &self,
        path: impl Into<String>,
    ) -> Result<InstallResult, BidiError> {
        self.install(ExtensionData::ArchivePath {
            path: path.into(),
        })
        .await
    }

    /// Install a base64-encoded extension archive.
    ///
    /// Convenience for [`install`](Self::install) with
    /// [`ExtensionData::Base64`].
    pub async fn install_base64(
        &self,
        value: impl Into<String>,
    ) -> Result<InstallResult, BidiError> {
        self.install(ExtensionData::Base64 {
            value: value.into(),
        })
        .await
    }

    /// Uninstall an extension via [`webExtension.uninstall`][spec].
    ///
    /// [spec]: https://w3c.github.io/webdriver-bidi/#command-webExtension-uninstall
    pub async fn uninstall(&self, extension: ExtensionId) -> Result<(), BidiError> {
        self.bidi
            .send(Uninstall {
                extension,
            })
            .await?;
        Ok(())
    }
}
