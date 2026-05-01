use serde::Serialize;

use crate::Capabilities;

/// Capabilities for Safari.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct SafariCapabilities {
    capabilities: Capabilities,
}

impl Default for SafariCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl SafariCapabilities {
    /// Create a new `SafariCapabilities`.
    pub fn new() -> Self {
        let mut capabilities = Capabilities::new();
        capabilities.set("browserName", "safari").expect("infallible");
        SafariCapabilities {
            capabilities,
        }
    }
}

impl From<SafariCapabilities> for Capabilities {
    fn from(caps: SafariCapabilities) -> Capabilities {
        caps.capabilities
    }
}

impl AsRef<Capabilities> for SafariCapabilities {
    fn as_ref(&self) -> &Capabilities {
        &self.capabilities
    }
}

impl AsMut<Capabilities> for SafariCapabilities {
    fn as_mut(&mut self) -> &mut Capabilities {
        &mut self.capabilities
    }
}
