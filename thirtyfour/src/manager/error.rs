use std::time::Duration;

/// Errors returned by the WebDriver manager.
///
/// Converted to [`crate::error::WebDriverError`] (as a `SessionCreateError`
/// variant carrying the manager-specific message) when surfaced through the
/// rest of the crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ManagerError {
    /// Failed to download a driver binary.
    #[error("download failed: {0}")]
    Download(String),

    /// Failed to extract a downloaded archive.
    #[error("extract failed: {0}")]
    Extract(String),

    /// Could not detect the locally-installed browser version.
    #[error("could not detect installed {browser}: {hint}")]
    LocalBrowserNotFound {
        /// Name of the browser that could not be detected.
        browser: &'static str,
        /// Hint shown to the user.
        hint: &'static str,
    },

    /// Browser is not supported by the manager.
    #[error("unsupported browser: {0} (manager supports chrome, chromium, firefox)")]
    UnsupportedBrowser(String),

    /// Browser name is missing from capabilities.
    #[error("browserName missing from capabilities")]
    MissingBrowserName,

    /// `browserVersion` was missing from capabilities and `DriverVersion::FromCapabilities` was used.
    #[error(
        "browserVersion missing from capabilities; required by DriverVersion::FromCapabilities"
    )]
    MissingCapabilityVersion,

    /// A `WebDriverManagerBuilder` was awaited without capabilities preloaded.
    /// This is a programmer error: builders constructed via
    /// [`crate::manager::WebDriverManager::builder`] must terminate with `.build()`
    /// (giving a manager) or `.launch(caps)` on the resulting manager. Only
    /// builders constructed via [`crate::WebDriver::managed`] (which preloads
    /// capabilities) can be awaited directly.
    #[error("WebDriverManagerBuilder awaited without capabilities; use .build() instead")]
    NoCapabilities,

    /// Driver process didn't reach a ready state in time.
    #[error("driver did not become ready within {0:?}")]
    DriverNotReady(Duration),

    /// Failed to spawn the driver process.
    #[error("failed to spawn driver: {0}")]
    Spawn(String),

    /// Failed to acquire the cache lock.
    #[error("cache lock failed: {0}")]
    Lock(String),

    /// Offline mode was requested and the driver was not in cache.
    #[error("offline mode and driver not present in cache")]
    Offline,

    /// I/O error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP error.
    #[error("http: {0}")]
    Http(String),

    /// Failed to parse upstream JSON.
    #[error("parse error: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for ManagerError {
    fn from(e: reqwest::Error) -> Self {
        ManagerError::Http(e.to_string())
    }
}
