# Translating Selenium And Playwright To `thirtyfour`

Use this guide when porting an existing Selenium or Playwright flow, or when an
AI-generated example uses an API that does not exist in `thirtyfour`. The goal
is to preserve the intent of the original test while expressing waits,
selectors, session ownership, and reusable UI structure explicitly.

## Concept Map

| Selenium / Playwright concept | Idiomatic `thirtyfour` |
| --- | --- |
| Selenium `find_element` | `query(...).single().await?` for a unique target, or `first()` when first-match behavior is intentional |
| Selenium `find_elements` | `query(...).all_from_selector_required().await?`, `any_required()`, or their empty-allowed variants |
| Selenium `WebDriverWait` / expected conditions | Query filters while locating, then `element.wait_until()` for later state changes |
| Playwright `locator()` | `driver.query(...)` or a scoped `element.query(...)`; queries are builders, not persistent locator handles |
| Playwright locator auto-waiting | Explicit query filters such as `and_displayed()` and explicit element waits such as `wait_until().clickable()` |
| Selenium page objects | `Component` derive plus `ElementResolver`, with selectors kept inside the component |
| Playwright browser contexts | Separate `WebDriver` sessions for isolation, or explicit cookie/storage cleanup when session reuse is deliberate |
| Local browser launch | `WebDriver::managed(...)` |
| Selenium Grid / remote WebDriver | `WebDriver::new(...)` or `WebDriver::builder(...)` |

## Element Lookup

Selenium commonly performs a one-shot lookup:

```python
submit = driver.find_element(By.CSS_SELECTOR, "[data-testid='checkout-submit']")
```

Playwright represents the target as a locator:

```typescript
const submit = page.getByTestId("checkout-submit");
```

In `thirtyfour`, build a polling query and choose the cardinality that is part
of the page contract:

```rust,no_run
# use thirtyfour::prelude::*;
# async fn example(driver: &WebDriver) -> WebDriverResult<()> {
let submit = driver
    .query(By::Testid("checkout-submit"))
    .and_displayed()
    .desc("checkout submit button")
    .single()
    .await?;
# Ok(())
# }
```

`find()` and `find_all()` still exist as low-level, one-shot WebDriver
operations. They are not replacements for Selenium's explicit-wait patterns or
Playwright's auto-waiting. Prefer [element queries](../features/queries.md) for
normal automation.

For multiple elements, make empty-result behavior explicit:

```rust,no_run
# use thirtyfour::prelude::*;
# async fn example(driver: &WebDriver) -> WebDriverResult<()> {
let rows = driver
    .query(By::Css("[data-testid='results'] > [role='row']"))
    .desc("search result rows")
    .all_from_selector_required()
    .await?;
# Ok(())
# }
```

Use `all_from_selector()` when an empty collection is valid. When a query has
`.or(...)` branches, use `any()` / `any_required()` to combine matches from all
branches.

## Waiting And Interaction Readiness

Selenium often combines `WebDriverWait` with an expected condition, while
Playwright waits inside actions. `thirtyfour` keeps the required state visible
in the test.

Use query filters when locating an element that must already be ready:

```rust,no_run
# use thirtyfour::prelude::*;
# async fn example(driver: &WebDriver) -> WebDriverResult<()> {
let save = driver
    .query(By::Testid("settings-save"))
    .and_clickable()
    .desc("settings save button")
    .single()
    .await?;
save.click().await?;
# Ok(())
# }
```

Use `wait_until()` when an element you already hold must change state:

```rust,no_run
# use thirtyfour::prelude::*;
# async fn example(driver: &WebDriver) -> WebDriverResult<()> {
let spinner = driver
    .query(By::Testid("save-spinner"))
    .desc("save progress indicator")
    .single()
    .await?;
spinner.wait_until().not_displayed().await?;

driver
    .query(By::Testid("save-status"))
    .with_text("Saved")
    .and_displayed()
    .desc("saved settings status")
    .single()
    .await?;
# Ok(())
# }
```

Do not translate an implicit or auto-wait into a fixed sleep. See
[Waiting For Element Changes](../features/waiting.md) for the full predicate
set and timeout controls.

## Scoped Locators

A Playwright locator chain scopes later lookups. Do the same by querying from a
container `WebElement`:

```rust,no_run
# use thirtyfour::prelude::*;
# async fn example(driver: &WebDriver) -> WebDriverResult<()> {
let dialog = driver
    .query(By::Testid("delete-dialog"))
    .and_displayed()
    .desc("delete confirmation dialog")
    .single()
    .await?;

dialog
    .query(By::Testid("confirm-delete"))
    .and_clickable()
    .desc("confirm delete button")
    .single()
    .await?
    .click()
    .await?;
# Ok(())
# }
```

