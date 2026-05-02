# CLAUDE.md

Notes for AI agents (and humans) working on this repo.

## Pre-push checklist

Before pushing a branch, run all three checks the `lint` workflow runs.
The workflow blocks merges if any fail:

```bash
cargo fmt --check                                # honors rustfmt.toml (max_width=100, use_small_heuristics=Off)
cargo doc --no-deps --all-features               # rustdoc::all is warn-level — broken intra-doc links matter
cargo clippy --all-features --all-targets        # default lints, no -D warnings (yet) but stay clean
```

In normal flow, just run them as one line:

```bash
cargo fmt && cargo clippy --all-features --all-targets && cargo doc --no-deps --all-features
```

## Tests

- `cargo test -p thirtyfour --lib` — fast unit tests, run on every change.
- `cargo test -p thirtyfour --doc` — doc tests; rarely break, but cheap to run.
- The integration tests under `thirtyfour/tests/*.rs` (other than `managed.rs`)
  use `WebDriver::managed`, so they download and lifetime-manage their own
  `chromedriver` / `geckodriver`. You only need a local browser install
  (Chrome by default; set `THIRTYFOUR_BROWSER=firefox` to switch). Within a
  binary, launches funnel through a single static `WebDriverManager` (see
  `thirtyfour/tests/common.rs`); the cross-binary download cache lives in
  the manager's default `cache_dir`. **Don't add a long-lived "anchor"
  session to keep the driver subprocess alive across tests** — its
  tokio-bound resources (HTTP pool, driver stdio readers) end up pinned to
  the first test's `#[tokio::test]` runtime, and when that runtime drops it
  wedges chromedriver on Windows.
- `thirtyfour/tests/managed.rs` is gated behind the `manager-tests` cargo
  feature and runs in its own `manager-test.yml` workflow. It exercises the
  manager's lifecycle semantics directly. Run locally with:
  ```bash
  cargo test -p thirtyfour --features manager-tests --test managed -- --test-threads=1
  ```
- `thirtyfour/tests/cdp_typed.rs` and `thirtyfour/tests/cdp_events.rs` are
  the CDP integration tests. They run in `cdp-test.yml` and use
  `WebDriver::managed`. Run locally with:
  ```bash
  cargo test -p thirtyfour --features manager-tests --test cdp_typed -- --test-threads=1
  cargo test -p thirtyfour --features manager-tests,cdp-events --test cdp_events -- --test-threads=1
  ```

## Adding new CDP commands or events

When adding to the curated set under `thirtyfour/src/cdp/domains/`, **verify
field names against the live spec** at
<https://chromedevtools.github.io/devtools-protocol/tot/>. CDP has several
wire-shape pitfalls with no compile-time signal:

- **SCREAMING acronyms**: `documentURL`, `baseURL`, `requestURL`, etc. —
  `rename_all = "camelCase"` produces `documentUrl` (lowercase `u`), which
  doesn't match. Override with `#[serde(rename = "documentURL")]`.
- **Command params shape**: chromedriver rejects `"params": null` with
  `invalid argument: params not passed`. Unit-struct commands serialise to
  `null`; the transports coerce to `{}` so this works, but new types should
  serialise to a JSON object directly when feasible.
- **Closed string sets**: use the [`string_enum!`] macro from
  `common/protocol.rs`, not raw `String` fields. The macro generates a
  forward-compat `Unknown(String)` variant for values not yet known.
- **Returns for `{}` results**: use `Empty` (the marker in `cdp/command.rs`),
  not `()` — `()` doesn't deserialize from `{}`.

**Every new typed command and event MUST come with an integration test**
in `thirtyfour/tests/cdp_typed.rs` (or `cdp_events.rs` for events) that
exercises it against a real chromedriver via `WebDriver::managed`. Unit
tests that round-trip a hand-written wire body are guess-against-guess: if
the Rust struct has the wrong shape, a unit test that asserts the same
wrong shape passes. Only a real browser tells you what CDP actually
expects. Wire-shape unit tests are NOT accepted.

[`string_enum!`]: thirtyfour/src/common/protocol.rs
