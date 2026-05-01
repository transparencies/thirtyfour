use serde::Serialize;

use crate::{BrowserCapabilitiesHelper, Capabilities};

/// Capabilities for Internet Explorer.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct InternetExplorerCapabilities {
    capabilities: Capabilities,
}

impl Default for InternetExplorerCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl InternetExplorerCapabilities {
    /// Create a new `InternetExplorerCapabilities`.
    pub fn new() -> Self {
        let mut capabilities = Capabilities::new();
        capabilities.set("browserName", "internet explorer").expect("infallible");
        InternetExplorerCapabilities {
            capabilities,
        }
    }
}

impl From<InternetExplorerCapabilities> for Capabilities {
    fn from(caps: InternetExplorerCapabilities) -> Capabilities {
        caps.capabilities
    }
}

impl AsRef<Capabilities> for InternetExplorerCapabilities {
    fn as_ref(&self) -> &Capabilities {
        &self.capabilities
    }
}

impl AsMut<Capabilities> for InternetExplorerCapabilities {
    fn as_mut(&mut self) -> &mut Capabilities {
        &mut self.capabilities
    }
}

impl BrowserCapabilitiesHelper for InternetExplorerCapabilities {
    const KEY: &'static str = "se:ieOptions";
}
