use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::Capabilities;
use crate::common::capabilities::desiredcapabilities::CapabilitiesHelper;

use super::error::ManagerError;

/// Browsers the manager can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrowserKind {
    /// Chrome / Chromium.
    Chrome,
    /// Firefox.
    Firefox,
    /// Microsoft Edge (Chromium-based).
    Edge,
    /// Apple Safari. macOS-only; uses the system `safaridriver` and does not
    /// download anything. Requires `safaridriver --enable` to be run once.
    Safari,
}

impl BrowserKind {
    /// Driver binary name (without `.exe`).
    pub(crate) fn driver_binary_stem(self) -> &'static str {
        match self {
            BrowserKind::Chrome => "chromedriver",
            BrowserKind::Firefox => "geckodriver",
            BrowserKind::Edge => "msedgedriver",
            BrowserKind::Safari => "safaridriver",
        }
    }

    /// Display name used in error hints.
    pub(crate) fn display_name(self) -> &'static str {
        match self {
            BrowserKind::Chrome => "Chrome",
            BrowserKind::Firefox => "Firefox",
            BrowserKind::Edge => "Microsoft Edge",
            BrowserKind::Safari => "Safari",
        }
    }

    /// Cache subdirectory name.
    pub(crate) fn cache_dir_name(self) -> &'static str {
        match self {
            BrowserKind::Chrome => "chromedriver",
            BrowserKind::Firefox => "geckodriver",
            BrowserKind::Edge => "msedgedriver",
            BrowserKind::Safari => "safaridriver",
        }
    }

    /// `true` if the driver is system-managed (no download, no cache).
    pub(crate) fn is_system_managed(self) -> bool {
        matches!(self, BrowserKind::Safari)
    }

    /// Derive `BrowserKind` from a W3C capabilities object's `browserName`.
    pub fn from_capabilities(caps: &Capabilities) -> Result<Self, ManagerError> {
        let name = caps
            ._get("browserName")
            .and_then(Value::as_str)
            .ok_or(ManagerError::MissingBrowserName)?;
        match name.to_ascii_lowercase().as_str() {
            "chrome" | "chromium" => Ok(BrowserKind::Chrome),
            "firefox" => Ok(BrowserKind::Firefox),
            "microsoftedge" | "msedge" | "edge" => Ok(BrowserKind::Edge),
            "safari" => Ok(BrowserKind::Safari),
            other => Err(ManagerError::UnsupportedBrowser(other.to_string())),
        }
    }

    /// Return a custom binary path declared in the capabilities, if any.
    /// Chrome users can set `goog:chromeOptions.binary`; Firefox users can set
    /// `moz:firefoxOptions.binary`; Edge users `ms:edgeOptions.binary`. Safari
    /// has no equivalent (the OS picks).
    pub(crate) fn binary_from_caps(self, caps: &Capabilities) -> Option<String> {
        let (key, sub) = match self {
            BrowserKind::Chrome => ("goog:chromeOptions", "binary"),
            BrowserKind::Firefox => ("moz:firefoxOptions", "binary"),
            BrowserKind::Edge => ("ms:edgeOptions", "binary"),
            BrowserKind::Safari => return None,
        };
        caps._get(key)?.get(sub)?.as_str().map(str::to_owned)
    }
}

/// Probe the locally-installed browser for its version. If `caps_binary` is
/// supplied (i.e. user has set a custom binary path in capabilities), use that;
/// otherwise probe a list of well-known install locations.
pub(crate) fn detect_local_version(
    browser: BrowserKind,
    caps_binary: Option<&str>,
) -> Result<String, ManagerError> {
    // Safari has no real version-resolution: there's only ever one
    // `safaridriver` on the system, matched to the installed Safari.
    if browser.is_system_managed() {
        return Ok("system".to_string());
    }

    if let Some(path) = caps_binary {
        if let Some(version) = run_version(path, browser) {
            return Ok(version);
        }
        return Err(ManagerError::LocalBrowserNotFound {
            browser: browser.display_name(),
            hint: "the binary path in capabilities did not respond to --version; \
                   try DriverVersion::Latest or DriverVersion::Exact(...)",
        });
    }

    for candidate in candidate_paths(browser) {
        if let Some(v) = run_version(&candidate, browser) {
            return Ok(v);
        }
    }

    Err(ManagerError::LocalBrowserNotFound {
        browser: browser.display_name(),
        hint: "no installed copy was found; try DriverVersion::Latest or DriverVersion::Exact(...)",
    })
}

