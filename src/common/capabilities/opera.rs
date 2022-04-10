use crate::CapabilitiesHelper;
use fantoccini::wd::Capabilities;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct OperaCapabilities {
    capabilities: Capabilities,
}

impl Default for OperaCapabilities {
    fn default() -> Self {
        let mut capabilities = Capabilities::new();
        capabilities.insert("browserName".to_string(), json!("opera"));
        OperaCapabilities {
            capabilities,
        }
    }
}

impl OperaCapabilities {
    pub fn new() -> Self {
        OperaCapabilities::default()
    }
}

impl CapabilitiesHelper for OperaCapabilities {
    fn get(&self, key: &str) -> Option<&Value> {
        self.capabilities.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.capabilities.get_mut(key)
    }

    fn set(&mut self, key: String, value: Value) {
        self.capabilities.insert(key, value);
    }
}

impl From<OperaCapabilities> for Capabilities {
    fn from(caps: OperaCapabilities) -> Capabilities {
        caps.capabilities
    }
}
