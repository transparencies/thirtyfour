# BiDi Events

The driver only delivers events the client has explicitly subscribed
to via `session.subscribe`. The typed `bidi.subscribe::<E>()` call
takes care of that for you — it sends the wire-level subscribe on
first call (per event method, per connection) and tears it down with
`session.unsubscribe` when the last stream drops. You just open a
stream and pull events off it.

## Typed Subscription

```rust
use futures_util::StreamExt;
use thirtyfour::bidi::events::LogEntryAdded;
use thirtyfour::bidi::modules::log::LogLevel;
use thirtyfour::prelude::*;

# async fn run(driver: WebDriver) -> WebDriverResult<()> {
let bidi = driver.bidi().await?;

// Auto-sends `session.subscribe`.
let mut events = bidi.subscribe::<LogEntryAdded>().await?;

let context = bidi.browsing_context().top_level().await?;
bidi.browsing_context()
    .navigate(
        context,
        "data:text/html,<script>console.log('hi');console.warn('warn')</script>",
        None,
    )
    .await?;

while let Some(entry) = events.next().await {
    let level = match entry.level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info  => "INFO",
        LogLevel::Warn  => "WARN",
        LogLevel::Error => "ERROR",
        LogLevel::Unknown(ref s) => s.as_str(),
    };
    println!("[{level}] {}", entry.text.as_deref().unwrap_or(""));
}
# Ok(()) }
```

A typed event implements [`BidiEvent`], pairing the wire method name
with a `Deserialize` struct. Items where the wire shape can't be
deserialised as the requested type are logged via `tracing::warn!`
(target `thirtyfour::bidi`) and skipped — install a tracing subscriber
to see them.

## Common Event Types

The most-used events are re-exported from
[`thirtyfour::bidi::events`][events-rustdoc] so a single `use` covers
the common cases:

```rust
use thirtyfour::bidi::events::{
    Load, DomContentLoaded, NavigationStarted, FragmentNavigated,
    ContextCreated, ContextDestroyed,
    UserPromptOpened, UserPromptClosed,
    BeforeRequestSent, ResponseStarted, ResponseCompleted, FetchError, AuthRequired,
    LogEntryAdded,
};
```

## Scoped Subscriptions

For events scoped to one browsing context (or one user context),
use the explicit `session.send(Subscribe { ... })` form — the
auto-subscribe path is global. Two patterns:

```rust
# use thirtyfour::prelude::*;
# async fn run(bidi: thirtyfour::bidi::BiDi) -> WebDriverResult<()> {
use thirtyfour::bidi::modules::session::Subscribe;

let context = bidi.browsing_context().top_level().await?;

// Wire-level subscribe scoped to one tab. Then open the local stream
// to read it (the typed subscribe also bumps the global refcount, so
// `unsubscribe` won't fire mid-test if you mix them — but the cleanest
// pattern is to pick one approach per stream).
bidi.send(Subscribe {
    events: vec!["browsingContext.load".into()],
    contexts: vec![context],
    user_contexts: vec![],
}).await?;
let mut raw = bidi.subscribe_raw();
# Ok(()) }
```

You can also subscribe by **whole module name** (e.g. `"browsingContext"`)
to opt in to every event in that module, again via the explicit form.

> The full list of valid event names lives in the BiDi spec's
> [Events section][bidi-events] — each module documents its own events,
> e.g. [`browsingContext.load`][bidi-bc-load]. Module-level subscribes
> (`"browsingContext"`, `"network"`, `"log"`, …) match the module
> headers in the spec.

## Raw Events

When the event isn't yet in the curated set, or you want a single
firehose for debugging, switch to a raw stream:

```rust
use futures_util::StreamExt;
# use thirtyfour::prelude::*;
# async fn run(bidi: thirtyfour::bidi::BiDi) -> WebDriverResult<()> {
bidi.session().subscribe_many(["network".into(), "browsingContext".into()]).await?;
let mut all = bidi.subscribe_raw();
while let Some(event) = all.next().await {
    println!("{}: {}", event.method, event.params);
}
# Ok(()) }
```

[`BiDi::subscribe_raw`] yields every event delivered on the
connection without filtering or deserialisation. Note it doesn't
auto-subscribe — that's why we explicitly call
`session().subscribe_many(...)` first.

## Network Interception