fn candidate_paths(browser: BrowserKind) -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        match browser {
            BrowserKind::Chrome => vec![
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".to_string(),
                "/Applications/Chromium.app/Contents/MacOS/Chromium".to_string(),
                "google-chrome".to_string(),
                "chromium".to_string(),
            ],
            BrowserKind::Firefox => vec![
                "/Applications/Firefox.app/Contents/MacOS/firefox".to_string(),
                "firefox".to_string(),
            ],
            BrowserKind::Edge => vec![
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".to_string(),
            ],
            BrowserKind::Safari => Vec::new(),
        }
    }
    #[cfg(target_os = "linux")]
    {
        match browser {
            BrowserKind::Chrome => vec![
                "google-chrome".to_string(),
                "google-chrome-stable".to_string(),
                "chromium".to_string(),
                "chromium-browser".to_string(),
            ],
            BrowserKind::Firefox => vec!["firefox".to_string(), "firefox-esr".to_string()],
            BrowserKind::Edge => vec![
                "microsoft-edge".to_string(),
                "microsoft-edge-stable".to_string(),
            ],
            BrowserKind::Safari => Vec::new(),
        }
    }
    #[cfg(target_os = "windows")]
    {
        match browser {
            BrowserKind::Chrome => vec![
                r"C:\Program Files\Google\Chrome\Application\chrome.exe".to_string(),
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe".to_string(),
                "chrome.exe".to_string(),
            ],
            BrowserKind::Firefox => vec![
                r"C:\Program Files\Mozilla Firefox\firefox.exe".to_string(),
                r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe".to_string(),
                "firefox.exe".to_string(),
            ],
            BrowserKind::Edge => vec![
                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe".to_string(),
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".to_string(),
                "msedge.exe".to_string(),
            ],
            BrowserKind::Safari => Vec::new(),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = browser;
        Vec::new()
    }
}

fn run_version(path: &str, browser: BrowserKind) -> Option<String> {
    if !exists_or_in_path(path) {
        return None;
    }

    // On Windows, Firefox doesn't flush its --version output cleanly without `| more`.
    #[cfg(target_os = "windows")]
    let output = if matches!(browser, BrowserKind::Firefox) {
        let cmd = format!("\"{path}\" --version | more");
        Command::new("cmd").args(["/C", &cmd]).output().ok()?
    } else {
        Command::new(path).arg("--version").output().ok()?
    };

    #[cfg(not(target_os = "windows"))]
    let output = {
        let _ = browser;
        Command::new(path).arg("--version").output().ok()?
    };

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version(&stdout)
}

fn exists_or_in_path(path: &str) -> bool {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.exists();
    }
    // Bare command: probe via the OS PATH by attempting --version with `where`/`which`.
    // Cheaper: just try to spawn it; the caller will get `None` on failure.
    true
}

