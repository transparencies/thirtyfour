# Introduction

Welcome to The Book for `thirtyfour`.

`thirtyfour` is a crate for automating Web Browsers in Rust using the `WebDriver` / `Selenium` ecosystem.
It also provides typed support for the `Chrome DevTools Protocol` — the lower-level inspection and
control API that Chromium-based browsers expose for debugging, profiling, and automation — including
optional WebSocket-based event subscription behind the `cdp-events` feature.

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
- Chrome DevTools Protocol (CDP) — typed commands plus optional WebSocket-based event subscription via the `cdp-events` feature
- [Powerful query interface](./features/queries.md) (the recommended way to find elements) with explicit waits and various predicates
- [Component](./features/components.md) Wrappers (similar to `Page Object Model`)