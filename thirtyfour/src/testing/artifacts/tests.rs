use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use http::{Request, Response};
use serde_json::json;

use super::*;
use crate::common::config::WebDriverConfig;
use crate::error::{WebDriverError, WebDriverResult};
use crate::session::handle::SessionHandle;
use crate::session::http::{Body, HttpClient};
use crate::{Capabilities, SessionId};

#[derive(Debug, Default)]
struct ProtocolState {
    requests: Mutex<Vec<(http::Method, String)>>,
    calls: AtomicUsize,
}

#[derive(Debug, Clone)]
struct ProtocolClient(Arc<ProtocolState>);

#[async_trait::async_trait]
impl HttpClient for ProtocolClient {
    async fn send(&self, request: Request<Body<'_>>) -> WebDriverResult<Response<Bytes>> {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        self.0.requests.lock().unwrap().push((method, path.clone()));
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        if path.ends_with("/url") {
            return Err(WebDriverError::RequestFailed("url unavailable".into()));
        }
        let value = if path.ends_with("/title") {
            json!({"value": "Example title"})
        } else if path.ends_with("/screenshot") {
            json!({"value": "AQID"})
        } else if path.ends_with("/source") {
            json!({"value": "<html>αβγ failure tail</html>"})
        } else if path.ends_with("/log") {
            json!({"value": [{
                "level": "SEVERE",
                "message": "console failure",
                "timestamp": 42
            }]})
        } else {
            panic!("unexpected request path: {path}")
        };
        Ok(Response::builder()
            .status(200)
            .body(Bytes::from(serde_json::to_vec(&value).unwrap()))
            .unwrap())
    }

    async fn new(&self) -> Arc<dyn HttpClient> {
        Arc::new(self.clone())
    }
}

fn protocol_driver() -> (WebDriver, Arc<ProtocolState>) {
    let state = Arc::new(ProtocolState::default());
    let client: Arc<dyn HttpClient> = Arc::new(ProtocolClient(Arc::clone(&state)));
    let handle = SessionHandle::new_with_config_guard_and_caps(
        client,
        "http://localhost:4444",
        SessionId::from("artifact-session"),
        WebDriverConfig::default(),
        None,
        Capabilities::new(),
    )
    .unwrap();
    (
        WebDriver {
            handle: Arc::new(handle),
        },
        state,
    )
}

#[test]
fn source_excerpt_is_utf8_safe_and_never_exceeds_limit() {
    let source = "start αβγ middle δεζ end".to_string();
    for limit in 0..=source.len() {
        let excerpt = SourceExcerpt::new(source.clone(), limit);
        assert!(excerpt.text.len() <= limit);
        assert!(excerpt.text.is_char_boundary(excerpt.text.len()));
        assert_eq!(excerpt.original_bytes, source.len());
        assert_eq!(excerpt.truncated, source.len() > limit);
    }
    let excerpt = SourceExcerpt::new(source.clone(), 30);
    assert!(excerpt.text.starts_with("start"));
    assert!(excerpt.text.ends_with("end"));
}

#[test]
fn driver_ring_retains_newest_entries_with_hard_text_bounds() {
    let limits = LogLimits {
        max_entries: 2,
        max_entry_text_bytes: 4,
        max_total_text_bytes: 8,
    };
    let mut buffer = DriverProcessLogBuffer::new(Some(limits));
    for line in ["first", "αβγ", "third"] {
        buffer.push(DriverProcessLogArtifact {
            stream: "stderr".into(),
            line: line.into(),
            truncated: false,
        });
    }
    let lines = buffer.entries.iter().map(|entry| entry.line.as_str()).collect::<Vec<_>>();
    assert_eq!(lines, ["αβ", "thir"]);
    assert!(buffer.total_bytes <= limits.max_total_text_bytes);
    assert!(buffer.entries.iter().all(|entry| entry.line.len() <= 4));
}

#[test]
fn browser_logs_retain_newest_entries_and_bound_total_text() {
    let entries = (0..5)
        .map(|i| BrowserLogEntry {
            level: "INFO".into(),
            message: format!("message-{i}"),
            timestamp: i,
            source: None,
        })
        .collect();
    let bounded = bound_browser_logs(
        entries,
        LogLimits {
            max_entries: 2,
            max_entry_text_bytes: 6,
            max_total_text_bytes: 12,
        },
    );
    assert_eq!(bounded.len(), 2);
    assert_eq!(bounded[0].timestamp, 3);
    assert_eq!(bounded[1].timestamp, 4);
    assert!(browser_log_text_bytes(&bounded) <= 12);
    assert!(bounded.iter().all(|entry| browser_log_entry_text_bytes(entry) <= 6));
}

