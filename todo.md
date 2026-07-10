# AI-Friendly Browser Automation Todo Order

This file captures the recommended implementation order for the AI-friendly
`thirtyfour` issue batch. Treat it as ordering guidance, not live issue status;
check GitHub before starting each item.

## 1. Foundation Docs

These items establish the source-of-truth patterns that every later doc or API
example should copy.

- [ ] [#329 Make top-level examples model robust query-based automation](https://github.com/stevepryde/thirtyfour/issues/329)
- [ ] [#336 Make By::Testid and stable selector guidance more prominent](https://github.com/stevepryde/thirtyfour/issues/336)
- [ ] [#339 Document style rules for reliable AI-generated thirtyfour tests](https://github.com/stevepryde/thirtyfour/issues/339)

## 2. AI Entrypoints

These should build on the foundation-doc language so AI tools have one compact
place to start.

- [ ] [#330 Add an AI/LLM quickstart for reliable browser automation](https://github.com/stevepryde/thirtyfour/issues/330)
- [ ] [#331 Publish llms.txt and llms-full.txt for AI tool discovery](https://github.com/stevepryde/thirtyfour/issues/331)

## 3. Broader Learning Material

These expand the quickstart into task-shaped material and translation help for
people or agents coming from other browser automation ecosystems.

- [ ] [#332 Add task-oriented browser automation recipes to the book](https://github.com/stevepryde/thirtyfour/issues/332)
- [ ] [#333 Add Selenium and Playwright translation guide for thirtyfour patterns](https://github.com/stevepryde/thirtyfour/issues/333)

## 4. API Ergonomics

These can be designed more confidently once the docs clarify the preferred user
flows and examples.

- [ ] [#338 Consider exporting common component ergonomics from the prelude](https://github.com/stevepryde/thirtyfour/issues/338)
- [ ] [#334 Add a managed test harness helper that guarantees browser cleanup](https://github.com/stevepryde/thirtyfour/issues/334)
- [ ] [#335 Add first-class failure artifact helpers for browser tests](https://github.com/stevepryde/thirtyfour/issues/335)
- [ ] [#337 Explore high-level safe interaction helpers built on query and waits](https://github.com/stevepryde/thirtyfour/issues/337)

## 5. Advanced AI/Debugging Capability

This is useful, but it should follow the simpler docs and ergonomics work so the
snapshot shape serves real debugging workflows rather than guessing too early.

- [ ] [#340 Explore accessibility or simplified DOM snapshot support for AI-assisted automation](https://github.com/stevepryde/thirtyfour/issues/340)

## Suggested First Milestone

Start with this docs-first slice:

- [ ] #329
- [ ] #336
- [ ] #339
- [ ] #330
- [ ] #331

That gives AI tools a better source of truth without waiting on API design.
