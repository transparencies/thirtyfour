use std::ops::Deref;
use std::sync::Arc;

use crate::Capabilities;
use crate::common::config::WebDriverConfig;
use crate::error::WebDriverResult;
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
/// # fn main() -> color_eyre::Result<()> {
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
    /// The underlying session handle.
    pub handle: Arc<SessionHandle>,
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
    /// ## Using Selenium Server
    /// - For selenium 3.x, you need to also add "/wd/hub/session" to the end of the url
    ///   (e.g. "http://localhost:4444/wd/hub/session")
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
        Self::new_with_config(server_url, capabilities, WebDriverConfig::default()).await
    }

    /// Create a new `WebDriver` with the specified `WebDriverConfig`.
    ///
    /// Use `WebDriverConfig::builder().build()` to construct the config.
    pub async fn new_with_config<S, C>(
        server_url: S,
        capabilities: C,
        config: WebDriverConfig,
    ) -> WebDriverResult<Self>
    where
        S: Into<String>,
        C: Into<Capabilities>,
    {
        // TODO: create builder
        #[cfg(feature = "reqwest")]
        let client = create_reqwest_client(config.reqwest_timeout);
        #[cfg(not(feature = "reqwest"))]
        let client = crate::session::http::null_client::create_null_client();
        Self::new_with_config_and_client(server_url, capabilities, config, client).await
    }

    /// Create a new `WebDriver` with the specified `WebDriverConfig`.
    ///
    /// Use `WebDriverConfig::builder().build()` to construct the config.
    pub async fn new_with_config_and_client<S, C>(
        server_url: S,
        capabilities: C,
        config: WebDriverConfig,
        client: impl HttpClient,
    ) -> WebDriverResult<Self>
    where
        S: Into<String>,
        C: Into<Capabilities>,
    {
        let capabilities = capabilities.into();
        let server_url = server_url
            .into()
            .parse()
            .map_err(|e| WebDriverError::ParseError(format!("invalid url: {e}")))?;

        let client = Arc::new(client);
        let session_id = start_session(client.as_ref(), &server_url, &config, capabilities).await?;

        let handle = SessionHandle::new_with_config(client, server_url, session_id, config)?;
        Ok(Self {
            handle: Arc::new(handle),
        })
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
    /// Use [`WebDriverManager::builder`] explicitly when you need a single
    /// manager to drive multiple browsers in one process.
    ///
    /// ## Process sharing across calls
    ///
    /// When called with no chained customization, multiple `WebDriver::managed`
    /// calls in the same process share a single underlying driver subprocess
    /// (refcount-keyed on `(browser, resolved_version, host)`). The shared
    /// driver lives only as long as at least one connected `WebDriver` exists.
    ///
    /// **Any chained customization** — `.latest()`, `.cache_dir(...)`,
    /// `.host(...)`, etc. — opts out of sharing and builds a *fresh* manager
    /// for that call. If you want to share one customized manager across many
    /// sessions, construct it explicitly via [`WebDriverManager::builder`] and
    /// call `.launch(caps)` on it for each session.
    ///
    /// [`WebDriverManagerBuilder`]: crate::manager::WebDriverManagerBuilder
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
