# Running the tests for `thirtyfour`

> You only need to run the tests if you plan on contributing to the development of `thirtyfour`.
> If you just want to use the crate in your own project, you can skip this section.

The integration tests use [`WebDriver::managed`](../features/manager.md), so they
auto-download and lifetime-manage their own `chromedriver` / `geckodriver`.
You only need a local browser install — Chrome by default.

```bash
cargo test
```

The first run downloads the matching driver into your system cache
(`~/.cache/thirtyfour/drivers` on Linux, `~/Library/Caches/thirtyfour/drivers`
on macOS); subsequent runs reuse it. Within each test binary, sibling tests
share a single `chromedriver` subprocess via a static `WebDriverManager`
plus an "anchor" session, so launching is a one-time cost per binary.

To run against Firefox instead, set `THIRTYFOUR_BROWSER`:

```bash
THIRTYFOUR_BROWSER=firefox cargo test
```

## Manager-gated test suites

A few test files are gated behind the `manager-tests` cargo feature because
they exercise the manager subsystem itself or run the heavier CDP suites:

```bash
# manager lifecycle smokes (Chrome, Firefox, Edge, Safari)
cargo test -p thirtyfour --features manager-tests --test managed -- --test-threads=1

# typed CDP commands
cargo test -p thirtyfour --features manager-tests --test cdp_typed -- --test-threads=1

# CDP event subscription (also needs cdp-events)
cargo test -p thirtyfour --features manager-tests,cdp-events --test cdp_events -- --test-threads=1

# ElementQuery filter predicates
cargo test -p thirtyfour --features manager-tests --test query_filters -- --test-threads=1
```
