//! Unit tests for the CDP layer.
//!
//! These cover serde round-trips for typed commands/responses and parsing
//! of the JSON-RPC error envelope. Live integration tests against
//! chromedriver / a real WebSocket live in `thirtyfour/tests/cdp_*.rs`.

use serde_json::json;

use crate::cdp::CdpCommand;
use crate::cdp::domains::{
    browser, dom, emulation, input, network, page, performance, runtime, storage, target,
};
use crate::cdp::error::CdpErrorEnvelope;
use crate::cdp::ids::{BackendNodeId, FrameId, NodeId, RequestId, SessionId, TargetId};

#[test]
fn empty_returns_for_void_commands() {
    // Each command that returns Empty should accept the empty-object JSON.
    fn assert_empty<C: CdpCommand>(_c: C)
    where
        C::Returns: serde::de::DeserializeOwned,
    {
        let _: C::Returns = serde_json::from_value(json!({})).expect("Empty parse");
    }
    assert_empty(network::Enable::default());
    assert_empty(network::Disable);
    assert_empty(network::ClearBrowserCache);
    assert_empty(page::Enable);
    assert_empty(page::Reload::default());
    assert_empty(runtime::Enable);
    assert_empty(runtime::Disable);
    assert_empty(dom::Enable);
    assert_empty(dom::ScrollIntoViewIfNeeded::default());
    assert_empty(emulation::ClearDeviceMetricsOverride);
    assert_empty(performance::Enable::default());
    assert_empty(performance::Disable);
    assert_empty(storage::ClearCookies::default());
}

#[test]
fn page_navigate_serialises_camel_case() {
    let cmd = page::Navigate::new("https://example.com/");
    let v = serde_json::to_value(&cmd).unwrap();
    assert_eq!(v["url"], "https://example.com/");
    // Optional fields should be skipped when None.
    assert!(v.get("frameId").is_none());
    assert_eq!(page::Navigate::METHOD, "Page.navigate");
}

#[test]
fn page_navigate_response_camel_case() {
    let body = json!({
        "frameId": "FRAME-1",
        "loaderId": "LOADER-1",
        "errorText": null,
    });
    let r: page::NavigateResult = serde_json::from_value(body).unwrap();
    assert_eq!(r.frame_id, FrameId::from("FRAME-1"));
    assert_eq!(r.loader_id.unwrap().as_str(), "LOADER-1");
    assert!(r.error_text.is_none());
}

#[test]
fn network_emulate_camel_case() {
    let cmd = network::EmulateNetworkConditions {
        offline: false,
        latency: 200,
        download_throughput: 1024,
        upload_throughput: 512,
        connection_type: Some(network::ConnectionType::Cellular3G),
    };
    let v = serde_json::to_value(&cmd).unwrap();
    assert_eq!(v["downloadThroughput"], 1024);
    assert_eq!(v["uploadThroughput"], 512);
    assert_eq!(v["connectionType"], "cellular3g");
    // CDP's wire format is camelCase; the snake_case form would mean the
    // override silently never reached chromium.
    assert!(v.get("download_throughput").is_none());
    assert_eq!(network::EmulateNetworkConditions::METHOD, "Network.emulateNetworkConditions");
}

#[test]
fn network_conditions_round_trip() {
    let original = network::NetworkConditions {
        offline: true,
        latency: 100,
        download_throughput: 2048,
        upload_throughput: 1024,
        connection_type: Some(network::ConnectionType::Wifi),
    };
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["downloadThroughput"], 2048);
    let parsed: network::NetworkConditions = serde_json::from_value(json).unwrap();
    assert_eq!(parsed, original);
}

#[test]
fn dom_node_ids_are_distinct_types() {
    // Compile-time: NodeId and BackendNodeId are different types — this
    // function wouldn't compile if they collapsed to the same alias.
    fn take_node(_: NodeId) {}
    fn take_backend(_: BackendNodeId) {}
    take_node(NodeId::new(1));
    take_backend(BackendNodeId::new(2));
}

#[test]
fn cdp_error_envelope_parses_method_not_found() {
    let raw = json!({
        "code": -32601,
        "message": "'Foo.bar' wasn't found"
    });
    let env: CdpErrorEnvelope = serde_json::from_value(raw).unwrap();
    let err = env.into_error("Foo.bar");
    assert_eq!(err.code, -32601);
    assert_eq!(err.command, "Foo.bar");
    assert!(err.message.contains("Foo.bar"));
    assert!(!err.is_server_error());
}

#[test]
fn cdp_error_envelope_parses_server_error_with_data() {
    let raw = json!({
        "code": -32000,
        "message": "Target closed.",
        "data": {"sessionId": "S1"}
    });
    let env: CdpErrorEnvelope = serde_json::from_value(raw).unwrap();
    let err = env.into_error("Page.navigate");
    assert!(err.is_server_error());
    assert!(err.is_target_closed());
    assert!(err.data.is_some());
}

#[test]
fn target_attach_to_target_flat_mode_sets_flatten_true() {
    let cmd = target::AttachToTarget::flat(TargetId::from("T1"));
    assert!(cmd.flatten);
    let v = serde_json::to_value(&cmd).unwrap();
    assert_eq!(v["flatten"], true);
    assert_eq!(v["targetId"], "T1");
}

