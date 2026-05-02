# WebDriver BiDi

**WebDriver BiDi** is the [W3C-standard][bidi-abstract] bidirectional
protocol that succeeds parts of Chrome DevTools Protocol with a
**cross-browser** equivalent. It runs over a WebSocket negotiated by
the standard `webSocketUrl: true` capability on `New Session`, and
both Chromium-based browsers (chromedriver ≥ 115) and Firefox
(geckodriver ≥ 0.31) implement it today.

If you've used [CDP](../cdp/overview.md), BiDi will feel familiar:
typed commands grouped by module (`browsingContext.*`, `script.*`,
`network.*`, …), event subscriptions, request interception. The
trade-off is coverage vs. portability:

|                       | CDP (Chromium-only)        | BiDi (cross-browser)       |
|-----------------------|----------------------------|----------------------------|
| Surface area          | Larger, Chromium-specific  | Full W3C BiDi spec covered |
| Portable across browsers | No                      | Yes                        |
| Standardised          | No                         | Yes (W3C)                  |
| Connection            | Direct WebSocket           | Via WebDriver session      |

## Enabling BiDi

BiDi requires both a runtime opt-in (the `webSocketUrl: true`
capability on the session) and the `bidi` feature flag at compile
time:

```toml
[dependencies]
thirtyfour = { version = "THIRTYFOUR_CRATE_VERSION", features = ["bidi"] }
```

`enable_bidi()` on a capabilities builder sets the W3C
`webSocketUrl: true` flag — that's what tells the driver to spin up a
BiDi WebSocket and return its URL on the session capabilities.

## Quick Start

```rust
use thirtyfour::bidi::events::Load;
use thirtyfour::prelude::*;
use futures_util::StreamExt;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> WebDriverResult<()> {
    let mut caps = DesiredCapabilities::chrome();
    caps.enable_bidi()?;             // <-- the W3C `webSocketUrl: true` opt-in.
    let driver = WebDriver::managed(caps).await?;

    // Lazy-connect the BiDi WebSocket on first use; cached afterwards.
    let bidi = driver.bidi().await?;

    let status = bidi.session().status().await?;
    println!("driver ready: {}", status.ready);

    // Pick the active browsing context.
    let context = bidi.browsing_context().top_level().await?;

    // Subscribe to load events and navigate. The typed `subscribe::<E>()`
    // call sends `session.subscribe` for us automatically; when the stream
    // drops, `session.unsubscribe` is sent in the background.
    let mut loads = bidi.subscribe::<Load>().await?;
    bidi.browsing_context().navigate(context.clone(), "https://example.com", None).await?;

    if let Some(load) = loads.next().await {
        println!("loaded: {}", load.url);
    }

    driver.quit().await
}
```

`driver.bidi()` is async because it lazily dials the WebSocket on
first use; it's cached afterwards, so subsequent calls just clone the
handle. Cloning a `BiDi` is cheap (the transport is `Arc`-shared).

## Modules

The curated typed bindings live under [`bidi::modules`][modules-rustdoc]:

| Module             | Use it for                                                                |
|--------------------|---------------------------------------------------------------------------|
| `session`          | `status`, `subscribe`/`unsubscribe`, `end`                                |
| `browser`          | Browser-wide control, user contexts, client windows, download behavior    |
| `browsing_context` | Tabs, frames, navigation, screenshots, printing, locating nodes, CSP      |
| `script`           | `evaluate`, `callFunction`, preload scripts, realms                       |
| `network`          | Interception, modify request/response, auth, data collectors, headers     |
| `storage`          | Cookies and partition lookup                                              |
| `log`              | `log.entryAdded` events                                                   |
| `input`            | `performActions`, `releaseActions`, `setFiles`, `fileDialogOpened` events |
| `permissions`      | `setPermission`                                                           |
| `emulation`        | Geolocation, locale, timezone, screen, user-agent, scripting & touch      |
| `web_extension`    | Install / uninstall web extensions                                        |

Each command in those modules is a Rust struct that implements
[`BidiCommand`], pairing the request type with its response type and
the wire method name. Module facades on `BiDi` (`session()`,
`browsing_context()`, `network()`, …) wrap the most common ones in
convenient async methods. For everything else, build the struct
directly and call `bidi.send(MyCommand { ... }).await`.

