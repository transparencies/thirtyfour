# CDP Events

The default `cdp` feature gives you typed CDP **commands** over the
WebDriver vendor endpoint. To **listen for CDP events**
(`Network.requestWillBeSent`, `Page.lifecycleEvent`,
`Runtime.consoleAPICalled`, …) you need a real WebSocket connection
to the browser's DevTools port. That's what the optional `cdp-events`
feature provides.

```toml
[dependencies]
thirtyfour = { version = "THIRTYFOUR_CRATE_VERSION", features = ["cdp-events"] }
```

`cdp-events` pulls in `tokio-tungstenite` and adds:

- [`Cdp::connect`] — open a WebSocket session.
- [`CdpSession`] — a session bound to one CDP `sessionId`, supporting
  both commands and event subscriptions.

## How A Session Is Established

`Cdp::connect` discovers the browser-level CDP WebSocket URL from the
session's capabilities (in priority order: Selenium Grid's `se:cdp`,
the W3C `webSocketDebuggerUrl` field, then resolving
`goog:chromeOptions.debuggerAddress` via
`/json/version`). It dials the socket, calls `Target.getTargets`, picks
the active page target, and runs `Target.attachToTarget` in **flat
mode** so a single connection can multiplex multiple sessions.

You don't have to think about any of that — it all happens inside
`connect`:

```rust
# use thirtyfour::prelude::*;
# async fn run() -> WebDriverResult<()> {
let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;
let session = driver.cdp().connect().await?;
println!("attached as session {:?}", session.session_id());
# driver.quit().await }
```

## Subscribing To A Typed Event

Subscribe by type with [`CdpSession::subscribe`]. The session
remembers which CDP domains have been enabled and sends
`Network.enable` / `Page.enable` / `Runtime.enable` / `Log.enable` for
you the first time you ask for an event from that domain — no manual
`session.send(network::Enable::default())` ceremony needed:

```rust
use futures_util::StreamExt;
use thirtyfour::cdp::events::RequestWillBeSent;
use thirtyfour::prelude::*;

# async fn run(driver: WebDriver) -> WebDriverResult<()> {
let session = driver.cdp().connect().await?;

let mut requests = session.subscribe::<RequestWillBeSent>().await?;

driver.goto("https://example.com").await?;

while let Some(event) = requests.next().await {
    println!("→ {} (frame {:?})", event.document_url, event.loader_id);
    if event.document_url.starts_with("https://example.com") { break; }
}
# driver.quit().await }
```

Each typed event implements [`CdpEvent`], which pairs the wire method
name with a `Deserialize` struct and (optionally) a domain-enable
command. The stream is backed by a broadcast channel scoped to the
session — call `subscribe` as many times as you like.

## Common Event Types

The most-used events are re-exported from
[`thirtyfour::cdp::events`][cdp-events-rustdoc] so a single `use`
covers the common cases:

```rust
use thirtyfour::cdp::events::{
    RequestWillBeSent, ResponseReceived, LoadingFinished, LoadingFailed,
    LifecycleEvent, FrameNavigated, LoadEventFired,
    ConsoleApiCalled, ExceptionThrown,
    LogEntryAdded,
    RequestPaused,                         // Fetch — manual enable required
    AttachedToTarget, DetachedFromTarget,  // Target — manual enable required
};
```

| Domain         | Auto-enabled? | Typical events                                                              |
|----------------|----------------|-----------------------------------------------------------------------------|
| `network`      | Yes            | `RequestWillBeSent`, `ResponseReceived`, `LoadingFinished`, `LoadingFailed` |
| `page`         | Yes            | `LifecycleEvent`, `FrameNavigated`, `LoadEventFired`                        |
| `runtime`      | Yes            | `ConsoleApiCalled`, `ExceptionThrown`                                       |
| `log`          | Yes            | `LogEntryAdded`                                                             |
| `fetch`        | No             | `RequestPaused` — call `session.send(fetch::Enable { ... })` first         |
| `target`       | No             | `AttachedToTarget`, `DetachedFromTarget` — call `Target.setDiscoverTargets` |

