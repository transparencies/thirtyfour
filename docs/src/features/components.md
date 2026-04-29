# Components

When you automate a real web app, the same selectors and helper
methods tend to show up over and over: "find the search bar," "click
the submit button," "read the cart count." Without structure, that
logic spreads across your code and gets brittle. **Components** let
you wrap a piece of UI — a single button, a form, a whole page — in
a Rust struct, attach methods to it, and then reuse it anywhere.

This is the same idea as the
[Page Object Model](https://www.selenium.dev/documentation/test_practices/encouraged/page_object_models/)
from other Selenium ecosystems. In `thirtyfour` it's just a derive
macro on a struct, and any DOM node — not only "pages" — can be a
Component.

## A Quick Example

Suppose your page contains a search form:

```html
<form id="search-form">
    <input type="text" id="search-input" />
    <button type="submit">Search</button>
</form>
```

Wrap it in a Component:

```rust
use thirtyfour::prelude::*;
use thirtyfour::components::ElementResolver;

#[derive(Debug, Clone, Component)]
pub struct SearchForm {
    base: WebElement,                              // The <form> itself.
    #[by(id = "search-input")]
    input: ElementResolver<WebElement>,            // The <input>.
    #[by(css = "button[type='submit']")]
    submit: ElementResolver<WebElement>,           // The <button>.
}

impl SearchForm {
    pub async fn search(&self, term: &str) -> WebDriverResult<()> {
        self.input.resolve().await?.send_keys(term).await?;
        self.submit.resolve().await?.click().await?;
        Ok(())
    }
}
```

Find the `<form>` and turn it into a `SearchForm`:

```rust
let form_el = driver.query(By::Id("search-form")).single().await?;
let form: SearchForm = form_el.into();         // From<WebElement> is derived.
form.search("Selenium").await?;
```

A few things are happening here:

- The `base: WebElement` field is mandatory — it holds the outer
  element the component wraps. The derive macro implements
  `From<WebElement>` for you, so any `WebElement` becomes a
  `SearchForm` with `.into()`.
- Each `#[by(...)]` field is an `ElementResolver`. It doesn't query
  anything until you call `.resolve()`, and it caches the result so
  subsequent calls don't hit WebDriver again.
- Resolvers always search relative to `base`, so a `SearchForm` can
  only ever find elements inside its own `<form>`. That scoping is one
  of the big wins over scattered `driver.query(...)` calls — you can't
  accidentally match elements from a different form on the page.

## The `#[by(...)]` Attribute

Every resolver field needs a `#[by(...)]` attribute. The first part
picks the selector:

| Attribute            | Selector used    |
| -------------------- | ---------------- |
| `id = "..."`         | `By::Id`         |
| `css = "..."`        | `By::Css`        |
| `xpath = "..."`      | `By::XPath`      |
| `tag = "..."`        | `By::Tag`        |
| `class = "..."`      | `By::ClassName`  |
| `name = "..."`       | `By::Name`       |
| `link = "..."`       | `By::LinkText`   |
| `testid = "..."`     | `By::Testid`     |

Pair the selector with extra options, comma-separated. Which options
apply depends on whether the resolver is single or multi (the macro
infers that from the field type — `ElementResolver<T>` is single,
`ElementResolver<Vec<T>>` is multi):

| Option                     | Applies to | Effect                                                    |
| -------------------------- | ---------- | --------------------------------------------------------- |
| `single`                   | single     | Match exactly one element. Errors if 0 or 2+. Default.    |
| `first`                    | single     | Match the first element instead.                          |
| `not_empty`                | multi      | Match at least one element. Errors if empty. Default.     |
| `allow_empty`              | multi      | Match zero or more elements. Empty Vec is OK.             |
| `description = "..."`      | both       | Attach a label that shows up in timeout error messages.   |
| `wait(timeout_ms = N, interval_ms = N)` | both | Override the poll cadence for this field.            |
| `nowait`                   | both       | Try once without polling.                                 |
| `ignore_errors`            | both       | Forward `ignore_errors` to the underlying query.          |
| `multi`                    | multi      | Force multi-resolver behaviour. Only needed for custom type aliases. |
| `custom = my_resolver_fn`  | both       | Use a custom resolver function (see [Custom Resolvers](#custom-resolvers)). Mutually exclusive with the other options. |

Examples:

```rust
// Use the first match if there are several.
#[by(id = "search-input", first)]
input: ElementResolver<WebElement>,

// All <li> elements; empty list is fine.
#[by(tag = "li", allow_empty)]
items: ElementResolver<Vec<WebElement>>,

// Wait up to 60 seconds, polling every second, with a friendly description.
#[by(css = ".loading-spinner", description = "loading spinner",
     wait(timeout_ms = 60_000, interval_ms = 1_000))]
spinner: ElementResolver<WebElement>,
```

## ElementResolver Methods

Every resolver field exposes the same handful of methods:

| Method                       | Behaviour                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------ |
| `.resolve().await?`          | Run the query (or return the cached value) and return the result.                    |
| `.resolve_present().await?`  | Like `.resolve()`, but if the cached value is stale (detached from the DOM), re-query first. Use this when the page may have re-rendered. |
| `.resolve_force().await?`    | Drop the cache and re-query unconditionally.                                         |
| `.invalidate()`              | Drop the cache without querying. The next `.resolve()` will run the query again.     |
| `.validate().await?`         | Return the cached value if it's still in the DOM, or `None`.                         |

Two macros wrap the most common calls:

```rust
let elem = resolve!(self.input);            // self.input.resolve().await?
let elem = resolve_present!(self.input);    // self.input.resolve_present().await?
```

The macros are useful for chained method calls without scattering
`.await?` around:

```rust
resolve!(self.submit).click().await?;
```

## Caching And Staleness

Resolvers cache the resolved value, which is what makes them cheap to
call repeatedly inside helper methods. If the page changes underneath
you — a re-render, a SPA route change, a click that swapped the DOM —
the cached `WebElement` may go stale. Two strategies:

- Use `resolve_present!(field)` instead of `resolve!(field)`. It checks
  whether the cached value is still attached to the DOM and re-queries
  if not.
- Call `.invalidate()` (or `.resolve_force()`) when you *know* the
  underlying DOM has moved.

For elements that change frequently, `resolve_present` is the safe
default. For elements that are stable for the lifetime of the
component, plain `resolve` is faster.

## Nested Components

`ElementResolver<T>` works whenever `T` implements `Component`, so a
Component can contain other Components:

```rust
#[derive(Debug, Clone, Component)]
pub struct CheckboxOption {
    base: WebElement,                              // The <label>.
    #[by(css = "input[type='checkbox']")]
    input: ElementResolver<WebElement>,
}

impl CheckboxOption {
    pub async fn is_ticked(&self) -> WebDriverResult<bool> {
        let input = resolve!(self.input);
        Ok(input.prop("checked").await?.unwrap_or_default() == "true")
    }

    pub async fn tick(&self) -> WebDriverResult<()> {
        let input = resolve_present!(self.input);
        if input.is_clickable().await? && !self.is_ticked().await? {
            input.click().await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Component)]
pub struct CheckboxSection {
    base: WebElement,
    #[by(tag = "label", allow_empty)]
    options: ElementResolver<Vec<CheckboxOption>>,
}
```

When the resolver's element type is itself a Component, the derive
calls `From<WebElement>` on each match, so you get a
`Vec<CheckboxOption>` back — already wrapped, ready to call methods
on:

```rust
let section_el = driver.query(By::Id("checkbox-section")).single().await?;
let section: CheckboxSection = section_el.into();

for option in section.options.resolve().await? {
    option.tick().await?;
}
```

## Non-Element Fields

Fields without a `#[by(...)]` attribute are initialised via
`Default::default()`, so you can tack on bookkeeping state without
extra boilerplate:

```rust
#[derive(Debug, Clone, Component, Default)]
pub struct LoginForm {
    base: WebElement,
    #[by(id = "username")]
    username: ElementResolver<WebElement>,
    attempt_count: u32,                            // Initialised to 0.
}
```

## Custom Resolvers

If a built-in selector can't express what you need — say, "find the
button whose text starts with 'Run'" — write the resolver yourself
and reference it from `custom = ...`:

```rust
use thirtyfour::stringmatch::StringMatchable;

async fn resolve_run_button(elem: WebElement) -> WebDriverResult<WebElement> {
    elem.query(By::Tag("button"))
        .with_text("Run".match_partial().case_insensitive())
        .first()
        .await
}

#[derive(Debug, Clone, Component)]
pub struct Header {
    base: WebElement,
    #[by(custom = resolve_run_button)]
    run_button: ElementResolver<WebElement>,
}
```

A custom resolver receives the component's base element and returns a
`WebDriverResult<T>` matching the field's type. `custom = ...` is
mutually exclusive with the selector and modifier options — the
function is doing all of that work itself.

## When To Reach For A Component

Any time you're going to interact with the same UI element in more
than one place, wrapping it in a Component is usually worth it. You
get:

- A single place to update if the underlying selectors change.
- Scoped queries that can't accidentally match unrelated elements.
- Methods named after what the user does ("`form.search(...)`,
  `option.tick()`") instead of low-level click/type plumbing.
- Cheap repeated access via the resolver cache.

For a one-off lookup deep in a single test, a plain `driver.query(...)`
is fine. Once a piece of UI shows up in two or three places, lift it
into a Component.

## API Reference

For the full list of methods and attribute combinations, see:

- [`thirtyfour::components`](https://docs.rs/thirtyfour/latest/thirtyfour/components/index.html) —
  the `Component` trait, `ElementResolver`, and helper wrappers.
- [`thirtyfour_macros::Component`](https://docs.rs/thirtyfour-macros/latest/thirtyfour_macros/derive.Component.html) —
  the derive macro and every supported attribute.