Unlike a Playwright `Locator`, an `ElementQuery` terminator resolves concrete
`WebElement` values; later actions do not automatically re-resolve those
elements. Run the query again when you need to locate the target after a
render, or use an `ElementResolver` inside a Component for reusable
stale-element recovery.

## Page Objects Become Components

Translate a Selenium page object into a focused Component that owns selectors
and exposes user-intent methods:

```rust,ignore
use thirtyfour::prelude::*;

#[derive(Debug, Clone, Component)]
struct LoginForm {
    base: WebElement,
    #[by(testid = "login-email")]
    email: ElementResolver<WebElement>,
    #[by(testid = "login-password")]
    password: ElementResolver<WebElement>,
    #[by(testid = "login-submit")]
    submit: ElementResolver<WebElement>,
}

impl LoginForm {
    async fn sign_in(&self, email: &str, password: &str) -> WebDriverResult<()> {
        self.email.resolve_present().await?.send_keys(email).await?;
        self.password
            .resolve_present()
            .await?
            .send_keys(password)
            .await?;

        let submit = self.submit.resolve_present().await?;
        submit.wait_until().clickable().await?;
        submit.click().await
    }
}
```

The default `component` feature exports `Component` and `ElementResolver`
through `thirtyfour::prelude::*`. See [Components](../features/components.md)
for resolver cardinality, nesting, caching, and stale-element behavior.

## Local, Remote, And Isolated Sessions

For a local installed browser, replace Playwright's browser launcher or a
manually started local driver with the managed setup:

```rust,no_run
# use thirtyfour::prelude::*;
# async fn example() -> WebDriverResult<()> {
let driver = WebDriver::managed(DesiredCapabilities::chrome()).await?;
let flow_result: WebDriverResult<()> = async {
    driver.goto("https://example.test").await?;
    Ok(())
}
.await;

let quit_result = driver.quit().await;
flow_result?;
quit_result
# }
```

For Selenium Grid or another externally owned endpoint, keep that lifecycle
external:

```rust,no_run
# use thirtyfour::prelude::*;
# async fn example() -> WebDriverResult<()> {
let driver = WebDriver::builder(
    "http://selenium-grid:4444",
    DesiredCapabilities::chrome(),
)
.await?;
driver.quit().await?;
# Ok(())
# }
```

See [WebDriver Manager](../features/manager.md) for local driver ownership and
[Manual WebDriver Setup](../appendix/manual-webdriver.md) for remote builder
configuration.

A Playwright browser context is not the same as a cloned `WebDriver`. Clones
control the same browser session and must not run independent flows
concurrently. Prefer a fresh managed session per isolated test. If session reuse
is intentional, explicitly reset cookies and application storage between flows;
do not assume a new context was created.

## APIs Not Provided By `thirtyfour`

These names are common translation mistakes, especially in generated code:

| Do not invent | Use instead |
| --- | --- |
| `page.locator(...)` / `driver.locator(...)` | `driver.query(...)` or `element.query(...)` |
| `get_by_role`, `get_by_label`, `get_by_text` | `By::Testid`, stable semantic CSS, or a strong selector plus a text filter |
| `expect(locator).to_be_visible()` | Query with `and_displayed()` or call `element.wait_until().displayed()` |
| `locator.click()` with hidden auto-waiting | Query with `and_clickable()`, or call `WebElement::click_when_ready()` on a deliberately held element |
| `browser.new_context()` | A separate `WebDriver` session, or deliberate cookie/storage cleanup |
| Selenium `WebDriverWait` / `ExpectedConditions` types | `query()` filters and `wait_until()` predicates |
| Selenium `ActionChains` spelling | `driver.action_chain()` and the `ActionChain` API |

`thirtyfour` is a WebDriver client, so it intentionally exposes WebDriver
sessions and elements rather than reproducing Playwright's object model. When a
translation needs richer structure, build it from queries, explicit waits, and
Components instead of guessing at a similarly named method.

## Translation Checklist

- Replace routine one-shot lookup with a described polling query.
- Choose `single()`, `first()`, or a multi-element terminator deliberately.
- Preserve interaction readiness with query filters or `wait_until()`.
- Scope through a container element or Component.
- Use `WebDriver::managed` locally and `new` / `builder` for remote services.
- Use separate sessions for isolated concurrent flows.
- Preserve explicit `quit()` on both success and failure paths.

Apply the [Reliable AI-Generated Tests](../getting-started/reliable-tests.md)
checklist after translating a flow.

For the source ecosystem behavior referenced here, see Selenium's official
[element finder](https://www.selenium.dev/documentation/webdriver/elements/finders/)
and [waiting](https://www.selenium.dev/documentation/webdriver/waits/)
documentation, and Playwright's official
[locator](https://playwright.dev/docs/locators) and
[actionability](https://playwright.dev/docs/actionability) documentation.