#[test]
fn browser_logs_bound_adversarial_level_source_and_message_text() {
    let entries = (0..4)
        .map(|i| BrowserLogEntry {
            level: "L".repeat(2_000),
            message: "M".repeat(2_000),
            timestamp: i,
            source: Some("S".repeat(2_000)),
        })
        .collect();
    let limits = LogLimits {
        max_entries: 3,
        max_entry_text_bytes: 31,
        max_total_text_bytes: 62,
    };

    let bounded = bound_browser_logs(entries, limits);

    assert_eq!(bounded.len(), 2);
    assert!(bounded.iter().all(|entry| entry.truncated));
    assert!(
        bounded
            .iter()
            .all(|entry| browser_log_entry_text_bytes(entry) <= limits.max_entry_text_bytes)
    );
    assert!(browser_log_text_bytes(&bounded) <= limits.max_total_text_bytes);
    assert!(bounded.iter().all(|entry| entry.level.len() <= 3));
    assert!(bounded.iter().all(|entry| entry.source.as_ref().unwrap().len() <= 3));

    let artifacts = FailureArtifacts {
        url: Artifact::Disabled,
        title: Artifact::Disabled,
        screenshot: Artifact::Disabled,
        source: Artifact::Disabled,
        browser_logs: Artifact::Captured(bounded),
        driver_process_logs: Artifact::Disabled,
    };
    assert!(artifacts.to_string().contains("[truncated]"));
}

fn browser_log_entry_text_bytes(entry: &BrowserLogArtifact) -> usize {
    entry.level.len() + entry.message.len() + entry.source.as_deref().map_or(0, str::len)
}

fn browser_log_text_bytes(entries: &[BrowserLogArtifact]) -> usize {
    entries.iter().map(browser_log_entry_text_bytes).sum()
}

#[tokio::test]
async fn capture_attempts_every_endpoint_after_an_independent_failure() {
    let (driver, state) = protocol_driver();
    let config = FailureArtifactConfig {
        source_max_bytes: Some(20),
        driver_process_logs: None,
        ..FailureArtifactConfig::default()
    };
    let collector = FailureArtifactCollector::with_config(&driver, config);

    let artifacts = collector.capture().await;

    assert!(matches!(artifacts.url, Artifact::Unavailable(_)));
    assert!(matches!(artifacts.title, Artifact::Captured(ref value) if value == "Example title"));
    assert!(matches!(artifacts.screenshot, Artifact::Captured(ref value) if value == &[1, 2, 3]));
    assert!(matches!(artifacts.source, Artifact::Captured(ref value) if value.text.len() <= 20));
    assert!(matches!(artifacts.browser_logs, Artifact::Captured(ref value) if value.len() == 1));
    assert!(matches!(artifacts.driver_process_logs, Artifact::Disabled));
    assert_eq!(
        state.requests.lock().unwrap().as_slice(),
        &[
            (http::Method::GET, "/session/artifact-session/url".into()),
            (http::Method::GET, "/session/artifact-session/title".into()),
            (http::Method::GET, "/session/artifact-session/screenshot".into()),
            (http::Method::GET, "/session/artifact-session/source".into()),
            (http::Method::POST, "/session/artifact-session/log".into()),
        ]
    );
    driver.leak().unwrap();
}

#[tokio::test]
async fn disabled_artifacts_issue_no_webdriver_requests() {
    let (driver, state) = protocol_driver();
    let config = FailureArtifactConfig {
        capture_url: false,
        capture_title: false,
        capture_screenshot: false,
        source_max_bytes: None,
        browser_logs: None,
        driver_process_logs: None,
    };
    let artifacts = FailureArtifactCollector::with_config(&driver, config).capture().await;
    assert!(matches!(artifacts.url, Artifact::Disabled));
    assert!(matches!(artifacts.title, Artifact::Disabled));
    assert!(matches!(artifacts.screenshot, Artifact::Disabled));
    assert!(matches!(artifacts.source, Artifact::Disabled));
    assert!(matches!(artifacts.browser_logs, Artifact::Disabled));
    assert_eq!(state.calls.load(Ordering::SeqCst), 0);
    driver.leak().unwrap();
}

#[test]
fn display_reports_screenshot_size_without_exposing_bytes_or_base64() {
    let artifacts = FailureArtifacts {
        url: Artifact::Disabled,
        title: Artifact::Disabled,
        screenshot: Artifact::Captured(vec![222, 173, 190, 239]),
        source: Artifact::Disabled,
        browser_logs: Artifact::Disabled,
        driver_process_logs: Artifact::Disabled,
    };
    let output = artifacts.to_string();
    assert!(output.contains("4 PNG bytes"));
    assert!(!output.contains("222"));
    assert!(!output.contains("3q2+7w"));
}

#[test]
fn display_bounds_url_and_title_fields() {
    let oversized = "x".repeat(DEFAULT_DISPLAY_FIELD_BYTES * 2);
    let artifacts = FailureArtifacts {
        url: Artifact::Captured(oversized.clone()),
        title: Artifact::Captured(oversized),
        screenshot: Artifact::Disabled,
        source: Artifact::Disabled,
        browser_logs: Artifact::Disabled,
        driver_process_logs: Artifact::Disabled,
    };

    let output = artifacts.to_string();
    assert!(output.contains(OMITTED.trim()));
    assert!(output.len() < DEFAULT_DISPLAY_FIELD_BYTES * 2 + 256);
}
