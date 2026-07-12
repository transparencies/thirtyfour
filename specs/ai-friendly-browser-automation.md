# AI-friendly browser automation

## Purpose

This specification owns the reliability contract for entry-level `thirtyfour`
examples, selector guidance, the AI/LLM quickstart, and coding-agent guidance
that humans and agents are likely to copy. It does not define the later recipe,
harness, or debugging work ordered in [`todo.md`](../todo.md).

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
