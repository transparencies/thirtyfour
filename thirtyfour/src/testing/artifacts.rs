use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};

use crate::{BrowserLogEntry, WebDriver};

const DEFAULT_SOURCE_BYTES: usize = 64 * 1024;
const DEFAULT_LOG_ENTRIES: usize = 200;
const DEFAULT_LOG_ENTRY_BYTES: usize = 4 * 1024;
const DEFAULT_LOG_TOTAL_BYTES: usize = 64 * 1024;
const DEFAULT_DISPLAY_FIELD_BYTES: usize = 8 * 1024;
const OMITTED: &str = "\n… omitted …\n";

/// The result of capturing one failure artifact.
#[derive(Debug)]
#[non_exhaustive]
pub enum Artifact<T> {
    /// The artifact was captured successfully.
    Captured(T),
    /// Capture was attempted but the browser, feature set, or session could not provide it.
    Unavailable(ArtifactUnavailable),
    /// Capture was explicitly disabled in [`FailureArtifactConfig`].
    Disabled,
}

/// A non-fatal failure to capture one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ArtifactUnavailable {
    /// Human-readable error suitable for CI output.
    pub message: String,
}

impl ArtifactUnavailable {
    fn new(error: impl Display) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

/// Hard limits applied to browser and managed-driver-process log capture.
///
/// The default retains the newest 200 entries, at most 4 KiB of driver-provided
/// text per entry, and at most 64 KiB of driver-provided text in total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LogLimits {
    /// Maximum number of entries retained. The newest entries win.
    pub max_entries: usize,
    /// Maximum UTF-8 bytes retained from all driver-provided text in one entry.
    pub max_entry_text_bytes: usize,
    /// Maximum UTF-8 bytes retained from driver-provided text across all entries.
    pub max_total_text_bytes: usize,
}

impl Default for LogLimits {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_LOG_ENTRIES,
            max_entry_text_bytes: DEFAULT_LOG_ENTRY_BYTES,
            max_total_text_bytes: DEFAULT_LOG_TOTAL_BYTES,
        }
    }
}

/// Configuration for [`FailureArtifactCollector`].
///
/// Page-source and log text have hard bounds. `None` disables that artifact.
/// By default the source excerpt is capped at 64 KiB and both log streams use
/// [`LogLimits::default`]. URL and title values remain available in full on
/// [`FailureArtifacts`], while its [`Display`] report bounds each of those
/// fields to 8 KiB.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FailureArtifactConfig {
    /// Capture the current URL.
    pub capture_url: bool,
    /// Capture the page title.
    pub capture_title: bool,
    /// Capture viewport PNG bytes.
    pub capture_screenshot: bool,
    /// Maximum page-source excerpt bytes, or `None` to disable source capture.
    pub source_max_bytes: Option<usize>,
    /// Browser-log bounds, or `None` to disable browser logs.
    pub browser_logs: Option<LogLimits>,
    /// Managed-driver-process-log bounds, or `None` to disable process logs.
    pub driver_process_logs: Option<LogLimits>,
}

impl Default for FailureArtifactConfig {
    fn default() -> Self {
        Self {
            capture_url: true,
            capture_title: true,
            capture_screenshot: true,
            source_max_bytes: Some(DEFAULT_SOURCE_BYTES),
            browser_logs: Some(LogLimits::default()),
            driver_process_logs: Some(LogLimits::default()),
        }
    }
}

/// A UTF-8-safe, hard-bounded page-source excerpt.
///
/// Truncated excerpts preserve both ends. The omission marker counts toward
/// the configured byte limit; very small limits retain only a safe prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceExcerpt {
    /// Bounded source text. When truncated, both the beginning and end are retained.
    pub text: String,
    /// Size of the original source in UTF-8 bytes.
    pub original_bytes: usize,
    /// Whether any source bytes were omitted.
    pub truncated: bool,
}

impl SourceExcerpt {
    fn new(source: String, max_bytes: usize) -> Self {
        let original_bytes = source.len();
        let truncated = original_bytes > max_bytes;
        let text = if truncated {
            truncate_middle(&source, max_bytes)
        } else {
            source
        };
        Self {
            text,
            original_bytes,
            truncated,
        }
    }
}

