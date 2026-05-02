# Introduction

Welcome to The Book for `thirtyfour`.

`thirtyfour` is a crate for automating Web Browsers in Rust using the `WebDriver` / `Selenium` ecosystem.
On top of the W3C WebDriver protocol it also provides typed support for two bidirectional control
protocols:

- The [Chrome DevTools Protocol](./cdp/overview.md) — the lower-level inspection and control API
  that Chromium-based browsers expose for debugging, profiling, and automation. Typed commands are
  on by default; optional WebSocket-based event subscription is gated behind the `cdp-events`
  feature.
- [WebDriver BiDi](./bidi/overview.md) — the W3C-standard cross-browser bidirectional protocol,
  available behind the `bidi` feature. It works on both Chromium-based browsers and Firefox.

## Why is it called "thirtyfour" ?

Thirty-four (34) is the atomic number for the Selenium chemical element (Se) ⚛️.

## Features

- All W3C WebDriver V1 and WebElement methods are supported
- Create new browser session directly via WebDriver (e.g. chromedriver)
- Create new browser session via Selenium Standalone or Grid
- Find elements (via all common selectors e.g. Id, Class, CSS, Tag, XPath)
- Send keys to elements, including key-combinations
- Execute Javascript
- Action Chains
- Get and set cookies
- Switch to frame/window/element/alert
- Shadow DOM support
- Alert support
- Capture / Save screenshot of browser or individual element as PNG
- [Chrome DevTools Protocol (CDP)](./cdp/overview.md) — typed commands plus optional WebSocket-based event subscription via the `cdp-events` feature
- [WebDriver BiDi](./bidi/overview.md) — the W3C-standard cross-browser bidirectional protocol, behind the `bidi` feature
- [Powerful query interface](./features/queries.md) (the recommended way to find elements) with explicit waits and various predicates
- [Component](./features/components.md) Wrappers (similar to `Page Object Model`)