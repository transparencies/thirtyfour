use std::collections::HashMap;
use std::future::{Future, IntoFuture};
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, Weak};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::Mutex;
use url::Url;

use crate::Capabilities;
use crate::common::capabilities::desiredcapabilities::CapabilitiesHelper;
use crate::common::config::WebDriverConfig;
use crate::error::{WebDriverError, WebDriverResult};
use crate::session::DriverGuard;
use crate::session::create::start_session;
use crate::session::handle::SessionHandle;
use crate::session::http::create_reqwest_client;
use crate::web_driver::WebDriver;

use super::browser::{BrowserKind, detect_local_version};
use super::download::{DownloadConfig, Mirror, ensure_driver, resolve_version};
use super::error::ManagerError;
use super::process::{ManagedDriverProcess, SpawnConfig, StdioMode};
use super::version::DriverVersion;

const DEFAULT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_READY_TIMEOUT: Duration = Duration::from_secs(30);

/// Default cache directory: `<cache_dir>/thirtyfour/drivers`, falling back to the
/// system temp dir if no cache dir is available.
fn default_cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("thirtyfour").join("drivers")
}

/// Process-wide default manager.
fn shared_manager() -> &'static Arc<WebDriverManager> {
    static SHARED: OnceLock<Arc<WebDriverManager>> = OnceLock::new();
    SHARED.get_or_init(|| WebDriverManager::builder().build())
}

/// Auto-download and lifetime-managed local WebDriver process manager.
///
/// See the [module documentation](super) for the high-level usage; the most
/// common entry points are [`WebDriver::managed`] (one-shot) and
/// [`WebDriverManager::builder`] (for sharing one manager across many sessions).
///
/// [`WebDriver::managed`]: crate::WebDriver::managed
pub struct WebDriverManager {
    pub(crate) cfg: ResolvedConfig,
    /// HTTP client used for fetching upstream metadata and binaries.
    download_client: reqwest::Client,
    /// Map from `(browser, resolved_version)` to a Weak handle on a live driver.
    drivers: Mutex<HashMap<DriverKey, Weak<ManagedDriverProcess>>>,
}

impl std::fmt::Debug for WebDriverManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebDriverManager").field("cfg", &self.cfg).finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub(crate) struct ResolvedConfig {
    pub version: DriverVersion,
    pub cache_dir: PathBuf,
    pub host: IpAddr,
    pub download_timeout: Duration,
    pub ready_timeout: Duration,
    pub offline: bool,
    pub mirror: Mirror,
    pub stdio: StdioMode,
}

impl std::fmt::Debug for ResolvedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedConfig")
            .field("version", &self.version)
            .field("cache_dir", &self.cache_dir)
            .field("host", &self.host)
            .field("download_timeout", &self.download_timeout)
            .field("ready_timeout", &self.ready_timeout)
            .field("offline", &self.offline)
            .field("stdio", &self.stdio)
            .finish_non_exhaustive()
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub(super) struct DriverKey {
    pub(super) browser: BrowserKind,
    pub(super) version: String,
    pub(super) host: IpAddr,
}

impl DriverGuard for ManagedDriverProcess {}

/// Builder for [`WebDriverManager`]. The same type is used both via
/// `WebDriverManager::builder()` and (with capabilities preloaded) via
/// [`WebDriver::managed`]. See the [module documentation](super) for examples.
///
/// [`WebDriver::managed`]: crate::WebDriver::managed
#[derive(Debug, Clone)]
pub struct WebDriverManagerBuilder {
    pub(crate) version: DriverVersion,
    pub(crate) cache_dir: Option<PathBuf>,
    pub(crate) host: Option<IpAddr>,
    pub(crate) download_timeout: Option<Duration>,
    pub(crate) ready_timeout: Option<Duration>,
    pub(crate) offline: Option<bool>,
    pub(crate) mirror: Option<Mirror>,
    pub(crate) stdio: Option<StdioMode>,
    /// Set when constructed via `WebDriver::managed(caps)`.
    pub(crate) preloaded_caps: Option<Capabilities>,
}

impl Default for WebDriverManagerBuilder {
    fn default() -> Self {
        Self {
            version: DriverVersion::default(),
            cache_dir: None,
            host: None,
            download_timeout: None,
            ready_timeout: None,
            offline: None,
            mirror: None,
            stdio: None,
            preloaded_caps: None,
        }
    }
}

