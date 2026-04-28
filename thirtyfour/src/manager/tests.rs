//! Tests that exercise the manager beyond the per-module unit tests.

use std::time::Duration;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr};

use super::browser::BrowserKind;
use super::download::{DownloadConfig, Mirror, resolve_version};
use super::manager::DriverKey;
use super::version::DriverVersion;
use super::{StdioMode, WebDriverManager};
use crate::Capabilities;

#[test]
fn builder_defaults_match_match_local() {
    let b = WebDriverManager::builder();
    let mgr = b.build();
    assert_eq!(mgr.cfg.version, DriverVersion::MatchLocalBrowser);
    assert_eq!(mgr.cfg.host.to_string(), "127.0.0.1");
    assert!(!mgr.cfg.offline);
}

#[test]
fn builder_chain_latest() {
    let mgr = WebDriverManager::builder().latest().build();
    assert_eq!(mgr.cfg.version, DriverVersion::Latest);
}

#[test]
fn builder_chain_exact() {
    let mgr = WebDriverManager::builder().exact("126").build();
    assert_eq!(mgr.cfg.version, DriverVersion::Exact("126".to_string()));
}

#[test]
fn builder_chain_from_caps() {
    let mgr = WebDriverManager::builder().from_caps().build();
    assert_eq!(mgr.cfg.version, DriverVersion::FromCapabilities);
}

#[test]
fn builder_offline_toggle() {
    let mgr = WebDriverManager::builder().offline().build();
    assert!(mgr.cfg.offline);
    let mgr = WebDriverManager::builder().offline().online().build();
    assert!(!mgr.cfg.offline);
}

#[test]
fn builder_stdio() {
    let mgr = WebDriverManager::builder().stdio(StdioMode::Null).build();
    assert!(matches!(mgr.cfg.stdio, StdioMode::Null));
}

#[test]
fn shared_returns_same_arc() {
    let a = WebDriverManager::shared();
    let b = WebDriverManager::shared();
    assert!(std::sync::Arc::ptr_eq(&a, &b));
}

fn hash_of<T: Hash>(t: &T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

#[test]
fn driver_key_equal_when_all_fields_match() {
    let a = DriverKey {
        browser: BrowserKind::Chrome,
        version: "126.0.6478.126".to_string(),
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let b = a.clone();
    assert_eq!(a, b);
    assert_eq!(hash_of(&a), hash_of(&b));
}

#[test]
fn driver_key_differs_when_browser_differs() {
    let chrome = DriverKey {
        browser: BrowserKind::Chrome,
        version: "1".to_string(),
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let firefox = DriverKey {
        browser: BrowserKind::Firefox,
        version: "1".to_string(),
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    assert_ne!(chrome, firefox);
}

#[test]
fn driver_key_differs_when_version_differs() {
    let v1 = DriverKey {
        browser: BrowserKind::Chrome,
        version: "126".to_string(),
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let v2 = DriverKey {
        browser: BrowserKind::Chrome,
        version: "127".to_string(),
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    assert_ne!(v1, v2);
}

#[test]
fn driver_key_differs_when_host_differs() {
    let loopback = DriverKey {
        browser: BrowserKind::Chrome,
        version: "1".to_string(),
        host: IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let any = DriverKey {
        browser: BrowserKind::Chrome,
        version: "1".to_string(),
        host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    };
    assert_ne!(loopback, any);
}

#[test]
fn webdriver_managed_returns_builder_with_caps() {
    let mut caps = Capabilities::new();
    caps.insert("browserName".into(), json!("chrome"));
    let builder = crate::WebDriver::managed(caps);
    assert!(builder.preloaded_caps.is_some());
    assert_eq!(builder.version, DriverVersion::MatchLocalBrowser);
}

#[tokio::test]
async fn resolve_version_latest_chrome() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/chrome-for-testing/LATEST_RELEASE_STABLE"))
        .respond_with(ResponseTemplate::new(200).set_body_string("126.0.6478.126"))
        .mount(&server)
        .await;

    let cfg = DownloadConfig {
        cache_dir: std::env::temp_dir(),
        mirror: Mirror {
            chrome_metadata: format!("{}/", server.uri()).parse().unwrap(),
            ..Mirror::default()
        },
        download_timeout: Duration::from_secs(5),
        offline: false,
    };

    let client = reqwest::Client::new();
    let v = resolve_version(
        &client,
        &cfg,
        BrowserKind::Chrome,
        &DriverVersion::Latest,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(v, "126.0.6478.126");
}

#[tokio::test]
async fn resolve_version_exact_chrome_major_picks_latest_in_index() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/chrome-for-testing/known-good-versions-with-downloads.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "versions": [
                { "version": "126.0.6478.10",  "downloads": {"chromedriver": []} },
                { "version": "126.0.6478.126", "downloads": {"chromedriver": []} },
                { "version": "127.0.6533.50",  "downloads": {"chromedriver": []} },
            ]
        })))
        .mount(&server)
        .await;

    let cfg = DownloadConfig {
        cache_dir: std::env::temp_dir(),
        mirror: Mirror {
            chrome_metadata: format!("{}/", server.uri()).parse().unwrap(),
            ..Mirror::default()
        },
        download_timeout: Duration::from_secs(5),
        offline: false,
    };

    let client = reqwest::Client::new();
    let v = resolve_version(
        &client,
        &cfg,
        BrowserKind::Chrome,
        &DriverVersion::Exact("126".into()),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(v, "126.0.6478.126");
}

#[tokio::test]
async fn from_capabilities_errors_when_caps_missing_version() {
    let cfg = DownloadConfig {
        cache_dir: std::env::temp_dir(),
        mirror: Mirror::default(),
        download_timeout: Duration::from_secs(5),
        offline: false,
    };
    let client = reqwest::Client::new();
    let err = resolve_version(
        &client,
        &cfg,
        BrowserKind::Chrome,
        &DriverVersion::FromCapabilities,
        None,
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, super::ManagerError::MissingCapabilityVersion));
}
