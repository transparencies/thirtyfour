use std::fmt::Formatter;
use std::future::{Future, IntoFuture};
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use http::HeaderValue;

use crate::Capabilities;
use crate::common::config::{WebDriverConfig, WebDriverConfigBuilder};
use crate::error::WebDriverResult;
use crate::extensions::query::IntoElementPoller;
use crate::prelude::WebDriverError;
use crate::session::create::start_session;
use crate::session::handle::SessionHandle;
use crate::session::http::HttpClient;
#[cfg(feature = "reqwest")]
use crate::session::http::create_reqwest_client;

/// The `WebDriver` struct encapsulates an async Selenium WebDriver browser
/// session.
///
/// # Example:
/// ```no_run
/// use thirtyfour::prelude::*;
/// # use thirtyfour::support::block_on;
///
/// # fn main() -> anyhow::Result<()> {
/// #     block_on(async {
/// let caps = DesiredCapabilities::firefox();
/// let driver = WebDriver::new("http://localhost:4444", caps).await?;
/// driver.goto("https://www.rust-lang.org/").await?;
/// // Always remember to close the session.
/// driver.quit().await?;
/// #         Ok(())
/// #     })
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct WebDriver {
    pub(crate) handle: Arc<SessionHandle>,
}

#[derive(Debug, thiserror::Error)]
#[error("Webdriver has already quit, can't leak an already quit driver")]
pub struct AlreadyQuit(pub(crate) ());

impl WebDriver {
    /// Create a new WebDriver as follows:
    ///
    /// # Example
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # use thirtyfour::support::block_on;
    /// #
    /// # fn main() -> WebDriverResult<()> {
    /// #     block_on(async {
    /// let caps = DesiredCapabilities::firefox();
    /// let driver = WebDriver::new("http://localhost:4444", caps).await?;
    /// #         driver.quit().await?;
    /// #         Ok(())
    /// #     })
    /// # }
    /// ```
    ///
    /// To customize timeouts, the user agent, the element poller, or supply a
    /// custom [`HttpClient`], use [`WebDriver::builder`] instead.
    ///
    /// ## Using Selenium Server
    /// - For selenium 3.x, you need to also add "/wd/hub" to the end of the url
    ///   (e.g. "http://localhost:4444/wd/hub")
    /// - For selenium 4.x and later, no path should be needed on the url.
    ///
    /// ## Troubleshooting
    ///
    /// - If the webdriver appears to freeze or give no response, please check that the
    ///   capabilities' object is of the correct type for that webdriver.
    pub async fn new<S, C>(server_url: S, capabilities: C) -> WebDriverResult<Self>
    where
        S: Into<String>,
        C: Into<Capabilities>,
    {
        Self::builder(server_url, capabilities).await
    }

    /// Construct a [`WebDriverBuilder`] for advanced session configuration —
    /// custom timeouts, user agent, element poller, or a user-supplied
    /// [`HttpClient`]. Awaiting the builder constructs the session.
    ///
    /// For the default configuration, prefer [`WebDriver::new`].
    ///
    /// # Example
    /// ```no_run
    /// # use std::time::Duration;
    /// # use thirtyfour::prelude::*;
    /// # async fn run() -> WebDriverResult<()> {
    /// let caps = DesiredCapabilities::chrome();
    /// let driver = WebDriver::builder("http://localhost:4444", caps)
    ///     .request_timeout(Duration::from_secs(30))
    ///     .user_agent("my-app/1.0")
    ///     .await?;
    /// # driver.quit().await }
    /// ```
    pub fn builder<S, C>(server_url: S, capabilities: C) -> WebDriverBuilder
    where
        S: Into<String>,
        C: Into<Capabilities>,
    {
        WebDriverBuilder::new(server_url, capabilities)
    }

    /// Returns a reference to the underlying [`SessionHandle`]. Useful for
    /// extension code that needs to clone the handle (e.g. building a
    /// [`WebElement`] from a JSON value via [`WebElement::from_json`]).
    /// Most users won't need this — methods on `SessionHandle` are already
    /// reachable through `Deref` (e.g. `driver.session_id()`).
    ///
    /// [`WebElement`]: crate::WebElement
    /// [`WebElement::from_json`]: crate::WebElement::from_json
    pub fn handle(&self) -> &Arc<SessionHandle> {
        &self.handle
    }