/// One bounded browser-log entry.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BrowserLogArtifact {
    /// Driver-provided severity.
    pub level: String,
    /// Bounded message text.
    pub message: String,
    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp: i64,
    /// Optional driver-provided source.
    pub source: Option<String>,
    /// Whether the level, message, or source was truncated.
    pub truncated: bool,
}

/// One bounded stdout/stderr line from a managed driver process.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DriverProcessLogArtifact {
    /// Library-provided stream label (`stdout` or `stderr`).
    pub stream: String,
    /// Bounded line text.
    pub line: String,
    /// Whether the line was truncated.
    pub truncated: bool,
}

/// Independently captured browser failure context.
#[derive(Debug)]
#[non_exhaustive]
pub struct FailureArtifacts {
    /// Current page URL.
    pub url: Artifact<String>,
    /// Current page title.
    pub title: Artifact<String>,
    /// Viewport screenshot as PNG bytes.
    pub screenshot: Artifact<Vec<u8>>,
    /// Bounded page-source excerpt.
    pub source: Artifact<SourceExcerpt>,
    /// Bounded browser-console log entries.
    pub browser_logs: Artifact<Vec<BrowserLogArtifact>>,
    /// Bounded managed-driver-process stdout/stderr lines.
    pub driver_process_logs: Artifact<Vec<DriverProcessLogArtifact>>,
}

/// Collects failure context and records live managed-driver-process logs.
///
/// Construct this before the test body: managed-driver subscriptions are live
/// only and cannot recover lines emitted before attachment. The manager owns a
/// driver *process*, which it may reuse for multiple sessions, so these lines
/// are process-scoped and can include activity from another concurrent session
/// using that process. Browser logs also require the appropriate capability at
/// session creation and are unsupported by some drivers (notably geckodriver).
/// Those limitations are reported as an `Unavailable` field without preventing
/// other artifacts from being captured.
/// CDP and BiDi diagnostics are deliberately not collected here: this baseline
/// report uses portable WebDriver APIs and remains available without either
/// optional protocol feature.
#[derive(Debug)]
pub struct FailureArtifactCollector {
    driver: WebDriver,
    config: FailureArtifactConfig,
    #[cfg_attr(not(feature = "manager"), allow(dead_code))]
    driver_process_log_buffer: Arc<Mutex<DriverProcessLogBuffer>>,
    #[cfg(feature = "manager")]
    driver_process_log_subscription: Option<crate::manager::DriverLogSubscription>,
}

impl FailureArtifactCollector {
    /// Attach a collector using bounded defaults.
    pub fn new(driver: &WebDriver) -> Self {
        Self::with_config(driver, FailureArtifactConfig::default())
    }

    /// Attach a collector with explicit capture settings and hard bounds.
    pub fn with_config(driver: &WebDriver, config: FailureArtifactConfig) -> Self {
        let buffer = Arc::new(Mutex::new(DriverProcessLogBuffer::new(config.driver_process_logs)));
        #[cfg(feature = "manager")]
        let driver_process_log_subscription = config.driver_process_logs.and_then(|_| {
            let target = Arc::clone(&buffer);
            driver.on_driver_log(move |entry| {
                if let Ok(mut target) = target.lock() {
                    target.push(DriverProcessLogArtifact {
                        stream: entry.stream.to_string(),
                        line: entry.line.clone(),
                        truncated: false,
                    });
                }
            })
        });

        Self {
            driver: driver.clone(),
            config,
            driver_process_log_buffer: buffer,
            #[cfg(feature = "manager")]
            driver_process_log_subscription,
        }
    }

