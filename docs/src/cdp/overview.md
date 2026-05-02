# Chrome DevTools Protocol

**Chrome DevTools Protocol** (CDP) is the lower-level inspection and
control API that Chromium-based browsers expose for debugging,
profiling, and automation. It predates W3C WebDriver BiDi and only
works on Chromium browsers (Chrome, Edge, Brave, Opera, …) — but it
covers a large surface area that WebDriver itself doesn't, including
network throttling, fine-grained DOM inspection, runtime JS evaluation
with object handles, request interception, device emulation, and more.

`thirtyfour` ships typed bindings for the most-used CDP commands so
you can call them like normal Rust async methods, plus an optional
WebSocket-based session for **event subscription** (network requests
firing, lifecycle events, console messages, …).

## When To Use CDP vs BiDi

| You want…                                             | Use     |
|-------------------------------------------------------|---------|
| Cross-browser support (Chrome **and** Firefox)        | [BiDi]  |
| Rich Chromium-only features (full Network, Fetch, DOM) | CDP     |
| W3C-standard, future-proof bidirectional protocol     | [BiDi]  |
| To call any of `Network.*`, `Fetch.*`, `Runtime.*`, etc. | CDP   |
| To resolve a `WebElement` to a CDP `RemoteObjectId`   | CDP     |

Both can be used in the same session. CDP is on by default; BiDi opts
in via a capability and a feature flag — see the [BiDi chapter][BiDi]
for details.

[BiDi]: ../bidi/overview.md

## Quick Start

CDP is enabled by the `cdp` feature, which is on by default. Reach
the handle via `WebDriver::cdp()`:

```rust
use thirtyfour::cdp::domains::network::{ConnectionType, NetworkConditions};
use thirtyfour::prelude::*;

#[tokio::main]
async fn main() -> WebDriverResult<()> {
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;

    // Domain-grouped typed methods.
    let info = driver.cdp().browser().get_version().await?;
    println!("Chrome: {}", info.product);

    // Throttle the network like Chrome DevTools' "Slow 3G".
    driver.cdp().network().emulate_network_conditions(NetworkConditions {
        offline: false,
        latency: 200,
        download_throughput: 256 * 1024,
        upload_throughput: 64 * 1024,
        connection_type: Some(ConnectionType::Cellular3G),
    }).await?;

    driver.cdp().network().clear_browser_cache().await?;

    driver.quit().await
}
```

