# AI-friendly browser automation

## Purpose

This specification owns the reliability contract for entry-level `thirtyfour`
examples, selector guidance, the AI/LLM quickstart, task-oriented recipes, and
translation guidance, and coding-agent guidance that humans and agents are
likely to copy. It does not define the later harness or debugging work ordered
in [`todo.md`](../todo.md).

## Requirements

- **AI-AUTO-001 (confirmed):** Entry-level examples that manage a local browser
  must use `WebDriver::managed`; an example specifically teaching an external
  Selenium endpoint may use `WebDriver::new`. Every flow must explicitly await
  `driver.quit()` and propagate its error when no earlier flow error takes
  precedence.
- **AI-AUTO-002 (confirmed):** Normal element lookup in entry-level examples
  must use `WebDriver::query` or `WebElement::query`, so lookup polls for the
  required page state instead of making a one-shot request.
- **AI-AUTO-003 (confirmed):** Queries for important user-facing elements must
  include a human-readable description for timeout diagnostics.
- **AI-AUTO-004 (confirmed):** A query must use `single()` when exactly one
  match is part of the example's page contract, and `first()` only when
  first-match behavior is intentional.
- **AI-AUTO-005 (confirmed):** The entry-level documentation must describe
  `find()` and `find_all()` as lower-level, one-shot WebDriver operations rather
  than the normal automation path.
- **AI-AUTO-006 (observed):** The repository-root README is a symlink to the
  crate README, which directs readers to the `tokio_async` runnable example.
- **AI-AUTO-007 (confirmed):** The README, crate-level rustdoc, mdBook
  walkthrough, and matching runnable examples must stay behaviorally aligned.
- **AI-SEL-001 (confirmed):** Documentation must prefer stable, app-owned
  `data-testid` hooks through `By::Testid` when the application under test can
  provide them.
- **AI-SEL-002 (confirmed):** Selector guidance must recommend stable semantic
  CSS selectors when no test ID exists, and XPath only when CSS cannot
  reasonably express the target.
- **AI-SEL-003 (confirmed):** Visible-text matching is appropriate when the
  displayed copy is part of the behavior under test. It must not be presented
  as the default way to identify controls because copy, localization, and
  duplicate labels can make it brittle.
- **AI-SEL-004 (confirmed):** Examples against third-party sites must use
  selectors the real page exposes rather than pretending app-owned test hooks
  exist. The surrounding guidance must distinguish this constraint from the
  preferred practice for applications the reader controls.
- **AI-STYLE-001 (confirmed):** The mdBook must provide one concise checklist
  of reliability rules suitable for pasting into coding-agent instructions.
- **AI-STYLE-002 (confirmed):** The checklist must cover session setup and
  cleanup, query semantics and diagnostics, stable selectors, readiness waits,
  query scoping and components, session concurrency, and protocol-specific
  isolation and feature gates.
- **AI-STYLE-003 (confirmed):** Every major checklist rule must link to the
  deeper documentation that defines its behavior and must not claim that a
  planned API, such as the managed test harness, already exists.
- **AI-STYLE-004 (confirmed):** The checklist must be linked directly from the
  getting-started learning path.
- **AI-QS-001 (confirmed):** The mdBook must provide one compact AI/LLM
  quickstart with minimal dependencies and a starter test using
  `WebDriver::managed`, `query()`, readable descriptions, explicit readiness
  conditions, a user-visible outcome, and explicit session cleanup.
- **AI-QS-002 (confirmed):** The starter test must prefer app-owned test IDs,
  scoped queries, and a cleanup shape that still attempts `quit()` after the
  test body fails. A sample URL that is not runnable must be marked `no_run`.
- **AI-QS-003 (confirmed):** The quickstart must give direct selector,
  readiness, scoping, outcome, cleanup, and concurrency rules, while linking
  to the canonical reliability checklist rather than duplicating it.
- **AI-QS-004 (confirmed):** The quickstart must link to the query, waiting,
  component, manager, CDP, and BiDi guides and be part of the mdBook
  getting-started navigation.
- **AI-REC-001 (confirmed):** The mdBook must provide a dedicated recipes
  section with copyable, task-shaped examples for login, search, HTML modal or
  dialog interaction, iframes, shadow DOM, file uploads, screenshots on
  failure, table or list assertions, browser and driver logs, a simple CDP
  network or cache command, and a basic BiDi event subscription.
- **AI-REC-002 (confirmed):** Every recipe must use `WebDriver::managed` unless
  an external WebDriver service is essential, use polling queries and element
  waits for readiness, describe important queries, prefer stable selectors,
  and explicitly attempt session cleanup.
- **AI-REC-003 (confirmed):** Each recipe must be short enough to copy and
  adapt, use cardinality that matches the intended page contract, and assert a
  user-visible or protocol-visible outcome instead of treating a successful
  click as sufficient evidence.
- **AI-REC-004 (confirmed):** Recipes that depend on an application, local
  browser, filesystem fixture, or optional protocol feature must be marked
  `no_run` and state the required setup or feature gate. Recipes must not use a
  fixed sleep for page readiness.
- **AI-REC-005 (confirmed):** Protocol-specific recipes must keep portable
  WebDriver flow separate from Chromium-only CDP and opt-in BiDi behavior, and
  link to the deeper owning guides rather than duplicating their full API
  references.
