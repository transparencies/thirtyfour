//! Newtypes for opaque CDP identifiers.
//!
//! CDP uses many string and integer ids that are easy to mix up at the call
//! site (a `FrameId` is not a `TargetId`, a `NodeId` is not a `BackendNodeId`).
//! These newtypes preserve the wire format while keeping them distinct in
//! Rust.
//!
//! The `string_id!` and `int_id!` macros live in
//! [`crate::common::protocol`] and are shared with the BiDi layer.

use crate::common::protocol::{int_id, string_id};

string_id! {
    /// Identifier for a frame in the page's frame tree (`Page.FrameId`).
    FrameId
}

string_id! {
    /// Identifier for an in-flight network request (`Network.RequestId`).
    /// One id covers redirects under the same logical request.
    RequestId
}

string_id! {
    /// Identifier for an in-flight resource fetch interception
    /// (`Network.InterceptionId`).
    InterceptionId
}

string_id! {
    /// Identifier for an in-flight `Fetch` interception (`Fetch.RequestId`).
    FetchRequestId
}

string_id! {
    /// Identifier for a remote V8 object (`Runtime.RemoteObjectId`). Held
    /// alive by V8 in a specific execution context.
    RemoteObjectId
}

string_id! {
    /// Identifier for a CDP target (`Target.TargetId`). Stable across
    /// attach/detach.
    TargetId
}

string_id! {
    /// Routing key on a single CDP WebSocket in flat session mode
    /// (`Target.SessionId`).
    SessionId
}

string_id! {
    /// Identifier for an isolation container (`Browser.BrowserContextId`).
    BrowserContextId
}

string_id! {
    /// Identifier for a loader inside the page (`Network.LoaderId`).
    LoaderId
}

string_id! {
    /// Identifier for a Runtime script (`Runtime.ScriptId`). The name
    /// looks numeric but CDP defines it as an opaque string.
    ScriptId
}

int_id! {
    /// Short-lived integer id of a DOM node (`DOM.NodeId`). Valid only while
    /// the DOM agent is enabled in this session.
    NodeId(i64)
}

int_id! {
    /// Stable integer id for a DOM node (`DOM.BackendNodeId`). Survives DOM
    /// agent reattachment and crosses sessions.
    BackendNodeId(i64)
}

int_id! {
    /// Identifier for a Runtime execution context
    /// (`Runtime.ExecutionContextId`).
    ExecutionContextId(i64)
}

int_id! {
    /// Time origin for `Runtime.Timestamp` and `Network.MonotonicTime` —
    /// milliseconds since CDP's monotonic time origin, as `f64`.
    /// Stored unmodified; convert to `Duration` at the call site if needed.
    Timestamp(i64)
}

// Wire-shape coverage for these IDs lives in the integration tests in
// `thirtyfour/tests/cdp_typed.rs` — every command that returns or accepts
// an ID round-trips through real chromedriver, which is the only check
// that actually proves the transparent serde shape against CDP.
//
// The compile-time invariant that distinct IDs (e.g. `FrameId` vs
// `RequestId`, `NodeId` vs `BackendNodeId`) are separate Rust types is
// enforced by the type system itself — no runtime assertion needed.
