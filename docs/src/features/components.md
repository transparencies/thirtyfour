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
    <button type="submit" data-testid="search-submit">Search</button>
</form>
```

Wrap it in a Component:

```rust
use thirtyfour::prelude::*;

#[derive(Debug, Clone, Component)]
pub struct SearchForm {
    base: WebElement,                              // The <form> itself.
    #[by(id = "search-input")]
    input: ElementResolver<WebElement>,            // The <input>.
    #[by(testid = "search-submit", description = "search submit button")]
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
- Resolver queries start from `base`, so components normally stay scoped
  to their own subtree. XPath is the exception: an expression beginning
  with `//` is document-rooted. Use `.//` when an XPath resolver must stay
  inside the component.

## Component Shape And Generated API

`Component` can only be derived for a struct with named fields. The struct
must contain exactly one base `WebElement`:

```rust
#[derive(Debug, Clone, Component)]
pub struct Dialog {
    #[base]
    root: WebElement,
}
```

A field named `base` is detected automatically. Use `#[base]` when the
field has another name. Do not use both forms in the same struct; the
derive rejects multiple base fields. The base field type must be
`WebElement` (either imported or written as `thirtyfour::WebElement`).

For each component, the derive generates:

- `pub fn new(base: WebElement) -> Self`;
- `From<WebElement>`, which delegates to `new`;
- the `Component` trait implementation, including `base_element()`.

Fields with `#[by(...)]` become element resolvers. Other fields are
initialised with `Default::default()`, so each such field type must implement
`Default`. A `#[cfg(...)]` attribute on a field is preserved in the generated
constructor, allowing feature- or target-specific resolver and state fields.

## The `#[by(...)]` Attribute

Every resolver field needs one `#[by(...)]` attribute. Use either exactly
one selector or one custom resolver. Selector and `description` values must
be string literals.

| Attribute                | Selector used          |
| ------------------------ | ---------------------- |
| `id = "..."`             | `By::Id`               |
| `css = "..."`            | `By::Css`              |
| `xpath = "..."`          | `By::XPath`            |
| `tag = "..."`            | `By::Tag`              |
| `class = "..."`          | `By::ClassName`        |
| `name = "..."`           | `By::Name`             |
| `link = "..."`           | `By::LinkText`         |
| `partial_link = "..."`   | `By::PartialLinkText`  |
| `testid = "..."`         | `By::Testid`           |

All selectors start a query from the component's base element. For XPath,
remember that `//div` searches from the document root while `.//div` searches
within the base element.

### Resolver Types And Cardinality

Built-in selector resolvers can return `WebElement`, a nested `Component`, or
a `Vec` of either. The macro infers single- versus multi-element behaviour from
the field type:

| Field type                        | Inferred mode | Default cardinality |
| --------------------------------- | ------------- | ------------------- |
| `ElementResolver<T>`              | single        | exactly one         |
| `ElementResolver<Vec<T>>`         | multi         | one or more         |
| `ElementResolverSingle`           | single        | exactly one         |
| `ElementResolverMulti`            | multi         | one or more         |
| user-defined resolver type alias   | single        | use `multi` to force multi mode |

Here, `T` is either `WebElement` or a type implementing `Component + Clone`.
Nested components are constructed from each matched `WebElement` through
their generated `From<WebElement>` implementation.

`ElementResolver<Vec<T>>` is detected through the normal `ElementResolver`,
`components::ElementResolver`, and `thirtyfour::components::ElementResolver`
paths. If a type alias hides the `Vec`, add the `multi` option. The convenience
alias `ElementResolverMulti` is also detected when used directly.

### Query Options

Pair a selector with options, separated by commas:

| Option                                      | Applies to | Effect |
| ------------------------------------------- | ---------- | ------ |
| `single`                                    | single     | Require exactly one match. This is the default. |
| `first`                                     | single     | Return the first match instead of requiring uniqueness. |
| `not_empty`                                 | multi      | Require one or more matches. This is the default. |
| `allow_empty`                               | multi      | Return all matches, including an empty `Vec`. |
| `multi`                                     | multi      | Force multi mode when a type alias hides `Vec<T>`. |
| `description = "..."`                       | both       | Include a human-readable label in query errors. |
| `ignore_errors`                             | both       | Ignore query errors while polling and keep trying until the timeout. |
| `wait(timeout_ms = N, interval_ms = N)`     | both       | Override both the polling timeout and interval. Both arguments are required. |
| `nowait`                                    | both       | Poll once instead of waiting. |

The default mode uses the query system's default polling configuration.
`single` and `first` are mutually exclusive, as are `not_empty` and
`allow_empty`, and `wait(...)` and `nowait`. Single-only options cannot be
used for a multi resolver, and multi-only options cannot be used for a single
resolver. The derive also rejects repeated options and multiple selectors.

Examples:

```rust
type LinkListResolver = ElementResolver<Vec<WebElement>>;

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

// A custom alias that contains ElementResolver<Vec<WebElement>>.
#[by(partial_link = "Guide", multi, allow_empty)]
guide_links: LinkListResolver,
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
use thirtyfour::{resolve, resolve_present};

let elem = resolve!(self.input);            // self.input.resolve().await?
let elem = resolve_present!(self.input);    // self.input.resolve_present().await?
```

The macros are useful for chained method calls without scattering
`.await?` around. They are exported from the crate root instead of the
prelude so broad macro names do not get glob-imported accidentally:

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
#[derive(Debug, Clone, Component)]
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

A custom resolver receives the component's base element by value and returns
a `WebDriverResult<T>` matching the field's `ElementResolver<T>` type. Pass a
function path or another compatible expression, not a quoted string:

```rust
// Assume Row is another Component.
async fn resolve_rows(base: WebElement) -> WebDriverResult<Vec<Row>> {
    let elements = base.query(By::Css("tbody > tr")).all_from_selector().await?;
    Ok(elements.into_iter().map(Row::from).collect())
}

#[derive(Debug, Clone, Component)]
pub struct Table {
    base: WebElement,
    #[by(custom = resolve_rows)]
    rows: ElementResolver<Vec<Row>>,
}
```

The resolver function owns the complete lookup policy, including cardinality,
polling, filtering, and error handling. Therefore `custom = ...` cannot be
combined with a selector or any other `#[by(...)]` option. Single and `Vec`
field types are still inferred normally; do not add `multi` to a custom
resolver.

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

With the default `component` feature enabled, `use thirtyfour::prelude::*;`
imports the `Component` derive macro and `ElementResolver`. Import
`resolve!` and `resolve_present!` explicitly from `thirtyfour` when you
want the shorthand macros.
