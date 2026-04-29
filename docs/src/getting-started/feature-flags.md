# Feature Flags

- `rustls-tls`: (Default) Use rustls to provide TLS support (via reqwest).
- `native-tls`: Use native TLS (via reqwest).
- `component`: (Default) Enable the `Component` derive macro (via thirtyfour_macros).
- `manager`: (Default) Automatic webdriver download and process management.
  Disable this if you'd rather manage the webdriver yourself — see
  [Manual WebDriver Setup](../appendix/manual-webdriver.md).
