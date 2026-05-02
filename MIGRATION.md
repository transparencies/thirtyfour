# Migration guide

## New: WebDriver BiDi support

The `bidi` feature flag adds a typed WebDriver BiDi (W3C bidirectional
protocol) layer alongside the classic HTTP API and CDP. It targets
chromedriver ≥ 115 and geckodriver ≥ 0.31.

```toml
# Cargo.toml
thirtyfour = { version = "0.37", features = ["bidi"] }
```

```rust
use thirtyfour::prelude::*;

let mut caps = DesiredCapabilities::chrome();
caps.enable_bidi()?;                                  // sets `webSocketUrl: true`
let driver = WebDriver::new("http://localhost:4444", caps).await?;

let bidi = driver.bidi().await?;                      // lazy-connect the WS
let status = bidi.session().status().await?;
let tree   = bidi.browsing_context().get_tree(None).await?;
let ctx    = tree.contexts[0].context.clone();

bidi.browsing_context()
    .navigate(ctx.clone(), "https://example.com", None)
    .await?;

let result = bidi.script().evaluate(ctx, "document.title", false).await?;
println!("title: {:?}", result.ok_value());
```

Curated typed bindings for **session, browser, browsingContext, script,
network, storage, log, input, permissions** are under
`thirtyfour::bidi::modules::*`. Events use the same broadcast pattern as
the existing CDP events:

```rust
use thirtyfour::bidi::modules::browsing_context::events::Load;
use futures_util::StreamExt;

bidi.session().subscribe("browsingContext.load").await?;
let mut load_events = bidi.subscribe::<Load>();
// ... navigate, then load_events.next().await ...
```

The handle is cached on the session, so `driver.bidi().await` is cheap on
subsequent calls. Coexists with the classic HTTP API and `driver.cdp()`.

## 0.36 → 0.37