`driver.cdp()` is cheap to call — it just clones the underlying
`Arc<SessionHandle>`. There is no separate connection: typed commands
flow over the WebDriver vendor endpoint `goog/cdp/execute`, so they
work on any session backed by a Chromium driver (chromedriver,
msedgedriver, Brave's driver, …) and even through Selenium Grid.

## Typed Commands

Every command in [`cdp::domains`] is a Rust struct that pairs the
request type with its response type and the wire method name. The
domain facades on `Cdp` (`browser()`, `network()`, `page()`, …) wrap
the most common ones in convenient async methods:

| Domain        | Examples                                                                       |
|---------------|--------------------------------------------------------------------------------|
| `browser`     | `get_version`                                                                  |
| `page`        | `navigate`, `reload`, `get_frame_tree`, `capture_screenshot`                   |
| `network`     | `enable`, `clear_browser_cache`, `set_extra_http_headers`, `emulate_network_conditions` |
| `fetch`       | `enable`, `continue_request`, `fail_request`, `fulfill_request`                |
| `runtime`     | `evaluate`, `call_function_on`, `enable`, `disable`                            |
| `dom`         | `describe_node`, `get_box_model`, `query_selector`                             |
| `emulation`   | `set_device_metrics_override`, `set_user_agent_override`                       |
| `input`       | `dispatch_mouse_event`, `dispatch_key_event`                                   |
| `target`      | `get_targets`, `attach_to_target`, `detach_from_target`                        |
| `storage`     | `clear_data_for_origin`, `get_cookies`                                         |
| `log`         | `enable`, `clear`                                                              |
| `performance` | `enable`, `get_metrics`                                                        |

For the full list of fields on each command and response, see the
[`thirtyfour::cdp::domains` API docs][cdp-domains-rustdoc].

## The Untyped Escape Hatch

For one-off commands not in the curated set, use `Cdp::send_raw`. The
[CDP protocol viewer][cdp-spec] is the canonical reference for method
names and parameter shapes — every domain has its own page (e.g.
[Browser][cdp-browser-domain], [Page][cdp-page-domain],
[Network][cdp-network-domain]).

```rust
# use thirtyfour::prelude::*;
# async fn run(driver: WebDriver) -> WebDriverResult<()> {
// No-arg command: pass `()` for the params.
let info = driver.cdp().send_raw("Browser.getVersion", ()).await?;
println!("user agent: {}", info["userAgent"]);

// With params:
driver.cdp().send_raw(
    "Page.navigate",
    serde_json::json!({ "url": "https://example.com" }),
).await?;
# Ok(()) }
```

`send_raw` accepts anything `Serialize` for the params, so you can
also pass your own request struct. If you find yourself reaching for
`send_raw` on the same command repeatedly, implement [`CdpCommand`]
for your own request struct and get the typed `send` method back.
This is the same trait the curated domains use — there's no
second-class API.

```rust
use serde::{Deserialize, Serialize};
use thirtyfour::cdp::CdpCommand;

#[derive(Serialize)]
struct GetTitle;

#[derive(Deserialize)]
struct GetTitleResult { title: String }

impl CdpCommand for GetTitle {
    const METHOD: &'static str = "Page.getTitle";
    type Returns = GetTitleResult;
}
```

## Resolving WebElements To CDP Handles

When you already have a `WebElement` and want to hand it to a CDP
command (e.g. `DOM.getBoxModel`, `Runtime.callFunctionOn`), there are
two helpers on `WebElement`:

- [`cdp_remote_object_id`] — short-lived `RemoteObjectId` for
  `Runtime.*` calls.
- [`cdp_backend_node_id`] — stable `BackendNodeId` for `DOM.*` calls.

```rust
# use thirtyfour::prelude::*;
# async fn run(driver: WebDriver) -> WebDriverResult<()> {
use thirtyfour::cdp::domains::dom::GetBoxModel;

let elem = driver.find(By::Css("button.submit")).await?;
let backend_id = elem.cdp_backend_node_id().await?;

let box_model = driver.cdp().send(GetBoxModel {
    backend_node_id: Some(backend_id),
    ..Default::default()
}).await?;
println!("box model: {:?}", box_model.model);
# Ok(()) }
```

These are bridges between the WebDriver and CDP worlds — they let you
use `WebElement` to find things and CDP to inspect them.

## Where Next

- For event subscription (`Network.requestWillBeSent`,
  `Page.lifecycleEvent`, `Runtime.consoleAPICalled`, …), enable the
  `cdp-events` feature and read [CDP Events](./events.md).
- For the cross-browser counterpart, see [WebDriver BiDi][BiDi].
- The [`cdp::domains` API docs][cdp-domains-rustdoc] list every typed
  command and its fields.
- The CDP spec itself lives at <https://chromedevtools.github.io/devtools-protocol/>.

[`CdpCommand`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/trait.CdpCommand.html
[`cdp::domains`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/domains/index.html
[cdp-domains-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/domains/index.html
[`cdp_remote_object_id`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebElement.html#method.cdp_remote_object_id
[`cdp_backend_node_id`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebElement.html#method.cdp_backend_node_id
[cdp-spec]: https://chromedevtools.github.io/devtools-protocol/
[cdp-browser-domain]: https://chromedevtools.github.io/devtools-protocol/tot/Browser/
[cdp-page-domain]: https://chromedevtools.github.io/devtools-protocol/tot/Page/
[cdp-network-domain]: https://chromedevtools.github.io/devtools-protocol/tot/Network/