For the full list and field details, see the
[`thirtyfour::cdp::domains` API docs][cdp-domains-rustdoc].

## Raw Events

When you need every event on the connection — for example because
you're observing child sessions opened by auto-attach, or because the
event isn't yet in the curated set — switch to a raw stream:

- [`CdpSession::subscribe_all`] — every event on **this** session as
  a `(method, params, session_id)` tuple.
- [`CdpSession::subscribe_connection`] — every event on the
  underlying WebSocket regardless of session id.

```rust
use futures_util::StreamExt;
# use thirtyfour::prelude::*;
# async fn run(driver: WebDriver) -> WebDriverResult<()> {
let session = driver.cdp().connect().await?;
let mut all = session.subscribe_all();
while let Some(raw) = all.next().await {
    println!("[{:?}] {}: {}", raw.session_id, raw.method, raw.params);
}
# driver.quit().await }
```

Note that raw streams won't auto-enable any domain — that's only the
typed `subscribe::<E>()` path. For raw, send the relevant `*.enable`
yourself.

## Adding Your Own Typed Event

If a CDP event isn't in the curated set, write a `Deserialize` struct
matching the wire shape and `impl CdpEvent for …`. Then
`subscribe::<MyEvent>().await?` just works. Method names, parameter
shapes, and which `*.enable` (if any) gates the event all come from
the [CDP protocol viewer][cdp-spec] — each domain has its own page,
e.g. [Network][cdp-network-domain].

```rust
use serde::Deserialize;
use thirtyfour::cdp::CdpEvent;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebSocketCreated {
    request_id: String,
    url: String,
}

impl CdpEvent for WebSocketCreated {
    const METHOD: &'static str = "Network.webSocketCreated";
    // Network.* events fire only after Network.enable. The auto-enable path
    // picks this up from the const below.
    const ENABLE: Option<&'static str> = Some("Network.enable");
}
```

Set `ENABLE = None` for events whose domain doesn't have a generic
`enable` command (or where the default-enable would be too aggressive,
like `Fetch.enable` with empty patterns intercepting everything). The
user can still use the event — they just have to send the appropriate
enable themselves.

> **Watch out for `documentURL` and friends.** CDP uses SCREAMING
> acronyms on a number of fields (`documentURL`, `baseURL`, `requestURL`).
> A blanket `rename_all = "camelCase"` produces `documentUrl` (lowercase
> `u`) which won't deserialise. Add an explicit
> `#[serde(rename = "documentURL")]` to those fields. Events whose wire
> shape can't be deserialised as the typed struct are logged via
> `tracing::warn!` (target `thirtyfour::cdp`) and skipped — install a
> tracing subscriber to see them.

## Lifecycle

A `CdpSession` is `Clone` (the underlying transport is `Arc`-shared) and
detaches when explicitly dropped via [`CdpSession::detach`]. Letting it
fall out of scope is also fine — it just won't send a
`Target.detachFromTarget`.

[`Cdp::connect`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.Cdp.html#method.connect
[`CdpSession`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.CdpSession.html
[`CdpSession::subscribe`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.CdpSession.html#method.subscribe
[`CdpSession::subscribe_all`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.CdpSession.html#method.subscribe_all
[`CdpSession::subscribe_connection`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.CdpSession.html#method.subscribe_connection
[`CdpSession::detach`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/struct.CdpSession.html#method.detach
[`CdpEvent`]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/trait.CdpEvent.html
[cdp-events-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/events/index.html
[cdp-domains-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/cdp/domains/index.html
[cdp-spec]: https://chromedevtools.github.io/devtools-protocol/
[cdp-network-domain]: https://chromedevtools.github.io/devtools-protocol/tot/Network/