A few helpers worth knowing about:

- `bidi.browsing_context().top_level()` — the id of the first
  top-level context (i.e. "the active tab"). Saves you doing
  `tree.contexts[0].context.clone()` by hand.
- `bidi.browsing_context().top_levels()` — every top-level context.
- `bidi.network().add_intercept(...)` — returns an
  [`InterceptGuard`][intercept-guard-rustdoc] you can `.remove().await`
  explicitly, or just let drop (best-effort cleanup runs in the
  background).

## Emulation Overrides

The `emulation` module overrides browser-emulated APIs (geolocation,
locale, timezone, screen, user agent, scripting, touch, …). Each
command applies globally by default; build the command struct directly
to scope it to specific browsing contexts or user contexts. Pass `None`
to clear an override.

```rust
use thirtyfour::bidi::modules::emulation::GeolocationCoordinates;
# use thirtyfour::prelude::*;
# async fn run(bidi: thirtyfour::bidi::BiDi) -> WebDriverResult<()> {
bidi.emulation()
    .set_geolocation_override(Some(GeolocationCoordinates {
        latitude: -33.8688,
        longitude: 151.2093,
        accuracy: Some(50.0),
        altitude: None,
        altitude_accuracy: None,
        heading: None,
        speed: None,
    }))
    .await?;

bidi.emulation().set_timezone_override(Some("Australia/Sydney".into())).await?;
bidi.emulation().set_locale_override(Some("en-AU".into())).await?;
# Ok(()) }
```

> Driver support for `emulation.*` is uneven at the time of writing —
> chromedriver implements most, geckodriver lags. The wire shape is
> stable; commands return `error("unsupported operation")` where the
> driver doesn't yet handle them.

## The Untyped Escape Hatch

Method names and parameter shapes for commands not in the curated set
come from the BiDi spec's [Commands section][bidi-commands] — every
module (`session`, `browsingContext`, `script`, `network`, …) lists
its commands and their wire signatures, e.g.
[`browsingContext.activate`][bidi-bc-activate].

```rust
# use thirtyfour::prelude::*;
# async fn run(bidi: thirtyfour::bidi::BiDi) -> WebDriverResult<()> {
// No-arg command:
let res = bidi.send_raw("session.status", ()).await?;
println!("{res}");

// With params:
bidi.send_raw(
    "browsingContext.activate",
    serde_json::json!({ "context": "abc" }),
).await?;
# Ok(()) }
```

`send_raw` accepts anything `Serialize` for the params. If you reach
for it repeatedly on the same command, implement [`BidiCommand`] for
your own request struct — the same trait the curated modules use.
There's no second-class API.

```rust
use serde::{Deserialize, Serialize};
use thirtyfour::bidi::BidiCommand;

#[derive(Serialize)]
struct ActivateContext { context: String }

#[derive(Deserialize)]
struct ActivateResult {}

impl BidiCommand for ActivateContext {
    const METHOD: &'static str = "browsingContext.activate";
    type Returns = ActivateResult;
}
```

## What About CDP?

You can use both in the same session: CDP is on by default, BiDi opts
in via the capability + feature flag. CDP gives you Chromium-only
surface area that BiDi doesn't yet cover; BiDi gives you a single
event-driven control plane that also works on Firefox. See the
[CDP chapter](../cdp/overview.md) for the comparison.

## Where Next

- For event subscription mechanics, network interception, and
  user-prompt handling, read [BiDi Events](./events.md).
- The [`bidi::modules` API docs][modules-rustdoc] list every typed
  command, response, and event.
- For commands and events not in the curated set, jump straight into
  the [BiDi spec's modules section][bidi-modules].

[bidi-abstract]: https://w3c.github.io/webdriver-bidi/#abstract
[bidi-commands]: https://w3c.github.io/webdriver-bidi/#commands
[bidi-modules]: https://w3c.github.io/webdriver-bidi/#protocol-modules
[bidi-bc-activate]: https://w3c.github.io/webdriver-bidi/#command-browsingContext-activate
[`BidiCommand`]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/trait.BidiCommand.html
[modules-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/modules/index.html
[intercept-guard-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/modules/network/struct.InterceptGuard.html