/// Extract a version like `126.0.6478.126` (or `126.0`) from arbitrary `--version` output.
pub(crate) fn parse_version(s: &str) -> Option<String> {
    let mut it = s.chars().peekable();
    while it.peek().is_some() {
        if !it.peek().map_or(false, |c| c.is_ascii_digit()) {
            it.next();
            continue;
        }
        let mut buf = String::new();
        while let Some(&c) = it.peek() {
            if c.is_ascii_digit() || c == '.' {
                buf.push(c);
                it.next();
            } else {
                break;
            }
        }
        // Require at least one dot to look like a real version (not "30" from "Mac OS X 10.15...").
        if buf.contains('.') && buf.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            // Strip a trailing dot if any.
            let trimmed = buf.trim_end_matches('.');
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Major version (the segment before the first dot).
pub(crate) fn major(version: &str) -> &str {
    version.split('.').next().unwrap_or(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_chrome_version() {
        assert_eq!(
            parse_version("Google Chrome 126.0.6478.126 \n").as_deref(),
            Some("126.0.6478.126")
        );
    }

    #[test]
    fn parse_firefox_version() {
        assert_eq!(
            parse_version("Mozilla Firefox 128.0.2").as_deref(),
            Some("128.0.2")
        );
    }

    #[test]
    fn major_strips() {
        assert_eq!(major("126.0.6478.126"), "126");
        assert_eq!(major("128"), "128");
    }

    #[test]
    fn browser_kind_chrome() {
        let mut caps = Capabilities::new();
        caps.insert("browserName".into(), json!("chrome"));
        assert_eq!(BrowserKind::from_capabilities(&caps).unwrap(), BrowserKind::Chrome);
    }

    #[test]
    fn browser_kind_chromium() {
        let mut caps = Capabilities::new();
        caps.insert("browserName".into(), json!("chromium"));
        assert_eq!(BrowserKind::from_capabilities(&caps).unwrap(), BrowserKind::Chrome);
    }

    #[test]
    fn browser_kind_firefox() {
        let mut caps = Capabilities::new();
        caps.insert("browserName".into(), json!("firefox"));
        assert_eq!(BrowserKind::from_capabilities(&caps).unwrap(), BrowserKind::Firefox);
    }

    #[test]
    fn browser_kind_edge() {
        for name in ["microsoftedge", "MicrosoftEdge", "edge", "msedge"] {
            let mut caps = Capabilities::new();
            caps.insert("browserName".into(), json!(name));
            assert_eq!(
                BrowserKind::from_capabilities(&caps).unwrap(),
                BrowserKind::Edge,
                "browserName={name}"
            );
        }
    }

    #[test]
    fn browser_kind_safari() {
        let mut caps = Capabilities::new();
        caps.insert("browserName".into(), json!("safari"));
        assert_eq!(BrowserKind::from_capabilities(&caps).unwrap(), BrowserKind::Safari);
    }

    #[test]
    fn browser_kind_unsupported() {
        let mut caps = Capabilities::new();
        caps.insert("browserName".into(), json!("ie"));
        assert!(matches!(
            BrowserKind::from_capabilities(&caps),
            Err(ManagerError::UnsupportedBrowser(_))
        ));
    }

    #[test]
    fn safari_is_system_managed() {
        assert!(BrowserKind::Safari.is_system_managed());
        assert!(!BrowserKind::Chrome.is_system_managed());
        assert!(!BrowserKind::Firefox.is_system_managed());
        assert!(!BrowserKind::Edge.is_system_managed());
    }

    #[test]
    fn binary_from_caps_edge() {
        let mut caps = Capabilities::new();
        caps.insert("ms:edgeOptions".into(), json!({"binary": "/path/to/msedge"}));
        assert_eq!(
            BrowserKind::Edge.binary_from_caps(&caps).as_deref(),
            Some("/path/to/msedge")
        );
    }

    #[test]
    fn browser_kind_missing() {
        let caps = Capabilities::new();
        assert!(matches!(
            BrowserKind::from_capabilities(&caps),
            Err(ManagerError::MissingBrowserName)
        ));
    }

    #[test]
    fn binary_from_caps_chrome() {
        let mut caps = Capabilities::new();
        caps.insert(
            "goog:chromeOptions".into(),
            json!({"binary": "/path/to/chrome"}),
        );
        assert_eq!(
            BrowserKind::Chrome.binary_from_caps(&caps).as_deref(),
            Some("/path/to/chrome")
        );
    }

    #[test]
    fn binary_from_caps_firefox() {
        let mut caps = Capabilities::new();
        caps.insert(
            "moz:firefoxOptions".into(),
            json!({"binary": "/path/to/firefox"}),
        );
        assert_eq!(
            BrowserKind::Firefox.binary_from_caps(&caps).as_deref(),
            Some("/path/to/firefox")
        );
    }
}
