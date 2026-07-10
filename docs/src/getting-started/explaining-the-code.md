# Understanding Web Browser Automation

So what did this code actually do?

> **NOTE:** This section goes much deeper into how web browser automation works.
> Feel free to skip ahead and come back when you're ready to dig deeper.

## How Thirtyfour Works

Well, the first thing to know is that `thirtyfour` doesn't talk directly to the Web Browser.
It simply fires off commands to the webdriver server as HTTP Requests.
The webdriver then talks to the Web Browser and tells it to execute each command, and then
returns the response from the Web Browser to `thirtyfour`. As long as the webdriver is
running, `thirtyfour` can do just about anything a human can do in a web browser.

## Explaining The Code

So let's go through the code and see what is going on.

```rust
    let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;
```

This single line does a lot of work for you:

1. Picks a `chromedriver` version that matches your installed Chrome (downloading and caching the binary the first time, reusing it after).
2. Spawns it as a child process on a free local port and waits for it to be ready.
3. Connects to it and starts a new browser session.

The session opens the browser in a new profile (so it won't add anything to your
history etc.) and navigates to the default start page. When the last `WebDriver`
handle drops, the browser closes and the driver subprocess is torn down with it.

See [WebDriver Manager](../features/manager.md) for everything else the manager
can do — pinning a specific driver version, sharing one manager across many
sessions, supplying a pre-installed driver binary, and observing what's
happening as the manager works.

> If you'd rather run the webdriver yourself — for example to point `thirtyfour` at a
> remote Selenium grid — see [Manual WebDriver Setup](../appendix/manual-webdriver.md)
> in the Appendix. The remaining sections work the same either way.

## Capabilities

The way we tell it what browser we want is by using `DesiredCapabilities`. In this case,
we construct a new `ChromeCapabilities` instance. Each `*Capabilities` struct will have
additional helper methods for setting options like headless mode, proxy, and so on.
See the [documentation](https://docs.rs/thirtyfour/latest/thirtyfour/common/capabilities/chrome/struct.ChromeCapabilities.html) for more details.

## Element Queries

Next, we tell the browser to navigate to Wikipedia:

```rust
    driver.goto("https://wikipedia.org").await?;
```

And then we look for an element on the page:

```rust
    let elem_form = driver
        .query(By::Id("search-form"))
        .desc("Wikipedia search form")
        .single()
        .await?;
```

We search for elements using what we call a `selector` or `locator`. In this case we are looking for
an element with the id of `search-form`. The `query()` method polls until the element appears, and
`desc()` gives the query a readable name if it times out. Calling `single()` also checks our page
contract: exactly one search form should match this selector.

### Choosing A Stable Selector

When you control the application under test, add stable `data-testid` hooks to
important controls and prefer `By::Testid`:

```rust
let save_button = driver
    .query(By::Testid("settings-save"))
    .desc("settings save button")
    .single()
    .await?;
```

If there is no app-owned test ID, use a stable semantic CSS selector next.
Match visible text when the wording itself is part of the behavior you need to
verify, not as the default identity of a control: copy changes, localization,
and duplicate labels can otherwise break the test. Use XPath only when CSS
cannot reasonably express the target.

Wikipedia does not provide test IDs for this flow, so the walkthrough uses the
stable selectors that the real third-party page exposes.

> If you actually navigate to [https://wikipedia.org](https://wikipedia.org) and open your browser's
> devtools (F12 in most browsers), then go to the `Inspector` tab, you will see the raw HTML for
> the page. In the search box at the top of the inspector, if you type `#search-form`
> (the # is how we specify an id) you will see that it highlights the search form element.

This is the container element that contains both the input field and the button itself.

But we want to type into the field, so we need to do another query:

```rust
    let elem_text = elem_form
        .query(By::Id("searchInput"))
        .desc("Wikipedia search input")
        .single()
        .await?;
```

This query starts from `elem_form`, so it only searches inside the form's subtree. If you search in
the inspector for `#searchInput` you get an `<input />` element, which is the one we want to type
into.

## Typing Text Into An Element

To type into a field, we just tell `thirtyfour` to send the keys we want to type:

```rust
    elem_text.send_keys("selenium").await?;
```

This will literally type the text into the input field.

Now we need to find the search button and click it. Finding the element means doing another
query:

```rust
    let elem_button = elem_form
        .query(By::Css("button[type='submit']"))
        .desc("Wikipedia search button")
        .single()
        .await?;
```

This time we use a `CSS` selector to locate a `<button>` element with an attribute `type` that
has the value `submit`. To learn more about selectors, [click here](https://www.selenium.dev/documentation/webdriver/elements/locators/).

## Clicking The Button

Next, the call to `click()` tells `thirtyfour` to simulate a click on that element:

```rust
    elem_button.click().await?;
```

## Dealing With Page Loading Times

The page now starts loading the result of our search on Wikipedia. This brings us to our first
issue when automating a web browser. How does our code know when the page has finished loading?
If we had a slow internet connection, we might try to find an element on the page only for it
to fail because that element hasn't loaded yet.

**How do we solve this?**

Well, one option is to simply tell our code to wait for a few seconds and then try to find the
element we are looking for. But this is brittle and likely to fail. Don't do this. There are
cases where you are forced to use this approach, but it should be a last resort. Incidentally,
from a website testing perspective, it probably also means a human is going to be unsure of when
the page has actually finished loading, and this is an indicator of a poor user experience.

Instead, we usually want to look for an element on the page and wait until that element appears.

```rust
driver
    .query(By::ClassName("firstHeading"))
    .desc("Wikipedia article heading")
    .single()
    .await?;
assert_eq!(driver.title().await?, "Selenium - Wikipedia");
```

## Better Element Queries

To have `thirtyfour` wait until an element appears, we use the `query()` method, which provides a
builder interface for constructing flexible queries. Again, this uses one of the `By` selectors.
The `single()` terminator expresses that the selector must match exactly one element; use `first()`
instead when choosing the first of several matches is intentional.

By default, `query()` polls every half-second for up to 20 seconds. If no element matches, it times
out and returns an error that includes the selector and any description added with `desc()`. The
polling time and timeout duration can be changed using `WebDriverConfig`, or overridden for a given
query.

The `query()` method is the recommended way to search for elements. The `find()` and `find_all()`
methods are lower-level, one-shot operations shaped like the WebDriver specification. Use them when
one immediate lookup is deliberate. For everyday automation, `query()` adds polling and other
niceties, including more detail when an element was not found.

See the [`ElementQuery`](https://docs.rs/thirtyfour/latest/thirtyfour/extensions/query/struct.ElementQuery.html)
documentation for more details on the kinds of queries it supports.

## Closing The Browser

Finally we need to close the browser window. If we don't, it will remain open after our application
has exited.

```rust
    driver.quit().await?;
```

And this concludes the introduction.

Happy Browser Automation!