- **AI-TRANS-001 (confirmed):** The mdBook must map Selenium element lookup,
  explicit waits, page objects, Grid setup, and local browser setup to the
  corresponding `thirtyfour` query, waiter, Component, remote-session, and
  managed-session APIs.
- **AI-TRANS-002 (confirmed):** The mdBook must map Playwright locators,
  auto-waiting, and browser-context habits to scoped queries, explicit
  readiness conditions, and isolated WebDriver sessions without implying that
  a cloned `WebDriver` creates a separate browser context.
- **AI-TRANS-003 (confirmed):** The translation guide must explicitly name
  common Selenium and Playwright APIs that `thirtyfour` does not provide and
  show side-by-side source and Rust examples where the mapping benefits from
  code.
- **AI-TRANS-004 (confirmed):** The guide must link to the canonical query,
  waiting, Component, manager, manual WebDriver, and reliable-test guidance and
  be discoverable from the mdBook navigation and AI quickstart.
- **AI-COMP-001 (confirmed):** When the `component` feature is enabled, the
  prelude must export the `Component` derive/trait name and `ElementResolver`
  so the recommended `use thirtyfour::prelude::*;` import supports the common
  Component path.
- **AI-COMP-002 (confirmed):** The `resolve!` and `resolve_present!` helper
  macros must remain explicit crate-root imports. They are optional shorthand,
  and adding generic macro names to the wildcard prelude would introduce
  unnecessary collision risk.
- **AI-COMP-003 (confirmed):** Component rustdoc, mdBook guidance, examples,
  and tests must use the preferred import style and describe its `component`
  feature gate consistently.

## Acceptance criteria

- **AC-001:** The crate README, crate-level rustdoc, mdBook first-code page, and
  matching runnable examples contain no `find()` call in their Wikipedia user
  flow. Covers AI-AUTO-002.
- **AC-002:** The search form, search input, search button, and, where present,
  resulting article heading queries have readable descriptions and enforce
  their intended uniqueness. Covers AI-AUTO-003 and AI-AUTO-004.
- **AC-003:** The mdBook explanation teaches the query-based flow and explicitly
  preserves `find()` / `find_all()` as deliberate one-shot APIs. Covers
  AI-AUTO-002 and AI-AUTO-005.
- **AC-004:** The updated runnable examples compile with their existing required
  feature sets, and repository formatting, library tests, doc tests, clippy,
  and rustdoc validation pass. Covers AI-AUTO-001 through AI-AUTO-007.
- **AC-005:** The README, crate-level rustdoc, and getting-started path show
  `By::Testid` and explain the stable-selector priority without making the live
  Wikipedia flow inaccurate. Covers AI-SEL-001, AI-SEL-002, and AI-SEL-004.
- **AC-006:** The query documentation explains when text matching is meaningful
  versus brittle, shows the CSS escape hatch for custom test attributes, and
  reserves XPath for targets CSS cannot express. Covers AI-SEL-002 and
  AI-SEL-003.
- **AC-007:** The component documentation contains a concrete `testid` resolver
  example. Covers AI-SEL-001.
- **AC-008:** A standalone mdBook page contains a copyable checklist covering
  all AI-STYLE-002 topics without duplicating the deeper guides. Covers
  AI-STYLE-001 through AI-STYLE-003.
- **AC-009:** The mdBook summary and first-code page link to the checklist.
  Covers AI-STYLE-004.
- **AC-010:** A standalone AI/LLM quickstart contains the minimal dependency
  block and compile-checked or `no_run` starter test required by AI-QS-001 and
  AI-QS-002.
- **AC-011:** The starter test waits for an interactable control, asserts a
  user-visible result without a fixed sleep, and attempts explicit cleanup on
  both success and failure. Covers AI-QS-001 and AI-QS-002.
- **AC-012:** The quickstart contains concise prescriptive rules and links to
  the canonical review checklist plus every deeper guide named by AI-QS-004.
- **AC-013:** The mdBook summary links the quickstart from Getting Started.
  Covers AI-QS-004.
- **AC-014:** A Recipes section in the mdBook navigation contains all eleven
  task categories required by AI-REC-001. The examples are complete,
  independently copyable tests or programs, or clearly identify shared setup.
- **AC-015:** Every recipe is compile-checked or marked `no_run`, contains no
  fixed sleep, uses the recommended selector/query/wait shape where elements
  are involved, and explicitly attempts `driver.quit()` after its task result.
  Covers AI-REC-002 through AI-REC-004.
- **AC-016:** The logging, CDP, and BiDi recipes identify browser limitations,
  feature flags, and capability opt-ins and link to their deeper documentation.
  Covers AI-REC-005.
- **AC-017:** A standalone translation guide covers every Selenium and
  Playwright concept in AI-TRANS-001 and AI-TRANS-002 and uses `thirtyfour`
  APIs that exist in the current tree.
- **AC-018:** The guide includes source-to-Rust examples for lookup, waits,
  scoped targets, Components, and local or remote sessions, plus a dedicated
  table of common nonexistent APIs. Covers AI-TRANS-003.
- **AC-019:** The mdBook summary and AI quickstart link to the translation
  guide, which links onward to each deeper guide in AI-TRANS-004.
- **AC-020:** With default features, Component examples compile with
  `thirtyfour::prelude::*` supplying `Component` and `ElementResolver`, while
  resolver macros use explicit root imports. Crate rustdoc, mdBook Component
  guidance, the playground example, and Component integration tests show the
  same import contract. Covers AI-COMP-001 through AI-COMP-003.
