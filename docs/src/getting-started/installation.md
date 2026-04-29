# Installation

Add `thirtyfour` as a dependency in your `Cargo.toml`:

    [dependencies]
    thirtyfour = "THIRTYFOUR_CRATE_VERSION"

You'll also need the corresponding web browser (Chrome, Firefox, Edge,
or Safari on macOS) installed in your operating system. `thirtyfour`
handles the webdriver itself — it auto-downloads a compatible
`chromedriver` / `geckodriver` / `msedgedriver` for your installed
browser, runs it as a child process, and tears it down with your code.

> Want to run the webdriver yourself instead — for example, to point
> `thirtyfour` at a remote Selenium grid? See
> [Manual WebDriver Setup](../appendix/manual-webdriver.md) in the
> Appendix.
