use serde::Serialize;

use crate::common::capabilities::chromium::ChromiumLikeCapabilities;
use crate::{BrowserCapabilitiesHelper, Capabilities};

/// Capabilities for Microsoft Edge.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct EdgeCapabilities {
    capabilities: Capabilities,
}

impl Default for EdgeCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeCapabilities {
    /// Create a new `EdgeCapabilities`.
    pub fn new() -> Self {
        let mut capabilities = Capabilities::new();
        capabilities.set("browserName", "MicrosoftEdge").expect("infallible");
        EdgeCapabilities {
            capabilities,
        }
    }
}

impl From<EdgeCapabilities> for Capabilities {
    fn from(caps: EdgeCapabilities) -> Capabilities {
        caps.capabilities
    }
}

impl AsRef<Capabilities> for EdgeCapabilities {
    fn as_ref(&self) -> &Capabilities {
        &self.capabilities
    }
}

impl AsMut<Capabilities> for EdgeCapabilities {
    fn as_mut(&mut self) -> &mut Capabilities {
        &mut self.capabilities
    }
}

impl BrowserCapabilitiesHelper for EdgeCapabilities {
    const KEY: &'static str = "ms:edgeOptions";
}

impl ChromiumLikeCapabilities for EdgeCapabilities {}
