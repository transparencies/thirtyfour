# Feature Flags

- `rustls-tls`: (Default) Use rustls to provide TLS support (via reqwest).
- `native-tls`: Use native TLS (via reqwest).
- `component`: (Default) Enable the `Component` derive macro (via thirtyfour_macros).
- `manager`: (Default) Enable [`WebDriver::managed`](../features/manager.md), which auto-downloads
  and lifetime-manages the appropriate webdriver subprocess. See [Managed WebDriver](../features/manager.md).