impl WebDriverManagerBuilder {
    /// Pick a driver version. See [`DriverVersion`] for variants.
    pub fn version(mut self, v: DriverVersion) -> Self {
        self.version = v;
        self
    }

    /// Use the latest stable driver from upstream metadata.
    pub fn latest(self) -> Self {
        self.version(DriverVersion::Latest)
    }

    /// Probe the locally-installed browser and use a matching driver. This is
    /// the default; calling it explicitly is a no-op.
    pub fn match_local(self) -> Self {
        self.version(DriverVersion::MatchLocalBrowser)
    }

    /// Read `browserVersion` from the capabilities supplied to `launch()` /
    /// `WebDriver::managed()`.
    pub fn from_caps(self) -> Self {
        self.version(DriverVersion::FromCapabilities)
    }

    /// Pin a specific version (full like `"126.0.6478.126"` or major-only like `"126"`).
    pub fn exact(self, v: impl Into<String>) -> Self {
        self.version(DriverVersion::Exact(v.into()))
    }

    /// Override the cache directory. Default: `<system cache dir>/thirtyfour/drivers`.
    pub fn cache_dir(mut self, p: PathBuf) -> Self {
        self.cache_dir = Some(p);
        self
    }

    /// Override the host the driver binds to. Default: `127.0.0.1`.
    pub fn host(mut self, h: IpAddr) -> Self {
        self.host = Some(h);
        self
    }

    /// Override the timeout for upstream metadata + binary downloads. Default: 60s.
    pub fn download_timeout(mut self, d: Duration) -> Self {
        self.download_timeout = Some(d);
        self
    }

    /// Override the timeout for waiting on driver `/status` readiness. Default: 30s.
    pub fn ready_timeout(mut self, d: Duration) -> Self {
        self.ready_timeout = Some(d);
        self
    }

    /// Refuse to download anything; require the driver to already be in cache.
    pub fn offline(self) -> Self {
        self.offline_mode(true)
    }

    /// Allow downloads. This is the default.
    pub fn online(self) -> Self {
        self.offline_mode(false)
    }

    /// Set offline mode explicitly.
    pub fn offline_mode(mut self, yes: bool) -> Self {
        self.offline = Some(yes);
        self
    }

    /// Override the Chrome-for-Testing metadata base URL.
    pub fn chrome_metadata_mirror(mut self, base: Url) -> Self {
        let m = self.mirror.get_or_insert_with(Mirror::default);
        m.chrome_metadata = base;
        self
    }

    /// Override the geckodriver binary download base URL.
    pub fn geckodriver_downloads_mirror(mut self, base: Url) -> Self {
        let m = self.mirror.get_or_insert_with(Mirror::default);
        m.geckodriver_downloads = base;
        self
    }

    /// Override the msedgedriver download host.
    pub fn edge_downloads_mirror(mut self, base: Url) -> Self {
        let m = self.mirror.get_or_insert_with(Mirror::default);
        m.edge_downloads = base;
        self
    }

    /// Set how driver-process stdout/stderr is handled. Default:
    /// [`StdioMode::Tracing`].
    pub fn stdio(mut self, mode: StdioMode) -> Self {
        self.stdio = Some(mode);
        self
    }

    /// Build the manager.
    pub fn build(self) -> Arc<WebDriverManager> {
        let cfg = ResolvedConfig {
            version: self.version,
            cache_dir: self.cache_dir.unwrap_or_else(default_cache_dir),
            host: self.host.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            download_timeout: self.download_timeout.unwrap_or(DEFAULT_DOWNLOAD_TIMEOUT),
            ready_timeout: self.ready_timeout.unwrap_or(DEFAULT_READY_TIMEOUT),
            offline: self.offline.unwrap_or(false),
            mirror: self.mirror.unwrap_or_default(),
            stdio: self.stdio.unwrap_or_default(),
        };
        Arc::new(WebDriverManager {
            download_client: reqwest::Client::builder()
                .build()
                .expect("default reqwest client should always build"),
            cfg,
            drivers: Mutex::new(HashMap::new()),
        })
    }

    /// `true` if every configuration field is at its default. Lets
    /// `WebDriver::managed` route through the process-wide shared manager when
    /// the user hasn't overridden anything.
    fn is_all_defaults(&self) -> bool {
        self.version == DriverVersion::default()
            && self.cache_dir.is_none()
            && self.host.is_none()
            && self.download_timeout.is_none()
            && self.ready_timeout.is_none()
            && self.offline.is_none()
            && self.mirror.is_none()
            && self.stdio.is_none()
    }
}