    /// Clone this `WebDriver` keeping the session handle, but supplying a new `WebDriverConfig`.
    ///
    /// This still uses the same underlying client, and still controls the same browser
    /// session, but uses a different `WebDriverConfig` for this instance.
    ///
    /// This is useful in cases where you want to specify a custom poller configuration (or
    /// some other configuration option) for only one instance of `WebDriver`.
    pub fn clone_with_config(&self, config: WebDriverConfig) -> Self {
        Self {
            handle: Arc::new(self.handle.clone_with_config(config)),
        }
    }

    /// Plug-and-play constructor that auto-downloads the appropriate driver
    /// (chromedriver / geckodriver), spawns it locally, and tears it down when
    /// the last connected `WebDriver` is dropped.
    ///
    /// Returns a [`WebDriverManagerBuilder`] pre-loaded with `caps`. Awaiting
    /// the builder constructs the session; chained methods customize the
    /// version or other settings before the await:
    ///
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # async fn run() -> WebDriverResult<()> {
    /// // Default (matches the locally-installed browser).
    /// let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;
    ///
    /// // Override the version inline using the same builder shape used by
    /// // `WebDriverManager::builder()`.
    /// # let caps = DesiredCapabilities::chrome();
    /// let driver = WebDriver::managed(caps).latest().await?;
    /// # Ok(()) }
    /// ```
    ///
    /// Each call constructs its own [`WebDriverManager`] and spawns its own
    /// driver subprocess. To share one manager (and its driver subprocesses)
    /// across many sessions, construct it explicitly via
    /// [`WebDriverManager::builder`] and call `.launch(caps)` on it for each
    /// session.
    ///
    /// [`WebDriverManagerBuilder`]: crate::manager::WebDriverManagerBuilder
    /// [`WebDriverManager`]: crate::manager::WebDriverManager
    /// [`WebDriverManager::builder`]: crate::manager::WebDriverManager::builder
    #[cfg(feature = "manager")]
    pub fn managed<C>(capabilities: C) -> crate::manager::WebDriverManagerBuilder
    where
        C: Into<Capabilities>,
    {
        let mut builder = crate::manager::WebDriverManager::builder();
        builder.preloaded_caps = Some(capabilities.into());
        builder
    }

    /// Open (or return the already-cached) WebDriver BiDi handle for this
    /// session.
    ///
    /// Requires `webSocketUrl: true` to have been set on the capabilities
    /// before [`WebDriver::new`] (use
    /// [`CapabilitiesHelper::enable_bidi`](crate::CapabilitiesHelper::enable_bidi))
    /// so the driver returns a `webSocketUrl` string the BiDi handle can
    /// dial.
    ///
    /// Lazy: the WebSocket isn't connected until the first call. Subsequent
    /// calls return the same cached handle (cloning is cheap).
    ///
    /// # Example
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # async fn run() -> WebDriverResult<()> {
    /// let mut caps = DesiredCapabilities::chrome();
    /// caps.enable_bidi()?;
    /// let driver = WebDriver::new("http://localhost:4444", caps).await?;
    ///
    /// let bidi = driver.bidi().await?;
    /// let status = bidi.session().status().await?;
    /// println!("BiDi ready: {}", status.ready);
    /// # driver.quit().await }
    /// ```
    #[cfg(feature = "bidi")]
    pub async fn bidi(&self) -> WebDriverResult<crate::bidi::BiDi> {
        let handle = self.handle.clone();
        let cell = self.handle.bidi_cell().clone();
        let bidi = cell
            .get_or_try_init(|| async move { crate::bidi::BiDi::connect(handle).await })
            .await?;
        Ok(bidi.clone())
    }

