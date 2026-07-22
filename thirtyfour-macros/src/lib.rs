//! Thirtyfour is a Selenium / WebDriver library for Rust, for automated website UI testing.
//!
//! This crate provides proc macros for use with [thirtyfour](https://docs.rs/thirtyfour).
//!
//!

use crate::component::expand_component_derive;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod component;

macro_rules! bail {
    ($span: expr, $($fmt:tt)*) => {
        return Err(syn::Error::new($span, format_args!($($fmt)*)))
    };
}

pub(crate) use bail;

/// Derive macro for a wrapped [`Component`].
///
/// A component owns a base [`WebElement`]. Its [`ElementResolver`] fields lazily query from that
/// element and cache their results. Queries normally stay within the base element's subtree;
/// XPath expressions beginning with `//` are document-rooted, so use `.//` for a relative XPath.
///
/// ## Supported structs and generated API
///
/// The derive supports non-generic structs with named fields. Exactly one field must be the base
/// `WebElement`. A field named `base` is detected automatically; use `#[base]` to give it another
/// name. Multiple base fields, tuple structs, enums, and unions are not supported.
///
/// The derive generates:
///
/// - `pub fn new(base: WebElement) -> Self`;
/// - `From<WebElement>`;
/// - the [`Component`] implementation and its `base_element()` method.
///
/// A field without `#[by(...)]` is initialized with `Default::default()`. Its type must therefore
/// implement `Default`. A field-level `#[cfg(...)]` is preserved in the generated constructor.
///
/// ## Selectors
///
/// A normal resolver requires exactly one selector with a string-literal value:
///
/// | Attribute | Generated selector |
/// | --- | --- |
/// | `id = "..."` | `By::Id` |
/// | `tag = "..."` | `By::Tag` |
/// | `link = "..."` | `By::LinkText` |
/// | `partial_link = "..."` | `By::PartialLinkText` |
/// | `css = "..."` | `By::Css` |
/// | `xpath = "..."` | `By::XPath` |
/// | `name = "..."` | `By::Name` |
/// | `class = "..."` | `By::ClassName` |
/// | `testid = "..."` | `By::Testid` |
///
/// ## Resolver type and cardinality
///
/// `ElementResolver<T>` and `ElementResolverSingle` use single mode, which requires exactly one
/// match by default. `ElementResolver<Vec<T>>` and the unqualified `ElementResolverMulti` alias
/// use multi mode, which requires at least one match by default. Here `T` may be `WebElement` or
/// a nested type that implements `Component + Clone`.
///
/// The `Vec` is detected for `ElementResolver`, `components::ElementResolver`, and
/// `thirtyfour::components::ElementResolver` paths. If a user-defined type alias hides the `Vec`,
/// add `multi` to force multi mode.
///
/// ## Query options
///
/// Selector resolvers accept these comma-separated options:
///
/// | Option | Mode | Behavior |
/// | --- | --- | --- |
/// | `single` | single | Require exactly one match. This is the default. |
/// | `first` | single | Return the first match instead of requiring uniqueness. |
/// | `not_empty` | multi | Require one or more matches. This is the default. |
/// | `allow_empty` | multi | Return all matches, including an empty `Vec`. |
/// | `multi` | multi | Force multi mode when a type alias hides `Vec<T>`. |
/// | `description = "..."` | both | Add a human-readable label to query errors. |
/// | `ignore_errors` | both | Ignore query errors while polling and keep trying. |
/// | `wait(timeout_ms = N, interval_ms = N)` | both | Override the polling timeout and interval; both arguments are required. |
/// | `nowait` | both | Poll once without waiting. |
///
/// The derive rejects repeated options and incompatible combinations. In particular, `single`
/// and `first`, `not_empty` and `allow_empty`, and `wait(...)` and `nowait` are mutually
/// exclusive. Single-only and multi-only options must match the inferred resolver mode.
///
/// ## Custom resolvers
///
/// Use `custom = expression` instead of a selector to supply a compatible [`ElementQueryFn`]. A
/// custom resolver receives the base `WebElement` by value and returns `WebDriverResult<T>`, where
/// `T` matches the field's `ElementResolver<T>` type. Pass a function path or another expression,
/// not a quoted string. `custom` cannot be combined with a selector or any other option.
///
/// ## Example
///
/// ```ignore
/// async fn resolve_button(base: WebElement) -> WebDriverResult<WebElement> {
///     base.query(By::ClassName("run-button")).and_displayed().first().await
/// }
///
/// #[derive(Debug, Clone, Component)]
/// pub struct FormComponent {
///     #[base]
///     root: WebElement,
///     #[by(css = "input[type='checkbox']", first)]
///     checkbox: ElementResolver<WebElement>,
///     #[by(partial_link = "Help", description = "help link")]
///     help: ElementResolver<WebElement>,
///     #[by(tag = "label", allow_empty)]
///     labels: ElementResolver<Vec<WebElement>>,
///     #[by(custom = resolve_button)]
///     button: ElementResolver<WebElement>,
///     attempts: u32,
/// }
/// ```
///
/// [`Component`]: https://docs.rs/thirtyfour/latest/thirtyfour/components/trait.Component.html
/// [`ElementQueryFn`]: https://docs.rs/thirtyfour/latest/thirtyfour/common/types/trait.ElementQueryFn.html
/// [`ElementResolver`]: https://docs.rs/thirtyfour/latest/thirtyfour/components/struct.ElementResolver.html
/// [`WebElement`]: https://docs.rs/thirtyfour/latest/thirtyfour/struct.WebElement.html
#[proc_macro_derive(Component, attributes(base, by))]
pub fn derive_component_fn(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    expand_component_derive(ast).into()
}
