//! Newtypes for opaque CDP identifiers.
//!
//! CDP uses many string and integer ids that are easy to mix up at the call
//! site (a `FrameId` is not a `TargetId`, a `NodeId` is not a `BackendNodeId`).
//! These newtypes preserve the wire format while keeping them distinct in
//! Rust.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Construct from any string-like value.
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            /// Borrow the inner string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

macro_rules! int_id {
    ($(#[$meta:meta])* $name:ident($repr:ty)) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $repr);

        impl $name {
            /// Construct from a raw integer.
            pub fn new(v: $repr) -> Self {
                Self(v)
            }

            /// Get the raw integer.
            pub fn get(self) -> $repr {
                self.0
            }
        }

        impl From<$repr> for $name {
            fn from(v: $repr) -> Self {
                Self(v)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

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
    /// Identifier for a Runtime script (`Runtime.ScriptId`).
    ScriptId(i64)
}

int_id! {
    /// Time origin for `Runtime.Timestamp` and `Network.MonotonicTime` —
    /// milliseconds since CDP's monotonic time origin, as `f64`.
    /// Stored unmodified; convert to `Duration` at the call site if needed.
    Timestamp(i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn string_id_serialises_transparently() {
        let id = FrameId::from("F-1");
        assert_eq!(serde_json::to_value(&id).unwrap(), json!("F-1"));
        let back: FrameId = serde_json::from_value(json!("F-1")).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn string_id_constructors_and_accessors() {
        let from_str: RequestId = "R".into();
        let from_string: RequestId = String::from("R").into();
        let new_call = RequestId::new("R");
        assert_eq!(from_str, new_call);
        assert_eq!(from_string, new_call);
        assert_eq!(from_str.as_str(), "R");
        assert_eq!(format!("{from_str}"), "R");
    }

    #[test]
    fn int_id_serialises_transparently() {
        let n = NodeId::new(42);
        assert_eq!(serde_json::to_value(n).unwrap(), json!(42));
        let back: NodeId = serde_json::from_value(json!(42)).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn int_id_constructors_and_accessors() {
        let n = NodeId::from(7);
        assert_eq!(n.get(), 7);
        assert_eq!(format!("{n}"), "7");
    }

    #[test]
    fn distinct_string_ids_are_separate_types() {
        // Compile-time guard: a FrameId is not a RequestId.
        fn _frame(_: FrameId) {}
        fn _request(_: RequestId) {}
        _frame(FrameId::from("X"));
        _request(RequestId::from("X"));
    }

    #[test]
    fn distinct_int_ids_are_separate_types() {
        // Compile-time guard: NodeId vs BackendNodeId.
        fn _node(_: NodeId) {}
        fn _backend(_: BackendNodeId) {}
        _node(NodeId::new(1));
        _backend(BackendNodeId::new(1));
    }

    #[test]
    fn id_hash_eq_work_for_use_as_map_keys() {
        use std::collections::HashMap;
        let mut m = HashMap::new();
        m.insert(SessionId::from("S1"), 1);
        m.insert(SessionId::from("S2"), 2);
        assert_eq!(m.get(&SessionId::from("S1")), Some(&1));
    }
}
