# Installation

To use the `thirtyfour` crate in your Rust project, you need to add it as a dependency in your `Cargo.toml` file:

    [dependencies]
    thirtyfour = "THIRTYFOUR_CRATE_VERSION"

To automate a web browser, `thirtyfour` needs to communicate with a webdriver
server (`chromedriver`, `geckodriver`, etc.). With the default feature set
this is taken care of for you: the [Managed WebDriver](../features/manager.md)
feature auto-downloads a compatible driver for your locally-installed browser,
caches it, and runs it as a child process for the lifetime of your `WebDriver`
handle. You don't need to download or start anything yourself.

You will still need the corresponding web browser (Chrome, Firefox, Edge, or
Safari on macOS) to be installed in your operating system.

> If you'd rather download and run a webdriver yourself — for example, to point
> `thirtyfour` at a remote Selenium grid, or because you've disabled the
> `manager` feature — see
> [Manual WebDriver Setup](../appendix/manual-webdriver.md) in the Appendix.