    /// Capture every configured artifact, preserving failures independently.
    ///
    /// This method is intentionally infallible: one unsupported endpoint or a
    /// partially closed session never prevents the remaining attempts. Capture
    /// always uses the same session passed to [`Self::new`] or
    /// [`Self::with_config`].
    pub async fn capture(&self) -> FailureArtifacts {
        let driver = &self.driver;
        let url = if self.config.capture_url {
            match driver.current_url().await {
                Ok(value) => Artifact::Captured(value.to_string()),
                Err(error) => Artifact::Unavailable(ArtifactUnavailable::new(error)),
            }
        } else {
            Artifact::Disabled
        };
        let title = if self.config.capture_title {
            match driver.title().await {
                Ok(value) => Artifact::Captured(value),
                Err(error) => Artifact::Unavailable(ArtifactUnavailable::new(error)),
            }
        } else {
            Artifact::Disabled
        };
        let screenshot = if self.config.capture_screenshot {
            match driver.screenshot_as_png().await {
                Ok(value) => Artifact::Captured(value),
                Err(error) => Artifact::Unavailable(ArtifactUnavailable::new(error)),
            }
        } else {
            Artifact::Disabled
        };
        let source = match self.config.source_max_bytes {
            Some(limit) => match driver.source().await {
                Ok(value) => Artifact::Captured(SourceExcerpt::new(value, limit)),
                Err(error) => Artifact::Unavailable(ArtifactUnavailable::new(error)),
            },
            None => Artifact::Disabled,
        };
        let browser_logs = match self.config.browser_logs {
            Some(limits) => match driver.browser_log().await {
                Ok(entries) => Artifact::Captured(bound_browser_logs(entries, limits)),
                Err(error) => Artifact::Unavailable(ArtifactUnavailable::new(error)),
            },
            None => Artifact::Disabled,
        };
        let driver_process_logs = self.capture_driver_process_logs();

        FailureArtifacts {
            url,
            title,
            screenshot,
            source,
            browser_logs,
            driver_process_logs,
        }
    }

    fn capture_driver_process_logs(&self) -> Artifact<Vec<DriverProcessLogArtifact>> {
        if self.config.driver_process_logs.is_none() {
            return Artifact::Disabled;
        }
        #[cfg(feature = "manager")]
        if self.driver_process_log_subscription.is_none() {
            return Artifact::Unavailable(ArtifactUnavailable::new(
                "session was not launched by WebDriverManager",
            ));
        }
        #[cfg(not(feature = "manager"))]
        return Artifact::Unavailable(ArtifactUnavailable::new(
            "thirtyfour manager feature is disabled",
        ));

        #[cfg(feature = "manager")]
        match self.driver_process_log_buffer.lock() {
            Ok(buffer) => Artifact::Captured(buffer.entries.iter().cloned().collect()),
            Err(_) => Artifact::Unavailable(ArtifactUnavailable::new(
                "managed-driver-process log buffer was poisoned",
            )),
        }
    }
}

#[cfg_attr(not(feature = "manager"), allow(dead_code))]
#[derive(Debug)]
struct DriverProcessLogBuffer {
    limits: Option<LogLimits>,
    entries: VecDeque<DriverProcessLogArtifact>,
    total_bytes: usize,
}

#[cfg_attr(not(feature = "manager"), allow(dead_code))]
impl DriverProcessLogBuffer {
    fn new(limits: Option<LogLimits>) -> Self {
        Self {
            limits,
            entries: VecDeque::new(),
            total_bytes: 0,
        }
    }

    fn push(&mut self, mut entry: DriverProcessLogArtifact) {
        let Some(limits) = self.limits else {
            return;
        };
        let original = entry.line.len();
        entry.line = truncate_prefix(
            &entry.line,
            limits.max_entry_text_bytes.min(limits.max_total_text_bytes),
        );
        entry.truncated = entry.line.len() < original;
        while (!self.entries.is_empty() && self.entries.len() >= limits.max_entries)
            || (!self.entries.is_empty()
                && self.total_bytes + entry.line.len() > limits.max_total_text_bytes)
        {
            self.total_bytes -= self.entries.pop_front().expect("checked nonempty").line.len();
        }
        if limits.max_entries > 0
            && limits.max_total_text_bytes > 0
            && entry.line.len() <= limits.max_total_text_bytes
        {
            self.total_bytes += entry.line.len();
            self.entries.push_back(entry);
        }
    }
}

