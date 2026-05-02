# Feature Flags

- `rustls-tls`: (Default) Use rustls to provide TLS support (via reqwest).
- `native-tls`: Use native TLS (via reqwest).
- `component`: (Default) Enable the `Component` derive macro (via thirtyfour_macros).
- `manager`: (Default) Automatic webdriver download and process management;
  see [WebDriver Manager](../features/manager.md). Disable this if you'd
  rather manage the webdriver yourself — see
  [Manual WebDriver Setup](../appendix/manual-webdriver.md).
- `cdp`: (Default) Typed Chrome DevTools Protocol commands and the
  `WebDriver::cdp()` / `WebElement::cdp_*()` integrations. Works on
  any Chromium-based session via the WebDriver vendor endpoint
  `goog/cdp/execute` — no extra connection required. See the
  [CDP overview](../cdp/overview.md).
- `cdp-events`: WebSocket-based CDP session with event subscription.
  Adds `Cdp::connect()` and `CdpSession`; pulls in `tokio-tungstenite`.
  Off by default. See [CDP Events](../cdp/events.md).
- `bidi`: WebDriver BiDi (W3C) — typed commands and event
  subscription over a WebSocket negotiated via the `webSocketUrl: true`
  capability. Off by default; pulls in `tokio-tungstenite`. See the
  [BiDi overview](../bidi/overview.md).
