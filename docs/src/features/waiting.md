# Waiting For Element Changes

`ElementQuery` waits for an element to *appear*. `ElementWaiter` waits
for an element you already have to reach a particular state — visible,
clickable, gone, with certain text, etc. Reach for it whenever you've
clicked something and need the page to settle before you continue.

```rust
let button = driver.query(By::Css(".save")).single().await?;
button.click().await?;
button.wait_until().not_displayed().await?;
```

`wait_until()` is available on every `WebElement`. It returns an
`ElementWaiter` that polls the element until either a predicate
matches or the timeout elapses.

## Built-In Predicates

State predicates polled directly via WebDriver:

| Method                      | Waits until the element is...                |
| --------------------------- | -------------------------------------------- |
| `.displayed().await?`       | rendered (`isDisplayed` returns `true`)      |
| `.not_displayed().await?`   | hidden                                       |
| `.enabled().await?`         | not disabled                                 |
| `.not_enabled().await?`     | disabled                                     |
| `.selected().await?`        | selected (checkboxes, options, radios)       |
| `.not_selected().await?`    | deselected                                   |
| `.clickable().await?`       | both displayed and enabled                   |
| `.not_clickable().await?`   | hidden or disabled                           |
| `.stale().await?`           | detached from the DOM                        |

`.stale()` is especially useful right after a click — it lets you wait
for the element you just acted on to disappear before assuming the
next page is loaded.

## Text, Class, Attribute, Property Waits

Each of these takes a `Needle` (from the
[`stringmatch`](https://crates.io/crates/stringmatch) crate) — a plain
`&str` for exact match, or a `StringMatch` for partial /
case-insensitive / word-boundary matches.

| Method                           | Waits until...                          |
| -------------------------------- | --------------------------------------- |
| `.has_text(needle)`              | the element's text matches              |
| `.lacks_text(needle)`            | the element's text does *not* match     |
| `.has_class("name")`             | the element's class list contains it    |
| `.lacks_class("name")`           | the class is no longer present          |
| `.has_value(needle)`             | the input's value matches               |
| `.lacks_value(needle)`           | the input's value no longer matches     |
| `.has_attribute(name, needle)`   | a single attribute matches              |
| `.lacks_attribute(name, needle)` | a single attribute no longer matches    |
| `.has_attributes([...])`         | several attributes match together       |
| `.lacks_attributes([...])`       | none of those attributes match          |
| `.has_property(name, needle)`    | a JS property matches                   |
| `.lacks_property(name, needle)`  | a JS property does not match            |
| `.has_properties([...])`         | several properties match together       |
| `.lacks_properties([...])`       | none of those properties match          |
| `.has_css_property(name, needle)`   | a computed CSS property matches      |
| `.lacks_css_property(name, needle)` | a computed CSS property does not match |
| `.has_css_properties([...])`     | several CSS properties match together   |
| `.lacks_css_properties([...])`   | none of those CSS properties match      |

```rust
use thirtyfour::stringmatch::StringMatchable;

elem.wait_until()
    .has_text("Order received".match_partial().case_insensitive())
    .await?;
```

## Custom Timeouts And Error Messages

Override the poll cadence on a single wait:

```rust
use std::time::Duration;

elem.wait_until()
    .wait(Duration::from_secs(60), Duration::from_secs(1))
    .clickable()
    .await?;
```

Attach a custom error message so a timeout reads in plain English:

```rust
elem.wait_until()
    .error("Timed out waiting for the spinner to disappear")
    .stale()
    .await?;
```

## Custom Predicates

For anything the built-ins don't cover, pass your own predicate. It
gets a `&WebElement` and returns `WebDriverResult<bool>`:

```rust
elem.wait_until()
    .condition(|elem| async move {
        let value = elem.value().await?.unwrap_or_default();
        Ok(value.parse::<u32>().map_or(false, |n| n > 100))
    })
    .await?;
```

Pre-built predicate constructors live in the
[`thirtyfour::extensions::query::conditions`](https://docs.rs/thirtyfour/latest/thirtyfour/extensions/query/conditions/index.html)
module. They share the same shape, so you can compose several into
one wait:

```rust
use thirtyfour::extensions::query::conditions;

elem.wait_until()
    .conditions(vec![
        conditions::element_is_displayed(true),
        conditions::element_is_clickable(true),
    ])
    .await?;
```

The `conditions` module is also useful as a source of filter functions
for `ElementQuery::with_filter()`.

## When To Reach For Which

- **Looking for an element on the page?** Use [`ElementQuery`](./queries.md).
- **Already have an element and waiting for it to change?** Use
  `ElementWaiter` (this chapter).
- **Waiting for an element to disappear?** Either works.
  `query(...).not_exists()` polls until the selector returns nothing;
  `elem.wait_until().stale()` polls until *that specific element* is
  detached.

## API Reference

For the full method list, see
[`ElementWaiter`](https://docs.rs/thirtyfour/latest/thirtyfour/extensions/query/struct.ElementWaiter.html)
on docs.rs.