fn bound_browser_logs(entries: Vec<BrowserLogEntry>, limits: LogLimits) -> Vec<BrowserLogArtifact> {
    let mut bounded = VecDeque::new();
    let mut total = 0;
    for entry in entries.into_iter().rev() {
        if bounded.len() >= limits.max_entries || total >= limits.max_total_text_bytes {
            break;
        }
        let allowed = limits.max_entry_text_bytes.min(limits.max_total_text_bytes - total);

        // Keep a small, bounded share for metadata. The message receives the
        // remainder because it normally carries the useful failure detail.
        let level_limit = allowed / 8;
        let source_limit = if entry.source.is_some() {
            allowed / 8
        } else {
            0
        };
        let level = truncate_prefix(&entry.level, level_limit);
        let source = entry.source.as_deref().map(|value| truncate_prefix(value, source_limit));
        let message_limit =
            allowed.saturating_sub(level.len() + source.as_deref().map_or(0, str::len));
        let message = truncate_prefix(&entry.message, message_limit);
        let retained = level.len() + message.len() + source.as_deref().map_or(0, str::len);
        let artifact = BrowserLogArtifact {
            truncated: level.len() < entry.level.len()
                || message.len() < entry.message.len()
                || source.as_deref().map_or(0, str::len)
                    < entry.source.as_deref().map_or(0, str::len),
            level,
            message,
            timestamp: entry.timestamp,
            source,
        };
        total += retained;
        bounded.push_front(artifact);
    }
    bounded.into()
}

fn truncate_prefix(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

fn truncate_middle(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    if max_bytes <= OMITTED.len() {
        return truncate_prefix(text, max_bytes);
    }
    let payload = max_bytes - OMITTED.len();
    let mut head_end = payload.div_ceil(2);
    while !text.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut tail_start = text.len() - payload / 2;
    while !text.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    format!("{}{}{}", &text[..head_end], OMITTED, &text[tail_start..])
}

impl Display for FailureArtifacts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "failure artifacts:")?;
        display_simple(f, "url", &self.url)?;
        display_simple(f, "title", &self.title)?;
        match &self.screenshot {
            Artifact::Captured(bytes) => {
                writeln!(f, "screenshot: captured ({} PNG bytes)", bytes.len())?
            }
            Artifact::Unavailable(error) => display_unavailable(f, "screenshot", error)?,
            Artifact::Disabled => writeln!(f, "screenshot: disabled")?,
        }
        match &self.source {
            Artifact::Captured(source) => {
                writeln!(
                    f,
                    "source: captured ({} original bytes, truncated: {})",
                    source.original_bytes, source.truncated
                )?;
                writeln!(f, "{}", source.text)?;
            }
            Artifact::Unavailable(error) => display_unavailable(f, "source", error)?,
            Artifact::Disabled => writeln!(f, "source: disabled")?,
        }
        display_logs(f, "browser logs", &self.browser_logs, |entry| {
            format!("[{}] {}{}", entry.level, entry.message, truncation_label(entry.truncated))
        })?;
        display_logs(f, "driver process logs", &self.driver_process_logs, |entry| {
            format!("[{}] {}{}", entry.stream, entry.line, truncation_label(entry.truncated))
        })
    }
}

fn truncation_label(truncated: bool) -> &'static str {
    if truncated {
        " [truncated]"
    } else {
        ""
    }
}

fn display_simple(
    f: &mut Formatter<'_>,
    name: &str,
    artifact: &Artifact<String>,
) -> std::fmt::Result {
    match artifact {
        Artifact::Captured(value) => {
            writeln!(f, "{name}: {}", truncate_middle(value, DEFAULT_DISPLAY_FIELD_BYTES))
        }
        Artifact::Unavailable(error) => display_unavailable(f, name, error),
        Artifact::Disabled => writeln!(f, "{name}: disabled"),
    }
}

fn display_logs<T>(
    f: &mut Formatter<'_>,
    name: &str,
    artifact: &Artifact<Vec<T>>,
    render: impl Fn(&T) -> String,
) -> std::fmt::Result {
    match artifact {
        Artifact::Captured(entries) => {
            writeln!(f, "{name}: captured ({} entries)", entries.len())?;
            for entry in entries {
                writeln!(f, "  {}", render(entry))?;
            }
            Ok(())
        }
        Artifact::Unavailable(error) => display_unavailable(f, name, error),
        Artifact::Disabled => writeln!(f, "{name}: disabled"),
    }
}

fn display_unavailable(
    f: &mut Formatter<'_>,
    name: &str,
    error: &ArtifactUnavailable,
) -> std::fmt::Result {
    writeln!(
        f,
        "{name}: unavailable ({})",
        truncate_middle(&error.message, DEFAULT_DISPLAY_FIELD_BYTES)
    )
}

#[cfg(test)]
mod tests;
