# AI-friendly browser automation

## Purpose

This specification owns the reliability contract for entry-level `thirtyfour`
examples that humans and coding agents are likely to copy. It does not define
the later selector-guidance, AI quickstart, recipe, harness, or debugging work
ordered in [`todo.md`](../todo.md).

## Requirements

- **AI-AUTO-001 (confirmed):** Entry-level examples that manage a local browser
  must use `WebDriver::managed`; an example specifically teaching an external
  Selenium endpoint may use `WebDriver::new`. Every flow must explicitly call
  `driver.quit().await?`.
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
