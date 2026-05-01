use serde::Serialize;

use crate::common::capabilities::chromium::ChromiumLikeCapabilities;
use crate::{BrowserCapabilitiesHelper, Capabilities};

/// Capabilities for Chrome.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct ChromeCapabilities {
    capabilities: Capabilities,
}

impl Default for ChromeCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl ChromeCapabilities {
    /// Create a new ChromeCapabilities struct.
    pub fn new() -> Self {
        let mut capabilities = Capabilities::new();
        capabilities.set("browserName", "chrome").expect("infallible: string literal");
        ChromeCapabilities {
            capabilities,
        }
    }
}

impl AsRef<Capabilities> for ChromeCapabilities {
    fn as_ref(&self) -> &Capabilities {
        &self.capabilities
    }
}

impl AsMut<Capabilities> for ChromeCapabilities {
    fn as_mut(&mut self) -> &mut Capabilities {
        &mut self.capabilities
    }
}

impl BrowserCapabilitiesHelper for ChromeCapabilities {
    const KEY: &'static str = "goog:chromeOptions";
}

impl ChromiumLikeCapabilities for ChromeCapabilities {}

impl From<ChromeCapabilities> for Capabilities {
    fn from(caps: ChromeCapabilities) -> Capabilities {
        caps.capabilities
    }
}
