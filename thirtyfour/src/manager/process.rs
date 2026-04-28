use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{debug, warn};

use super::browser::BrowserKind;
use super::error::ManagerError;

/// How driver-process stdout/stderr is handled.
#[derive(Debug, Clone, Copy, Default)]
pub enum StdioMode {
    /// Pipe both streams and emit each line via `tracing::debug!` under the
    /// `thirtyfour::manager::driver` target. This is the default.
    #[default]
    Tracing,
    /// Inherit the parent process's stdio.
    Inherit,
    /// Discard both streams.
    Null,
}

impl StdioMode {
    fn to_stdio(self) -> Stdio {
        match self {
            StdioMode::Tracing => Stdio::piped(),
            StdioMode::Inherit => Stdio::inherit(),
            StdioMode::Null => Stdio::null(),
        }
    }
}

pub(crate) struct SpawnConfig {
    pub host: IpAddr,
    pub ready_timeout: Duration,
    pub stdio: StdioMode,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            ready_timeout: Duration::from_secs(30),
            stdio: StdioMode::default(),
        }
    }
}

/// A spawned driver process. Killed on drop.
pub(crate) struct ManagedDriverProcess {
    pub host: IpAddr,
    pub port: u16,
    pub browser: BrowserKind,
    /// `None` after `Drop` has run.
    child: Option<Child>,
    /// Set when shutdown is initiated, so the stdio pump tasks can exit cleanly.
    shutdown: Arc<AtomicBool>,
}

impl std::fmt::Debug for ManagedDriverProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedDriverProcess")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("browser", &self.browser)
            .finish()
    }
}

impl ManagedDriverProcess {
    /// Spawn the driver and poll until it reports ready.
    ///
    /// Retries are only attempted on the narrow case of a port-collision after
    /// the bind/release dance. Spawn failures (binary not found, exit on
    /// startup) and readiness timeouts surface immediately — retrying those
    /// would just multiply the wait by 3x for no benefit.
    pub(crate) async fn spawn(
        binary: &Path,
        browser: BrowserKind,
        cfg: &SpawnConfig,
    ) -> Result<Self, ManagerError> {
        const MAX_PORT_ATTEMPTS: u8 = 3;
        let mut last_err: Option<ManagerError> = None;
        for attempt in 0..MAX_PORT_ATTEMPTS {
            let port = pick_port(cfg.host)?;
            match spawn_at_port(binary, browser, cfg, port).await {
                Ok(p) => return Ok(p),
                Err(e) if is_port_in_use(&e) => {
                    debug!("driver port {port} already in use (attempt {attempt}): {e}");
                    last_err = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| ManagerError::Spawn("port allocation exhausted".into())))
    }

    /// Connection URL.
    pub(crate) fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// Heuristic: did this error come from the OS reporting that the chosen port
/// was already bound? Both "Address already in use" and Windows's "Only one
/// usage of each socket address" land here.
fn is_port_in_use(err: &ManagerError) -> bool {
    let msg = match err {
        ManagerError::Spawn(s) => s.as_str(),
        _ => return false,
    };
    let lower = msg.to_ascii_lowercase();
    lower.contains("address already in use")
        || lower.contains("only one usage of each socket address")
        || lower.contains("addrinuse")
}

async fn spawn_at_port(
    binary: &Path,
    browser: BrowserKind,
    cfg: &SpawnConfig,
    port: u16,
) -> Result<ManagedDriverProcess, ManagerError> {
    // chromedriver, geckodriver, msedgedriver, and safaridriver all accept --port=N.
    let mut cmd = Command::new(binary);
    cmd.arg(format!("--port={port}"));
    cmd.stdout(cfg.stdio.to_stdio());
    cmd.stderr(cfg.stdio.to_stdio());
    cmd.stdin(Stdio::null());
    cmd.kill_on_drop(true);

    // chromedriver / msedgedriver bind only to loopback by default; pass
    // --allowed-ips when the user has configured a non-loopback host.
    if matches!(browser, BrowserKind::Chrome | BrowserKind::Edge)
        && cfg.host != IpAddr::V4(Ipv4Addr::LOCALHOST)
    {
        cmd.arg(format!("--allowed-ips={}", cfg.host));
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| ManagerError::Spawn(format!("spawn {}: {}", binary.display(), e)))?;

    let shutdown = Arc::new(AtomicBool::new(false));
    if matches!(cfg.stdio, StdioMode::Tracing) {
        if let Some(stdout) = child.stdout.take() {
            spawn_pump("stdout", stdout, Arc::clone(&shutdown));
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_pump("stderr", stderr, Arc::clone(&shutdown));
        }
    }

    if let Err(e) = wait_until_ready(cfg.host, port, cfg.ready_timeout).await {
        let _ = child.kill().await;
        return Err(e);
    }

    Ok(ManagedDriverProcess {
        host: cfg.host,
        port,
        browser,
        child: Some(child),
        shutdown,
    })
}

fn pick_port(host: IpAddr) -> Result<u16, ManagerError> {
    let listener = TcpListener::bind(SocketAddr::new(host, 0))
        .map_err(|e| ManagerError::Spawn(format!("bind ephemeral port: {e}")))?;
    let port =
        listener.local_addr().map_err(|e| ManagerError::Spawn(format!("local_addr: {e}")))?.port();
    drop(listener);
    Ok(port)
}

fn spawn_pump<R>(stream: &'static str, reader: R, shutdown: Arc<AtomicBool>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            match lines.next_line().await {
                Ok(Some(line)) => {
                    debug!(target: "thirtyfour::manager::driver", stream, line = %line)
                }
                Ok(None) => break,
                Err(e) => {
                    warn!(target: "thirtyfour::manager::driver", stream, error = %e);
                    break;
                }
            }
        }
    });
}

async fn wait_until_ready(host: IpAddr, port: u16, timeout: Duration) -> Result<(), ManagerError> {
    let url = format!("http://{host}:{port}/status");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| ManagerError::Spawn(e.to_string()))?;

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    if body
                        .get("value")
                        .and_then(|v| v.get("ready"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        return Ok(());
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(ManagerError::DriverNotReady(timeout))
}

impl Drop for ManagedDriverProcess {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        // Sync drop, so we can't await. `start_kill` issues SIGKILL (or
        // TerminateProcess on Windows) without blocking; `kill_on_drop(true)`
        // set on the Command ensures the child is reaped asynchronously when
        // the Child handle drops.
        if let Some(mut child) = self.child.take() {
            if let Err(e) = child.start_kill() {
                warn!(target: "thirtyfour::manager", error = %e, "failed to kill driver");
            }
        }
    }
}
