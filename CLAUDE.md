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
