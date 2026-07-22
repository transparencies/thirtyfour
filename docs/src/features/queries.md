# Element Queries

To find elements on a page, call `.query(...)` on a `WebDriver` or a
`WebElement`. `query()` is the recommended way to locate elements:
it knows how to wait for the element to appear, can describe what
you were looking for in error messages, and lets you chain filters
and alternatives until the query returns exactly what you want.

```rust
let elem = driver.query(By::Id("search-form")).single().await?;
```

That's the basic shape. The rest of this chapter unpacks each piece —
the selectors you can pass, the filters you can chain, and the
terminator at the end that decides what comes back.

## How It Works

A query has three parts:

1. A **starting selector** (`By::Id("search-form")`).
2. Optional **filters and chained alternatives** (e.g. `.with_text("Hello")`,
   `.or(By::Css("..."))`).
3. A **terminator** that decides what to return: a single element, all
   matches, just a boolean for existence, etc.

The query polls under the hood. By default it tries every 500ms for up
to 20 seconds; if the selector matches at any point, the query returns
immediately. If the timeout elapses with no match, you get a structured
error that includes the selector(s) you used and any descriptions you
attached.

`WebElement::query()` works the same way and scopes the search to the
element's subtree.

## Selectors

Pass any of these `By` variants to `query()`:

| Selector              | Matches                                  |
| --------------------- | ---------------------------------------- |
| `By::Id("foo")`       | Element with `id="foo"`                  |
| `By::Css("...")`      | CSS selector                             |
| `By::XPath("...")`    | XPath expression                         |
| `By::Tag("button")`   | Element by tag name                      |
| `By::ClassName("x")`  | Element with class `x`                   |
| `By::Name("user")`    | Element with `name="user"`               |
| `By::LinkText("...")` | `<a>` whose visible text matches exactly |
| `By::PartialLinkText("...")` | `<a>` whose visible text contains the string |
| `By::Testid("...")`   | Element with `data-testid="..."`         |

`By::Css` can express compound CSS selectors, while `By::XPath` covers
relationships CSS cannot represent. The other variants are convenient,
purpose-specific selectors, and most are implemented as CSS under the hood.

## Choosing Stable Selectors

Use selectors in this order when the application gives you the choice:

1. **App-owned test IDs.** Add a stable `data-testid` to important controls and
   select it with `By::Testid`. This keeps tests independent of styling and
   displayed copy.
2. **Stable semantic CSS.** When no test ID exists, target durable attributes
   and element roles, such as `button[type='submit']`, rather than generated
   classes or a long chain of DOM positions.
3. **Visible text when the copy is the behavior.** Text matching is useful for
   verifying an error message, heading, or labeled action. It is brittle as a
   default element identity because copy changes, localization, and duplicate
   labels can break an otherwise-correct flow.
4. **XPath only when CSS cannot express the target.** XPath is useful for
   relationships or document structures CSS cannot select, but is usually
   harder to read and easier to couple to the current DOM.

For an app-owned test hook:

```rust
let save_button = driver
    .query(By::Testid("settings-save"))
    .desc("settings save button")
    .single()
    .await?;
```

Use a strong selector before a text filter when the visible wording also
matters:

```rust
let status = driver
    .query(By::Testid("settings-status"))
    .with_text("Saved")
    .desc("saved settings status")
    .single()
    .await?;
```

`By::Testid` targets the conventional `data-testid` attribute. Teams that use
another app-owned attribute can express it with CSS, for example
`By::Css("[data-qa='settings-save']")`.

## Picking An Element

The terminator at the end of the chain decides what comes back. Pick
the one that matches what you actually need:

| Terminator                              | Returns                                                |
| --------------------------------------- | ------------------------------------------------------ |
| `.first().await?`                       | The first matching element. Errors if none.            |
| `.single().await?`                      | The matching element. Errors if 0 or 2+.               |
| `.first_opt().await?`                   | `Option<WebElement>` — `None` if none match.           |
| `.all_from_selector().await?`           | Elements from the first branch that matched.           |
| `.all_from_selector_required().await?`  | Same, but errors if empty.                             |
| `.any().await?`                         | Elements from every branch combined, possibly empty.   |
| `.any_required().await?`                | Same, but errors if empty.                             |
| `.exists().await?`                      | Waits for a match; returns `false` on timeout.          |
| `.not_exists().await?`                  | Waits for no matches; returns `false` on timeout.       |
| `.wait_until_gone().await?`             | Waits for no matches; timeout is an error.              |

`.any()` and `.all_from_selector()` differ when you've used `.or()`:
`.any()` runs every branch and returns the union of matches;
`.all_from_selector()` short-circuits on the first branch that
finds something.

The semantic difference between `single()` and `first()` is worth
calling out: `single()` is a contract that there should be exactly one
match. If two elements appear it returns an error rather than silently
picking one — useful for catching a sloppy selector.

## Multiple Selectors With `.or()`

