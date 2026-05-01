# Migration guide

## 0.36 → 0.37 — CDP rewrite

The Chrome DevTools Protocol (CDP) layer has been rewritten. The old
[`thirtyfour::extensions::cdp::ChromeDevTools`] still works (deprecated)
so existing code keeps compiling, but new code should use the new
[`thirtyfour::cdp`] module.

### What changed

- A new top-level [`thirtyfour::cdp`] module with typed CDP commands
  grouped by domain. Get a handle via [`WebDriver::cdp`] (no manual
  `Arc::clone(&driver.handle)` needed).
- One typed entry point: [`Cdp::send`] takes a request struct that
  implements [`CdpCommand`] and returns its associated response type.
- Domain facades — `driver.cdp().page().navigate(...)`,
  `driver.cdp().network().clear_browser_cache()`, etc. — for ergonomics.
- `Cdp::send_raw(method, params) -> Value` is the new untyped escape
  hatch (replaces `execute_cdp` / `execute_cdp_with_params`).
- New optional `cdp-events` feature: a WebSocket-backed
  [`CdpSession`] with event subscription via flat-mode session
  multiplexing. Powered by `tokio-tungstenite`.
- New element ↔ CDP bridge: [`WebElement::cdp_remote_object_id`] and
  [`WebElement::cdp_backend_node_id`].
- `ChromeDevTools` and the rest of `extensions::cdp` are now deprecated
  re-exports kept for compatibility. They will be removed in a future
  release.

### Quick before/after

```rust
// Before:
use thirtyfour::extensions::cdp::ChromeDevTools;
let dev = ChromeDevTools::new(driver.handle.clone());
let v = dev.execute_cdp("Browser.getVersion").await?;
let ua = v["userAgent"].as_str().unwrap();

// After:
let info = driver.cdp().browser().get_version().await?;
let ua = info.user_agent;
```

```rust
// Before — raw command with params:
dev.execute_cdp_with_params(
    "Network.setCacheDisabled",
    serde_json::json!({"cacheDisabled": true}),
).await?;

// After — raw escape hatch (or use a typed struct):
driver.cdp().send_raw(
    "Network.setCacheDisabled",
    serde_json::json!({"cacheDisabled": true}),
).await?;
```

### Network conditions

The legacy [`extensions::cdp::NetworkConditions`] wraps chromedriver's
`/chromium/network_conditions` vendor endpoint (snake_case fields).
The new [`cdp::domains::network::NetworkConditions`] wraps the standard
CDP `Network.emulateNetworkConditions` command (camelCase on the wire,
camelCase via `rename_all`). Prefer the new one — it's portable across
Chrome, Edge, Brave and Opera, and goes through the same code path as
all the other typed CDP commands.

```rust
// Before:
use thirtyfour::extensions::cdp::{ChromeDevTools, NetworkConditions};
let dev = ChromeDevTools::new(driver.handle.clone());
let mut conditions = NetworkConditions::new();
conditions.download_throughput = 256 * 1024;
dev.set_network_conditions(&conditions).await?;

// After:
use thirtyfour::cdp::domains::network::NetworkConditions;
driver.cdp().network().emulate_network_conditions(NetworkConditions {
    offline: false,
    latency: 0,
    download_throughput: 256 * 1024,
    upload_throughput: -1,
    connection_type: None,
}).await?;
```

### Events (new feature)

Enable the `cdp-events` feature to get event subscription via a
WebSocket-backed [`CdpSession`]:

```rust
let cdp = driver.cdp();
let session = cdp.connect().await?;       // resolves the CDP WS URL
session.send(thirtyfour::cdp::domains::network::Enable::default()).await?;
let mut events = session.subscribe::<thirtyfour::cdp::domains::network::RequestWillBeSent>();

driver.goto("https://example.com").await?;

use futures_util::StreamExt;
while let Some(event) = events.next().await {
    println!("request: {}", event.request_id);
}
```

URL discovery looks at `se:cdp` (set by Selenium Grid),
`webSocketDebuggerUrl` directly on the session, and finally
`goog:chromeOptions.debuggerAddress` (or the Edge equivalent) →
`/json/version`. The W3C `webSocketUrl` capability is intentionally
**not** used because that's a BiDi endpoint, not CDP.

### What's deprecated

- `thirtyfour::extensions::cdp::ChromeDevTools` — use [`Cdp`].
- `ChromeDevTools::execute_cdp` / `execute_cdp_with_params` — use
  [`Cdp::send`] / [`Cdp::send_raw`].
- `thirtyfour::extensions::cdp::NetworkConditions` /
  `ConnectionType` — use the equivalents in
  [`cdp::domains::network`].
- `thirtyfour::extensions::cdp::ChromeCommand` — internal type, but
  marked deprecated since the new path doesn't go through it.

[`thirtyfour::cdp`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/
[`thirtyfour::extensions::cdp::ChromeDevTools`]: https://docs.rs/thirtyfour/latest/thirtyfour/extensions/cdp/struct.ChromeDevTools.html
[`Cdp`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.Cdp.html
[`Cdp::send`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.Cdp.html#method.send
[`Cdp::send_raw`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.Cdp.html#method.send_raw
[`CdpCommand`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/trait.CdpCommand.html
[`CdpSession`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.CdpSession.html
[`WebDriver::cdp`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebDriver.html#method.cdp
[`WebElement::cdp_remote_object_id`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebElement.html#method.cdp_remote_object_id
[`WebElement::cdp_backend_node_id`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebElement.html#method.cdp_backend_node_id
[`extensions::cdp::NetworkConditions`]: https://docs.rs/thirtyfour/latest/thirtyfour/extensions/cdp/struct.NetworkConditions.html
[`cdp::domains::network::NetworkConditions`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/domains/network/struct.NetworkConditions.html
[`cdp::domains::network`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/domains/network/index.html
