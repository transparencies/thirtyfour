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
  require a running `chromedriver` / `geckodriver` on the standard ports
  (9515 / 4444). The `cargo test` workflow in CI starts those automatically;
  locally you'd start them yourself before running.
- `thirtyfour/tests/managed.rs` is gated behind the `manager-tests` cargo
  feature and runs in its own `manager-test.yml` workflow that does *not*
  pre-start drivers (the manager spawns them). Run locally with:
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
<https://chromedevtools.github.io/devtools-protocol/tot/>. CDP uses
`#[serde(rename_all = "camelCase")]` for most fields, but several exceptions
silently break deserialization with no compile-time signal:

- **SCREAMING acronyms**: `documentURL`, `baseURL`, `requestURL`, etc. —
  `rename_all = "camelCase"` produces `documentUrl` (lowercase `u`), which
  doesn't match. Override with `#[serde(rename = "documentURL")]`.
- **Command params shape**: chromedriver rejects `"params": null` with
  `invalid argument: params not passed`. Unit-struct commands serialise to
  `null`; the transports coerce to `{}` so this works, but new types should
  serialise to a JSON object directly when feasible.
- **Type wire-format vs Rust enum form**: enums (e.g. `ConnectionType`) map
  CDP's `cellular2g` → `Cellular2G` via per-variant `#[serde(rename = ...)]`,
  not just `rename_all = "lowercase"`.
- **Returns for `{}` results**: use `Empty` (the marker in `cdp/command.rs`),
  not `()` — `()` doesn't deserialize from `{}`.

Every new command/event must come with a unit test that round-trips the
exact wire shape (see `cdp/tests.rs` and the per-domain `mod tests` blocks
for the pattern). The serde defaults will look correct in Rust until they
silently fail against a real browser otherwise — exactly the class of bug
that's caught at unit-test time, not at integration-test time.