Network interception is a request-pause-and-continue loop driven by
events:

1. Subscribe to the appropriate event (`BeforeRequestSent` for the
   request phase, `ResponseStarted` for the response phase) **before**
   registering the intercept — otherwise you might miss the event
   you're waiting on.
2. Register the intercept with `bidi.network().add_intercept(...)`,
   choosing one or more [`InterceptPhase`]s and optional URL match
   patterns. The return value is an [`InterceptGuard`][intercept-guard-rustdoc]
   that wraps the underlying id.
3. The next matching request pauses on the wire. The event arrives
   with `is_blocked = true` and a `request.id` to identify it.
4. Continue the request unmodified
   (`bidi.network().continue_request(event.request.id)`), modify it,
   fail it (`fail_request`), or synthesize a response
   (`provide_response`). For response-phase interception, use
   `continue_response`.
5. When you're done, drop the guard or call `intercept.remove().await`
   for explicit error handling.

```rust
use futures_util::StreamExt;
use thirtyfour::bidi::events::BeforeRequestSent;
use thirtyfour::bidi::modules::browsing_context::ReadinessState;
use thirtyfour::bidi::modules::network;
use thirtyfour::prelude::*;

# async fn run(driver: WebDriver) -> WebDriverResult<()> {
let bidi = driver.bidi().await?;

// (1) Subscribe BEFORE adding the intercept (auto-sends `session.subscribe`).
let mut events = bidi.subscribe::<BeforeRequestSent>().await?;

// (2) Register the intercept. The returned guard removes the intercept
//     when it drops; call .remove() explicitly if you want the error.
let intercept = bidi
    .network()
    .add_intercept(vec![network::InterceptPhase::BeforeRequestSent], None)
    .await?;

let context = bidi.browsing_context().top_level().await?;

// (3) Kick off navigation in the background — it won't return until
//     we continue the paused request.
let nav = {
    let bidi = bidi.clone();
    let context = context.clone();
    tokio::spawn(async move {
        bidi.browsing_context()
            .navigate(context, "https://example.com/", Some(ReadinessState::Complete))
            .await
    })
};

// (4) Wait for the paused request, then let it through.
while let Some(event) = events.next().await {
    if event.is_blocked && event.request.url.starts_with("https://example.com/") {
        bidi.network().continue_request(event.request.id).await?;
        break;
    }
}

nav.await.map_err(|e| WebDriverError::FatalError(format!("nav join: {e}")))??;

// (5) Clean up — explicit form, surfaces any error.
intercept.remove().await?;
# Ok(()) }
```

For the response phase, swap the phase enum and call
`continue_response` instead. The shape of the loop is otherwise
identical.

## Adding Your Own Typed Event

Write a `Deserialize` struct matching the wire shape and `impl BidiEvent`:

```rust
use serde::Deserialize;
use thirtyfour::bidi::BidiEvent;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptOpened {
    context: String,
    prompt_type: String,
    message: String,
}

impl BidiEvent for PromptOpened {
    const METHOD: &'static str = "browsingContext.userPromptOpened";
}
```

Then `bidi.subscribe::<PromptOpened>().await?` works the same as for
the curated events. The wire shape and method name for this event are
specified in [`browsingContext.userPromptOpened`][bidi-bc-prompt-opened].

## Lifecycle

A `BiDi` handle is `Clone` (the underlying transport is `Arc`-shared).
Each typed `subscribe::<E>()` bumps a per-method refcount; when the
last stream for a given event drops, `session.unsubscribe` is sent in
the background on the current tokio runtime. Calling `session.end()`
ends the BiDi session entirely.

[`BiDi::subscribe_raw`]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/struct.BiDi.html#method.subscribe_raw
[`BidiEvent`]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/trait.BidiEvent.html
[`InterceptPhase`]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/modules/network/enum.InterceptPhase.html
[intercept-guard-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/modules/network/struct.InterceptGuard.html
[modules-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/modules/index.html
[events-rustdoc]: https://docs.rs/thirtyfour/latest/thirtyfour/bidi/events/index.html
[bidi-events]: https://w3c.github.io/webdriver-bidi/#events
[bidi-bc-load]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-load
[bidi-bc-prompt-opened]: https://w3c.github.io/webdriver-bidi/#event-browsingContext-userPromptOpened