Chain `.or()` to try multiple selectors in parallel. The first branch
that matches wins:

```rust
let elem = driver
    .query(By::Css(".legacy-button"))
    .or(By::Css(".new-button"))
    .first()
    .await?;
```

Each branch is checked once per poll iteration, so a slow page that
serves either layout will resolve as soon as one appears.

## Filters

Narrow a branch with chained filters. State filters short-circuit on
the WebDriver side (cheap):

```rust
let button = driver
    .query(By::Css("button.submit"))
    .and_displayed()
    .and_enabled()
    .and_clickable()
    .first()
    .await?;
```

Negative variants are also available: `.and_not_displayed()`,
`.and_not_enabled()`, `.and_not_selected()`, `.and_not_clickable()`.

Attribute, property, text, and class filters take a `Needle` — any
type that implements the [`stringmatch`](https://crates.io/crates/stringmatch)
crate's matching trait. A plain `&str` is exact-match; use `StringMatch`
for partial / case-insensitive / word-boundary matches:

```rust
use thirtyfour::stringmatch::StringMatchable;

let btn = driver
    .query(By::Testid("account-submit"))
    .with_text("Submit".match_partial().case_insensitive())
    .first()
    .await?;
```

Available filter families (each has a `with_*` and a `without_*` form):

- **Text:** `.with_text(needle)` — visible text content
- **Class:** `.with_class("name")` — `class` attribute contains `name`
- **Tag:** `.with_tag("button")`
- **Id:** `.with_id("submit")`
- **Value:** `.with_value(needle)` — for inputs
- **Attribute(s):** `.with_attribute("data-state", "ready")`,
  `.with_attributes([(name, needle), ...])`
- **Property(ies):** `.with_property(name, needle)`,
  `.with_properties(...)`
- **CSS property(ies):** `.with_css_property("color", "rgb(0, 0, 0)")`,
  `.with_css_properties(...)`

Each filter triggers an extra WebDriver round trip per poll iteration,
so prefer narrowing the initial `By` selector when you can. Start with an
app-owned test ID or stable CSS selector, add filters only for behavior the
test needs to verify, and reserve XPath for targets CSS cannot express.

## Custom Predicates

When a built-in filter isn't enough, supply your own:

```rust
let chosen = driver
    .query(By::Tag("li"))
    .with_filter(|elem| async move {
        Ok(elem.text().await?.starts_with("Status:"))
    })
    .first()
    .await?;
```

A predicate is any async function returning `WebDriverResult<bool>` for
a given `&WebElement`.

## Timeouts And Polling

Override the poll cadence on a single query:

```rust
use std::time::Duration;

let slow = driver
    .query(By::Id("late-loader"))
    .wait(Duration::from_secs(60), Duration::from_secs(1))
    .single()
    .await?;
```

Use `.nowait()` to opt out of polling entirely (one attempt, return
immediately):

```rust
let exists = driver.query(By::Id("maybe")).nowait().exists().await?;
let absent = driver.query(By::Id("maybe")).nowait().not_exists().await?;
```

Each branch is attempted at most once with `.nowait()`. `exists()` may stop at
the first matching branch; `not_exists()` must check every branch because all
of them must be absent. Without `.nowait()`, `not_exists()` polls until the
query has no matches and returns `false` if matches remain when polling ends.

When disappearance is required rather than optional, use the error-returning
form:

```rust
driver
    .query(By::Testid("saving-indicator"))
    .desc("saving indicator")
    .wait_until_gone()
    .await?;
```

“Gone” means no element currently matches any branch after that branch's
filters in the query's source context. A replacement node that still matches
keeps the query waiting. This returns immediately when the query is already
absent and returns a `WebDriverError::Timeout` if any branch still matches at
the deadline.

The default poller is 20 seconds with 500ms intervals. To change it for
the whole `WebDriver`, supply a custom `WebDriverConfig` — see
[`ElementPollerWithTimeout`] in the API docs.

## Better Error Messages

Attach a human-readable description so timeout errors say what you were
looking for:

```rust
let cart = driver
    .query(By::Css("[data-cart-count]"))
    .desc("shopping cart badge")
    .first()
    .await?;
```

If the query times out, the error includes `"shopping cart badge"`
instead of just the raw CSS selector.

## A Note On `find()` / `find_all()`

You may run across `find()` and `find_all()` methods on `WebDriver`
and `WebElement`. They exist to mirror the W3C WebDriver
specification — a one-shot lookup with no polling, no filters, and a
thin error if nothing matches. They're fine for the rare case where
you genuinely want exactly that, but for everyday automation prefer
`query()`: it handles slow loads, missing elements, and flickering
DOMs more gracefully and gives better diagnostics when something
goes wrong.

## API Reference

For the full method list and per-method semantics, see
[`ElementQuery`](https://docs.rs/thirtyfour/latest/thirtyfour/extensions/query/struct.ElementQuery.html)
on docs.rs.