    /// Get a Chrome DevTools Protocol handle for this session.
    ///
    /// Cheap to call (just clones the underlying `Arc<SessionHandle>`).
    /// Works on any Chromium-based driver session (chromedriver,
    /// msedgedriver, Brave, Opera, etc.) — non-Chromium drivers will return
    /// errors when CDP commands are issued.
    ///
    /// # Example
    /// ```no_run
    /// # use thirtyfour::prelude::*;
    /// # async fn run() -> WebDriverResult<()> {
    /// let driver = WebDriver::new("http://localhost:4444", DesiredCapabilities::chrome()).await?;
    /// let info = driver.cdp().browser().get_version().await?;
    /// println!("user agent: {}", info.user_agent);
    /// # driver.quit().await }
    /// ```
    ///
    /// For event subscription, upgrade to [`crate::cdp::CdpSession`] via
    /// [`Cdp::connect`] (feature `cdp-events`).
    ///
    /// [`Cdp::connect`]: crate::cdp::Cdp::connect
    #[cfg(feature = "cdp")]
    pub fn cdp(&self) -> crate::cdp::Cdp {
        crate::cdp::Cdp::new(self.handle.clone())
    }

    /// Identifier of the driver process serving this session, if it was
    /// launched via [`WebDriverManager`]. Returns `None` for sessions
    /// constructed with [`WebDriver::new`] (i.e. talking to an externally-
    /// managed driver server).
    ///
    /// This identifier matches the [`DriverLogLine::driver_id`] field on log
    /// events emitted via [`WebDriverManager::on_driver_log`], so a closure
    /// registered manager-wide can filter to "just this session's driver".
    ///
    /// [`WebDriverManager`]: crate::manager::WebDriverManager
    /// [`DriverLogLine::driver_id`]: crate::manager::DriverLogLine
    /// [`WebDriverManager::on_driver_log`]: crate::manager::WebDriverManager::on_driver_log
    #[cfg(feature = "manager")]
    pub fn driver_id(&self) -> Option<crate::manager::DriverId> {
        let guard = self.handle.driver_guard()?;
        let session_guard =
            guard.as_any().downcast_ref::<crate::manager::manager_internal::SessionGuard>()?;
        Some(session_guard.driver.driver_id)
    }

    /// Subscribe to log lines from just this session's driver process. Returns
    /// `None` if this session wasn't launched via [`WebDriverManager`];
    /// otherwise an RAII subscription whose drop unsubscribes.
    ///
    /// Lines also continue flowing to any subscribers attached at the manager
    /// level (via [`WebDriverManager::on_driver_log`]).
    ///
    /// [`WebDriverManager`]: crate::manager::WebDriverManager
    /// [`WebDriverManager::on_driver_log`]: crate::manager::WebDriverManager::on_driver_log
    #[cfg(feature = "manager")]
    pub fn on_driver_log<F>(&self, f: F) -> Option<crate::manager::DriverLogSubscription>
    where
        F: Fn(&crate::manager::DriverLogLine) + Send + Sync + 'static,
    {
        let guard = self.handle.driver_guard()?;
        let session_guard =
            guard.as_any().downcast_ref::<crate::manager::manager_internal::SessionGuard>()?;
        Some(session_guard.driver.subscribe_log(f))
    }

    /// End the webdriver session and close the browser.
    ///
    /// **NOTE:** Although `WebDriver` does close when all instances go out of scope.
    ///           When this happens it blocks the current executor,
    ///           therefore, if you know when a webdriver is no longer used/required
    ///           call this method and await it to more or less "asynchronously drop" it
    ///           this also allows you to catch errors during quitting,
    ///           and possibly panic or report back to the user
    pub async fn quit(self) -> WebDriverResult<()> {
        self.handle.quit().await
    }

    /// Leak the webdriver session and prevent it from being closed,
    /// use this if you don't want your driver to automatically close
    pub fn leak(self) -> Result<(), AlreadyQuit> {
        self.handle.leak()
    }
}