#[test]
fn id_newtypes_serialise_transparently() {
    let frame = FrameId::from("frame-1");
    let request = RequestId::from("req-1");
    let session = SessionId::from("sess-1");
    assert_eq!(serde_json::to_value(&frame).unwrap(), json!("frame-1"));
    assert_eq!(serde_json::to_value(&request).unwrap(), json!("req-1"));
    assert_eq!(serde_json::to_value(&session).unwrap(), json!("sess-1"));
    let node = NodeId::new(42);
    assert_eq!(serde_json::to_value(node).unwrap(), json!(42));
}

#[test]
fn input_dispatch_key_event_required_fields() {
    let cmd = input::DispatchKeyEvent {
        r#type: "keyDown".to_string(),
        modifiers: Some(0),
        text: Some("a".to_string()),
        unmodified_text: None,
        key: Some("a".to_string()),
        code: Some("KeyA".to_string()),
        native_virtual_key_code: None,
        windows_virtual_key_code: None,
        auto_repeat: None,
        is_keypad: None,
        is_system_key: None,
    };
    let v = serde_json::to_value(&cmd).unwrap();
    assert_eq!(v["type"], "keyDown");
    assert_eq!(v["text"], "a");
    assert_eq!(v["code"], "KeyA");
    assert!(v.get("nativeVirtualKeyCode").is_none());
}

#[test]
fn version_info_parses_browser_get_version_response() {
    let body = json!({
        "protocolVersion": "1.3",
        "product": "Chrome/121.0.6167.184",
        "revision": "@abcdef",
        "userAgent": "Mozilla/5.0 (X11; Linux x86_64)",
        "jsVersion": "12.1.285"
    });
    let r: browser::VersionInfo = serde_json::from_value(body).unwrap();
    assert_eq!(r.product, "Chrome/121.0.6167.184");
    assert!(r.user_agent.contains("Mozilla"));
}

#[test]
fn network_request_will_be_sent_event_uses_document_url_screaming() {
    // Regression: CDP spells the field `documentURL`, not `documentUrl`.
    // Without an explicit `#[serde(rename = "documentURL")]` the typed
    // event silently failed to deserialise and the typed event stream
    // appeared empty even though the raw event was arriving.
    let body = json!({
        "requestId": "R1",
        "loaderId": "L1",
        "documentURL": "http://localhost/",
        "request": {},
        "timestamp": 0.0,
        "wallTime": 0.0,
        "initiator": {}
    });
    let r: network::RequestWillBeSent = serde_json::from_value(body).unwrap();
    assert_eq!(r.document_url, "http://localhost/");
}

#[test]
fn dom_node_uses_screaming_url_field_names() {
    // Like `Network.requestWillBeSent`, `DOM.Node` uses `documentURL` and
    // `baseURL` on the wire (SCREAMING acronym).
    let body = json!({
        "nodeId": 1,
        "backendNodeId": 2,
        "nodeType": 9,
        "nodeName": "#document",
        "localName": "",
        "nodeValue": "",
        "documentURL": "http://example.com/",
        "baseURL": "http://example.com/"
    });
    let n: dom::DomNode = serde_json::from_value(body).unwrap();
    assert_eq!(n.document_url.as_deref(), Some("http://example.com/"));
    assert_eq!(n.base_url.as_deref(), Some("http://example.com/"));
}

#[test]
fn target_get_targets_response_camel_case() {
    // Regression: `targetInfos` was deserialising via the snake_case Rust
    // field name. Lock the wire shape.
    let body = json!({
        "targetInfos": [
            {
                "targetId": "T-1",
                "type": "page",
                "title": "",
                "url": "about:blank",
                "attached": true,
            }
        ]
    });
    let r: target::GetTargetsResult = serde_json::from_value(body).unwrap();
    assert_eq!(r.target_infos.len(), 1);
    assert_eq!(r.target_infos[0].target_id.as_str(), "T-1");
}

#[test]
fn unit_struct_commands_serialise_to_null() {
    // Documents the wire shape: unit-struct CDP commands serialise to
    // `null`, which the transports must rewrite to `{}` to keep
    // chromedriver happy ("invalid argument: params not passed").
    let v = serde_json::to_value(browser::GetVersion).unwrap();
    assert!(v.is_null(), "unit struct serialises to null; transport coerces to {{}}");
    let v = serde_json::to_value(network::ClearBrowserCache).unwrap();
    assert!(v.is_null());
}

#[test]
fn cdp_command_method_strings_are_correct() {
    assert_eq!(browser::GetVersion::METHOD, "Browser.getVersion");
    assert_eq!(network::Enable::METHOD, "Network.enable");
    assert_eq!(network::ClearBrowserCache::METHOD, "Network.clearBrowserCache");
    assert_eq!(page::CaptureScreenshot::METHOD, "Page.captureScreenshot");
    assert_eq!(runtime::Evaluate::METHOD, "Runtime.evaluate");
    assert_eq!(dom::QuerySelector::METHOD, "DOM.querySelector");
    assert_eq!(emulation::SetDeviceMetricsOverride::METHOD, "Emulation.setDeviceMetricsOverride");
    assert_eq!(input::DispatchKeyEvent::METHOD, "Input.dispatchKeyEvent");
    assert_eq!(target::SetAutoAttach::METHOD, "Target.setAutoAttach");
}