impl IntoFuture for WebDriverManagerBuilder {
    type Output = WebDriverResult<WebDriver>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let caps = self
                .preloaded_caps
                .clone()
                .ok_or_else(|| WebDriverError::from(ManagerError::NoCapabilities))?;
            let mgr = if self.is_all_defaults() {
                Arc::clone(shared_manager())
            } else {
                self.build()
            };
            mgr.launch(caps).await
        })
    }
}

impl WebDriverManager {
    /// Construct an empty builder.
    pub fn builder() -> WebDriverManagerBuilder {
        WebDriverManagerBuilder::default()
    }

    /// Process-wide default manager. Used by [`WebDriver::managed`] when no
    /// configuration is supplied. Multiple calls return the same `Arc`.
    ///
    /// [`WebDriver::managed`]: crate::WebDriver::managed
    pub fn shared() -> Arc<Self> {
        Arc::clone(shared_manager())
    }

    /// Spawn (or reuse) the appropriate driver and start a session.
    ///
    /// If a driver process for the same `(browser, resolved_version, host)` is
    /// already alive (held by another `WebDriver`), it's reused. Otherwise a
    /// new one is downloaded if needed and spawned.
    pub async fn launch(
        self: &Arc<Self>,
        capabilities: impl Into<Capabilities>,
    ) -> WebDriverResult<WebDriver> {
        let caps: Capabilities = capabilities.into();
        let driver = self.ensure_driver(&caps).await.map_err(WebDriverError::from)?;
        let server_url: Url = driver
            .url()
            .parse()
            .map_err(|e| WebDriverError::ParseError(format!("invalid driver url: {e}")))?;

        let config = WebDriverConfig::default();
        let client = create_reqwest_client(config.reqwest_timeout);
        let client_arc: Arc<dyn crate::session::http::HttpClient> = Arc::new(client);
        let session_id = start_session(client_arc.as_ref(), &server_url, &config, caps).await?;
        let guard: Arc<dyn DriverGuard> = driver;
        let handle = SessionHandle::new_with_config_and_guard(
            client_arc,
            server_url,
            session_id,
            config,
            Some(guard),
        )?;
        Ok(WebDriver {
            handle: Arc::new(handle),
        })
    }

    /// Ensure a driver is running and return an `Arc<ManagedDriverProcess>`
    /// whose existence keeps the child alive.
    async fn ensure_driver(
        &self,
        caps: &Capabilities,
    ) -> Result<Arc<ManagedDriverProcess>, ManagerError> {
        let browser = BrowserKind::from_capabilities(caps)?;

        // For MatchLocalBrowser we probe the binary up front (potentially using a
        // capabilities-supplied path).
        let local = match self.cfg.version {
            DriverVersion::MatchLocalBrowser => {
                let custom = browser.binary_from_caps(caps);
                Some(detect_local_version(browser, custom.as_deref())?)
            }
            _ => None,
        };
        let caps_version = caps._get("browserVersion").and_then(Value::as_str);

        let download_cfg = DownloadConfig {
            cache_dir: self.cfg.cache_dir.clone(),
            mirror: self.cfg.mirror.clone(),
            download_timeout: self.cfg.download_timeout,
            offline: self.cfg.offline,
        };
        let resolved = resolve_version(
            &self.download_client,
            &download_cfg,
            browser,
            &self.cfg.version,
            local.as_deref(),
            caps_version,
        )
        .await?;

        let key = DriverKey {
            browser,
            version: resolved.clone(),
            host: self.cfg.host,
        };

        // Fast path: another live `Arc<ManagedDriver>` keyed the same way.
        {
            let map = self.drivers.lock().await;
            if let Some(existing) = map.get(&key).and_then(Weak::upgrade) {
                return Ok(existing);
            }
        }

        let driver_path =
            ensure_driver(&self.download_client, &download_cfg, browser, &resolved).await?;
        let process = ManagedDriverProcess::spawn(
            &driver_path.binary,
            browser,
            &SpawnConfig {
                host: self.cfg.host,
                ready_timeout: self.cfg.ready_timeout,
                stdio: self.cfg.stdio,
            },
        )
        .await?;

        let arc = Arc::new(process);
        let mut map = self.drivers.lock().await;
        // Re-check after re-locking — another caller may have raced us.
        if let Some(existing) = map.get(&key).and_then(Weak::upgrade) {
            return Ok(existing);
        }
        map.insert(key, Arc::downgrade(&arc));
        Ok(arc)
    }
}
