mod common;

use std::time::Duration;

use assert_matches::assert_matches;
use common::*;
use rstest::rstest;
use thirtyfour::components::ElementResolverSingle;
use thirtyfour::error::WebDriverErrorInner;
use thirtyfour::prelude::*;
use thirtyfour::support::block_on;

#[rstest]
fn query_absence_polling(test_harness: TestHarness) -> WebDriverResult<()> {
    let c = test_harness.driver();
    block_on(async {
        c.goto(sample_page_url()).await?;

        // `nowait()` performs one complete check, including every `or()` branch.
        assert!(
            !c.query(By::Id("doesnotexist")).or(By::Id("navigation")).nowait().not_exists().await?
        );
        let error = c
            .query(By::Id("doesnotexist"))
            .or(By::Id("navigation"))
            .desc("navigation alternatives")
            .nowait()
            .wait_until_gone()
            .await
            .expect_err("the navigation branch still matches");
        let message = error.to_string();
        assert!(message.contains("navigation alternatives"));
        assert!(message.contains("Id(doesnotexist)"));
        assert!(message.contains("Id(navigation)"));
        assert_matches!(error.into_inner(), WebDriverErrorInner::Timeout(_));

        // A waiting `not_exists()` must forget matches from earlier poll attempts.
        c.execute("setTimeout(() => document.getElementById('navigation').remove(), 100);", vec![])
            .await?;
        assert!(
            c.query(By::Id("navigation"))
                .wait(Duration::from_secs(2), Duration::from_millis(20))
                .not_exists()
                .await?
        );

        // Filters are part of the query: the raw node may remain while the filtered match goes.
        c.execute(
            "const footer = document.getElementById('footer'); \
             footer.classList.add('temporary-match'); \
             setTimeout(() => footer.classList.remove('temporary-match'), 100);",
            vec![],
        )
        .await?;
        assert!(
            c.query(By::Id("footer"))
                .with_class("temporary-match")
                .wait(Duration::from_secs(2), Duration::from_millis(20))
                .not_exists()
                .await?
        );
        assert!(c.query(By::Id("footer")).nowait().exists().await?);

        // Every branch must be absent in the same poll attempt.
        c.goto(sample_page_url()).await?;
        c.execute(
            "setTimeout(() => document.getElementById('navigation').remove(), 50); \
             setTimeout(() => document.getElementById('footer').remove(), 150);",
            vec![],
        )
        .await?;
        c.query(By::Id("navigation"))
            .or(By::Id("footer"))
            .wait(Duration::from_secs(2), Duration::from_millis(20))
            .wait_until_gone()
            .await?;

        // Already-absent queries complete on their first attempt.
        c.query(By::Id("footer")).nowait().wait_until_gone().await?;
        Ok(())
    })
}

#[rstest]
fn resolver_element_helpers(test_harness: TestHarness) -> WebDriverResult<()> {
    let c = test_harness.driver();
    block_on(async {
        c.goto(sample_page_url()).await?;
        let section = c.find(By::Id("section-text")).await?;
        let input = ElementResolverSingle::new_single(section.clone(), By::Id("text-input2"));
        let button = ElementResolverSingle::new_single(section, By::Id("button-copy"));

        assert_eq!(input.value().await?.as_deref(), Some(""));
        input.send_keys("from resolver").await?;
        assert_eq!(input.value().await?.as_deref(), Some("from resolver"));
        assert_eq!(button.text().await?, "Copy");

        button.click().await?;
        assert_eq!(c.find(By::Id("text-output")).await?.text().await?, "from resolver");

        // Replace the cached button. The helper should validate and re-resolve it before clicking.
        c.execute(
            "const old = document.getElementById('button-copy'); old.replaceWith(old.cloneNode(true));",
            vec![],
        )
        .await?;
        input.clear().await?;
        input.send_keys("ready again").await?;
        button.click_when_ready().await?;
        assert_eq!(c.find(By::Id("text-output")).await?.text().await?, "ready again");

        Ok(())
    })
}
