use serde::Serialize;

use crate::common::capabilities::chromium::ChromiumLikeCapabilities;
use crate::{BrowserCapabilitiesHelper, Capabilities};

/// Capabilities for Opera.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct OperaCapabilities {
    capabilities: Capabilities,
}

impl Default for OperaCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl OperaCapabilities {
    /// Create a new `OperaCapabilities`.
    pub fn new() -> Self {
        let mut capabilities = Capabilities::new();
        capabilities.set("browserName", "opera").expect("infallible");
        OperaCapabilities {
            capabilities,
        }
    }
}

impl From<OperaCapabilities> for Capabilities {
    fn from(caps: OperaCapabilities) -> Capabilities {
        caps.capabilities
    }
}

impl AsRef<Capabilities> for OperaCapabilities {
    fn as_ref(&self) -> &Capabilities {
        &self.capabilities
    }
}

impl AsMut<Capabilities> for OperaCapabilities {
    fn as_mut(&mut self) -> &mut Capabilities {
        &mut self.capabilities
    }
}

impl BrowserCapabilitiesHelper for OperaCapabilities {
    const KEY: &'static str = "operaOptions";
}

impl ChromiumLikeCapabilities for OperaCapabilities {}