The 0.37 release has two main themes: a full rewrite of the CDP layer,
and an API cleanup that drops the long-deprecation tail and tightens up
a few rough edges. The CDP rewrite is purely additive (the old API is
deprecated, not deleted), but the cleanup involves several breaking
changes — see [API cleanup](#api-cleanup) below.

## CDP rewrite

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

## API cleanup

Several long-deprecated APIs have been removed, and a few existing types
have been tightened up. Most callers will hit one or two of these at
compile time; the diff is usually a one-line rename.

### `WebDriver` construction: builder API

`WebDriver::new_with_config` and `WebDriver::new_with_config_and_client`
have been replaced by [`WebDriver::builder`], which returns a
[`WebDriverBuilder`] that implements `IntoFuture` (so `.await` opens the
session). [`WebDriver::new`] still works as the simple two-arg shortcut.

```rust
// Before:
use thirtyfour::common::config::WebDriverConfig;
let config = WebDriverConfig::builder()
    .reqwest_timeout(Duration::from_secs(30))
    .user_agent("my-app/1.0")
    .build()?;
let driver = WebDriver::new_with_config("http://localhost:4444", caps, config).await?;

// After:
let driver = WebDriver::builder("http://localhost:4444", caps)
    .request_timeout(Duration::from_secs(30))
    .user_agent("my-app/1.0")
    .await?;
```

A custom [`HttpClient`] is now supplied via `.client(...)` on the
builder rather than a separate `new_with_config_and_client` constructor.

### `WebDriverConfig::reqwest_timeout` → `request_timeout`

The field, the [`WebDriverConfig`] builder method, and the
[`WebDriverBuilder`] method are all renamed: the timeout applies to any
[`HttpClient`] implementation, not just the default reqwest-based one.

```rust
// Before:
let config = WebDriverConfig::builder()
    .reqwest_timeout(Duration::from_secs(30))
    .build()?;
let timeout = config.reqwest_timeout;

// After:
let config = WebDriverConfig::builder()
    .request_timeout(Duration::from_secs(30))
    .build()?;
let timeout = config.request_timeout;
```

### `Capabilities` is now a newtype

[`Capabilities`] used to be a type alias for `serde_json::Map<String, Value>`.
It's now a `#[serde(transparent)]` newtype with its own inherent
`get` / `get_mut` / `set` / `remove` / `contains_key` / `len` / `is_empty` /
`iter` methods. The wire format is unchanged.

The most likely breakage is `caps.insert(key, value)` (which came from
the underlying `Map`). The replacement is `caps.set(key, value)?`,
which serialises any `T: Serialize` and returns `WebDriverResult<()>`.

```rust
// Before:
use serde_json::json;
let mut caps = Capabilities::new();
caps.insert("browserName".to_string(), json!("chrome"));

// After:
let mut caps = Capabilities::new();
caps.set("browserName", "chrome")?;
```

If you genuinely need a `serde_json::Map`, `Capabilities` is `From<Map>`
and `Into<Value>`, and `iter()` returns the underlying map's iterator.

### `CapabilitiesHelper` overhaul

The [`CapabilitiesHelper`] trait used to require three private-looking
hook methods (`_get`, `_get_mut`, `insert_base_capability`) on every
implementor. It now has a blanket impl on
`AsRef<Capabilities> + AsMut<Capabilities>`, and its accessors are
renamed to match the inherent API on [`Capabilities`]:

| Before                        | After     |
|-------------------------------|-----------|
| `_get(key)`                   | `get(key)` |
| `_get_mut(key)`               | `get_mut(key)` |
| `set_base_capability(k, v)`   | `set(k, v)` |
| `insert_base_capability(...)` | (removed — use `as_mut().set(...)`) |

If you have a custom capability wrapper, replace the manual
`CapabilitiesHelper for MyCaps` impl with `AsRef`/`AsMut` impls — the
trait then comes for free.

```rust
// Before:
impl CapabilitiesHelper for MyCaps {
    fn _get(&self, key: &str) -> Option<&Value> { self.inner._get(key) }
    fn _get_mut(&mut self, key: &str) -> Option<&mut Value> { self.inner._get_mut(key) }
    fn insert_base_capability(&mut self, key: String, value: Value) {
        self.inner.insert_base_capability(key, value);
    }
}

// After:
impl AsRef<Capabilities> for MyCaps {
    fn as_ref(&self) -> &Capabilities { &self.inner }
}
impl AsMut<Capabilities> for MyCaps {
    fn as_mut(&mut self) -> &mut Capabilities { &mut self.inner }
}
```

### `insert_browser_option` → `set_browser_option`

[`BrowserCapabilitiesHelper::set_browser_option`] is the new name for
the old `insert_browser_option`, for consistency with the `set` naming
used elsewhere. Behaviour is unchanged.

```rust
// Before:
caps.insert_browser_option("binary", "/path/to/chrome")?;

// After:
caps.set_browser_option("binary", "/path/to/chrome")?;
```

### `Alert` and `SwitchTo` types removed

Every method on `Alert` and `SwitchTo` had been a deprecated forwarder
since 0.30.0. Both types are now gone, along with the `prelude`
re-exports. Use the equivalent methods on [`WebDriver`] directly:

| Before                                       | After                                     |
|----------------------------------------------|-------------------------------------------|
| `driver.switch_to().active_element().await` | `driver.active_element().await`           |
| `driver.switch_to().alert().text().await`   | `driver.get_alert_text().await`           |
| `driver.switch_to().alert().accept().await` | `driver.accept_alert().await`             |
| `driver.switch_to().alert().dismiss().await`| `driver.dismiss_alert().await`            |
| `.alert().send_keys(...).await`              | `driver.send_alert_text(...).await`       |
| `.default_content().await`                   | `driver.enter_default_frame().await`      |
| `.frame_number(n).await`                     | `driver.enter_frame(n).await`             |
| `.frame_element(&el).await`                  | `el.clone().enter_frame().await`          |
| `.parent_frame().await`                      | `driver.enter_parent_frame().await`       |
| `.new_window().await` / `.new_tab().await`   | `driver.new_window().await` / `new_tab()` |
| `.window(handle).await`                      | `driver.switch_to_window(handle).await`   |
| `.window_name(name).await`                   | `driver.switch_to_named_window(name).await` |

### Removed deprecated method names

The following method names — all marked deprecated in 0.30.x or 0.32.x —
have been removed. Use the new names that are already in the codebase:

| Removed                                  | Replacement              |
|------------------------------------------|--------------------------|
| `WebElement::rectangle`                  | `rect`                   |
| `WebElement::get_property`               | `prop`                   |
| `WebElement::get_attribute`              | `attr`                   |
| `WebElement::get_css_property`           | `css_value`              |
| `WebElement::find_element`               | `find`                   |
| `WebElement::find_elements`              | `find_all`               |
| `SessionHandle::close`                   | `close_window`           |
| `SessionHandle::page_source`             | `source`                 |
| `SessionHandle::find_element`            | `find`                   |
| `SessionHandle::find_elements`           | `find_all`               |
| `SessionHandle::execute_script`          | `execute`                |
| `SessionHandle::execute_script_async`    | `execute_async`          |
| `SessionHandle::current_window_handle`   | `window`                 |
| `SessionHandle::window_handles`          | `windows`                |
| `SessionHandle::set_timeouts`            | `update_timeouts`        |
| `SessionHandle::get_cookies`             | `get_all_cookies`        |
| `SessionHandle::get_cookie`              | `get_named_cookie`       |
| `SessionHandle::switch_to`               | `switch_to_*` methods    |
| `ScriptRet::value`                       | `json`                   |
| `ScriptRet::get_element`                 | `element`                |
| `ScriptRet::get_elements`                | `elements`               |
| `ElementQuery::all`                      | `all_from_selector`      |
| `ElementQuery::all_required`             | `all_from_selector_required` |
| `DesiredCapabilities::accept_ssl_certs`  | `accept_insecure_certs`  |
| `WebDriverConfig::default_user_agent()`  | `WebDriverConfig::DEFAULT_USER_AGENT` constant |

### Privatised public fields

The redundant `pub` fields on [`WebDriver`] and [`WebElement`] are now
`pub(crate)`. Use the accessor methods instead:

| Before                | After                |
|-----------------------|----------------------|
| `driver.handle`       | `driver.handle()`    |
| `element.handle`      | `element.handle()`   |
| `element.element_id`  | `element.element_id()` (already existed) |

`SessionHandle::new()` and the unused intermediate constructors
(`new_with_config`, `new_with_config_and_guard`) are also private now —
the only path to a `SessionHandle` is via [`WebDriver::new`],
[`WebDriver::builder`], or [`WebDriver::managed`].

### Examples and doctests now use `anyhow`

The `color-eyre` dev-dependency has been replaced with `anyhow` in the
examples and the two doctest snippets in the README and `lib.rs`. This
is purely a docs/example change and doesn't affect the public API, but
if you copied a snippet verbatim you'll see `anyhow::Result<()>` where
it used to say `color_eyre::Result<()>`.

[`WebDriver`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebDriver.html
[`WebDriver::new`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebDriver.html#method.new
[`WebDriver::builder`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebDriver.html#method.builder
[`WebDriver::managed`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebDriver.html#method.managed
[`WebDriverBuilder`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebDriverBuilder.html
[`WebDriverConfig`]: https://docs.rs/thirtyfour/latest/thirtyfour/common/config/struct.WebDriverConfig.html
[`WebElement`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebElement.html
[`Capabilities`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.Capabilities.html
[`CapabilitiesHelper`]: https://docs.rs/thirtyfour/latest/thirtyfour/trait.CapabilitiesHelper.html
[`BrowserCapabilitiesHelper::set_browser_option`]: https://docs.rs/thirtyfour/latest/thirtyfour/trait.BrowserCapabilitiesHelper.html#method.set_browser_option
[`HttpClient`]: https://docs.rs/thirtyfour/latest/thirtyfour/session/http/trait.HttpClient.html

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
