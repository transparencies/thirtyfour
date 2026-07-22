# Reliable AI-Generated Tests

Use this checklist when asking a coding agent to write or review `thirtyfour`
automation. It is intentionally short enough to paste into project-level agent
instructions:

```text
- Use WebDriver::managed for local browsers; use WebDriver::new or builder only
  when connecting to a remote Selenium or Grid endpoint.
- Use query() for normal lookup. Use find() only for an intentional one-shot lookup.
- End unique queries with single(), and add desc(...) to important queries.
- Prefer By::Testid or another stable app-owned selector. Avoid generated classes,
  DOM-position chains, and XPath when stable CSS can express the target.
- Never use a fixed sleep for page readiness. Wait with query() or wait_until().
- Scope queries through a container element; use Components for repeated UI areas.
- Prefer run_browser_test(...) in tests; otherwise explicitly call
  driver.quit().await? when the session is finished.
- Do not share one WebDriver session across independent concurrent flows. A shared
  WebDriverManager may launch separate sessions for parallel tests.
- Keep CDP and BiDi code isolated from portable WebDriver flows. Gate optional
  cdp-events and bidi APIs with their Cargo features, and opt BiDi sessions in.
```

## Why These Rules

- [`WebDriver::managed`](../features/manager.md) downloads, launches, and
  lifetime-manages a local driver. Use the normal constructor or
  [`WebDriver::builder`](../appendix/manual-webdriver.md) when an external
  Selenium service owns that lifecycle. Explicit `quit()` reports shutdown
  errors and does not rely on asynchronous cleanup during `Drop`.
- [Element queries](../features/queries.md) poll for page state and provide
  filters, descriptions, and cardinality checks. See the guidance for
  [stable selectors](../features/queries.md#choosing-stable-selectors),
  [choosing `single()` or `first()`](../features/queries.md#picking-an-element),
  [describing important queries](../features/queries.md#better-error-messages),
  and [intentional one-shot lookup](../features/queries.md#a-note-on-find--find_all).
- [Element waits](../features/waiting.md) express the state an existing element
  must reach. A fixed sleep only guesses how long the page needs; a query or wait
  completes as soon as the required state exists and produces a useful timeout
  when it does not.
- Element-scoped queries prevent unrelated matches. For recurring UI areas,
  [Components](../features/components.md) keep selectors private and expose
  user-intent methods while their resolvers handle polling and stale elements.
- A cloned `WebDriver` controls the same browser session, so independent flows
  can race over tabs, navigation, and browser state. The
  [manager's shared configuration](../features/manager.md#sharing-one-manager-across-sessions)
  can instead launch a separate session for each test or worker.
- CDP is Chromium-specific, while BiDi is the cross-browser W3C protocol. Keep
  either behind a small boundary so the rest of the automation remains portable,
  and follow the [feature flag](./feature-flags.md) and
  [BiDi opt-in](../bidi/overview.md#enabling-bidi) requirements. The `cdp` feature
  is enabled by default; `cdp-events` and `bidi` are opt-in.

Treat these as review rules, not merely generation hints: generated code should
not be accepted until its selectors, waits, cleanup, concurrency, and feature
gates satisfy the same checklist.

## Cleanup That Survives Test Failures

Use [`run_browser_test`](https://docs.rs/thirtyfour/latest/thirtyfour/testing/fn.run_browser_test.html)
as the default test shape:

```rust,no_run
use thirtyfour::{
    prelude::*,
    testing::{BrowserTestError, run_browser_test},
};

#[tokio::test]
async fn page_has_heading() -> Result<(), BrowserTestError> {
    run_browser_test(
        WebDriver::managed(DesiredCapabilities::chrome()),
        |driver| async move {
            driver.goto("https://example.com").await?;
            driver.query(By::Css("h1")).single().await?;
            Ok(())
        },
    )
    .await
}
```

The runner keeps its own session handle, passes a clone to the test body, and
awaits `quit()` after success, an early `?` return, or a panicking assertion.
After a panic it resumes the original panic; if cleanup also fails, the cleanup
error is written through `tracing`. `panic = "abort"` cannot run cleanup.
The runner future must be allowed to finish: cancelling or aborting its task can
interrupt asynchronous cleanup.

Error precedence is explicit:

| Test body | `quit()` | Result |
|-----------|----------|--------|
| succeeds | succeeds | body value |
| fails | succeeds | `BrowserTestError::Body` |
| succeeds | fails | `BrowserTestError::Cleanup` |
| fails | fails | `BrowserTestError::BodyAndCleanup` with both errors |

The first argument accepts any future that creates a `WebDriver`, so the same
runner works with `WebDriver::managed(...)`, `WebDriver::builder(...)`, and
`WebDriver::new(...)`.
