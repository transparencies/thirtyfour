//! `Performance` domain — paint and layout timing metrics.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::cdp::macros::string_enum;
use crate::error::WebDriverResult;

string_enum! {
    /// Clock domain used by `Performance` metrics.
    pub enum TimeDomain {
        /// Wall-clock time ticks.
        TimeTicks = "timeTicks",
        /// Per-thread CPU ticks.
        ThreadTicks = "threadTicks",
    }
}

/// `Performance.enable`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enable {
    /// Time domain for the metrics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_domain: Option<TimeDomain>,
}
impl CdpCommand for Enable {
    const METHOD: &'static str = "Performance.enable";
    type Returns = Empty;
}

/// `Performance.disable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Disable;
impl CdpCommand for Disable {
    const METHOD: &'static str = "Performance.disable";
    type Returns = Empty;
}

/// One named metric.
#[derive(Debug, Clone, Deserialize)]
pub struct Metric {
    /// Metric name.
    pub name: String,
    /// Metric value.
    pub value: f64,
}

/// `Performance.getMetrics`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GetMetrics;
/// Response for [`GetMetrics`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetMetricsResult {
    /// Collected metrics.
    pub metrics: Vec<Metric>,
}
impl CdpCommand for GetMetrics {
    const METHOD: &'static str = "Performance.getMetrics";
    type Returns = GetMetricsResult;
}

/// Domain facade returned by [`Cdp::performance`].
#[derive(Debug)]
pub struct PerformanceDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> PerformanceDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `Performance.enable` (default time domain).
    pub async fn enable(&self) -> WebDriverResult<()> {
        self.cdp.send(Enable::default()).await?;
        Ok(())
    }

    /// `Performance.disable`.
    pub async fn disable(&self) -> WebDriverResult<()> {
        self.cdp.send(Disable).await?;
        Ok(())
    }

    /// `Performance.getMetrics`.
    pub async fn get_metrics(&self) -> WebDriverResult<Vec<Metric>> {
        Ok(self.cdp.send(GetMetrics).await?.metrics)
    }
}