/// The Deref implementation allows the WebDriver to "fall back" to SessionHandle and
/// exposes all the methods there without requiring us to use an async_trait.
/// See documentation at the top of this module for more details on the design.
impl Deref for WebDriver {
    type Target = Arc<SessionHandle>;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

/// Builder for [`WebDriver`] sessions that talk to an externally-managed
/// driver (e.g. a Selenium Grid, a manually-launched chromedriver, or any
/// W3C-compatible WebDriver server).
///
/// Construct via [`WebDriver::builder`]. Awaiting the builder opens the
/// session.
///
/// # Example
/// ```no_run
/// # use std::time::Duration;
/// # use thirtyfour::prelude::*;
/// # async fn run() -> WebDriverResult<()> {
/// let caps = DesiredCapabilities::chrome();
/// let driver = WebDriver::builder("http://localhost:4444", caps)
///     .request_timeout(Duration::from_secs(30))
///     .user_agent("my-app/1.0")
///     .await?;
/// # driver.quit().await }
/// ```
///
/// For driver-managed sessions (auto-download + spawn), use
/// [`WebDriver::managed`] instead.
pub struct WebDriverBuilder {
    server_url: String,
    capabilities: Capabilities,
    config: WebDriverConfigBuilder,
    client: Option<Box<dyn HttpClient>>,
}

impl std::fmt::Debug for WebDriverBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebDriverBuilder")
            .field("server_url", &self.server_url)
            .field("capabilities", &self.capabilities)
            .field("config", &self.config)
            .field("client", &self.client.as_ref().map(|_| "<custom>"))
            .finish()
    }
}

impl WebDriverBuilder {
    fn new<S, C>(server_url: S, capabilities: C) -> Self
    where
        S: Into<String>,
        C: Into<Capabilities>,
    {
        Self {
            server_url: server_url.into(),
            capabilities: capabilities.into(),
            config: WebDriverConfig::builder(),
            client: None,
        }
    }

    /// Set the `Connection: keep-alive` header on outgoing requests.
    /// Default: `true`.
    pub fn keep_alive(mut self, keep_alive: bool) -> Self {
        self.config = self.config.keep_alive(keep_alive);
        self
    }

    /// Set the element poller used by [`WebDriver::query`] and
    /// [`WebElement::wait_until`].
    ///
    /// [`WebDriver::query`]: crate::extensions::query::ElementQueryable::query
    /// [`WebElement::wait_until`]: crate::extensions::query::ElementWaitable::wait_until
    pub fn poller(mut self, poller: Arc<dyn IntoElementPoller + Send + Sync>) -> Self {
        self.config = self.config.poller(poller);
        self
    }

    /// Override the `User-Agent` header sent with WebDriver requests.
    /// Default: `WebDriverConfig::DEFAULT_USER_AGENT`.
    pub fn user_agent<V>(mut self, user_agent: V) -> Self
    where
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<WebDriverError>,
    {
        self.config = self.config.user_agent(user_agent);
        self
    }

    /// Override the per-request HTTP timeout. Default: `120s`. Applied to the
    /// default reqwest-based client; custom [`HttpClient`] implementations
    /// supplied via [`WebDriverBuilder::client`] may also honour it.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.config = self.config.request_timeout(timeout);
        self
    }

    /// Use a custom [`HttpClient`] instead of the default reqwest-based one.
    /// Useful for plugging in a proxy, custom TLS configuration, or running
    /// without the `reqwest` feature.
    pub fn client(mut self, client: impl HttpClient + 'static) -> Self {
        self.client = Some(Box::new(client));
        self
    }

    /// Open the session.
    pub async fn connect(self) -> WebDriverResult<WebDriver> {
        let config = self.config.build()?;

        let client: Arc<dyn HttpClient> = match self.client {
            Some(c) => Arc::from(c),
            None => {
                #[cfg(feature = "reqwest")]
                {
                    Arc::new(create_reqwest_client(config.request_timeout))
                }
                #[cfg(not(feature = "reqwest"))]
                {
                    Arc::new(crate::session::http::null_client::create_null_client())
                }
            }
        };

        let server_url = self
            .server_url
            .parse()
            .map_err(|e| WebDriverError::ParseError(format!("invalid url: {e}")))?;
        let started =
            start_session(client.as_ref(), &server_url, &config, self.capabilities).await?;

        let handle = SessionHandle::new_with_config_guard_and_caps(
            client,
            server_url,
            started.session_id,
            config,
            None,
            started.capabilities,
        )?;
        Ok(WebDriver {
            handle: Arc::new(handle),
        })
    }
}

impl IntoFuture for WebDriverBuilder {
    type Output = WebDriverResult<WebDriver>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.connect())
    }
}
