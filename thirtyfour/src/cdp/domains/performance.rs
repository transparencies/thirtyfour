//! `Performance` domain — paint and layout timing metrics.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::error::WebDriverResult;

/// `Performance.enable`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enable {
    /// Time domain for the metrics: `"timeTicks"` or `"threadTicks"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_domain: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn methods() {
        assert_eq!(Enable::METHOD, "Performance.enable");
        assert_eq!(Disable::METHOD, "Performance.disable");
        assert_eq!(GetMetrics::METHOD, "Performance.getMetrics");
    }

    #[test]
    fn enable_default_skips_time_domain() {
        let v = serde_json::to_value(Enable::default()).unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn enable_with_time_domain() {
        let v = serde_json::to_value(Enable {
            time_domain: Some("threadTicks".to_string()),
        })
        .unwrap();
        assert_eq!(v["timeDomain"], "threadTicks");
    }

    #[test]
    fn metric_parses() {
        let m: Metric = serde_json::from_value(json!({"name": "Timestamp", "value": 1.0})).unwrap();
        assert_eq!(m.name, "Timestamp");
        assert!((m.value - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn get_metrics_result_parses() {
        let body = json!({
            "metrics": [
                {"name": "Timestamp", "value": 1.0},
                {"name": "JSEventListeners", "value": 5.0}
            ]
        });
        let r: GetMetricsResult = serde_json::from_value(body).unwrap();
        assert_eq!(r.metrics.len(), 2);
        assert_eq!(r.metrics[0].name, "Timestamp");
    }
}
