//! AWS Application Load Balancer pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//!
//! # Per-unit pricing (original behavior)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.aws().alb().fetch().await?;
//! println!("${}/hour", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with LCU usage
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.aws().alb()
//!     .lcu_hours(10000)  // 10,000 LCU-hours per month
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::catalog::{aws_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS Application Load Balancer prices.
///
/// Returns the hourly rate for ALB. Additional LCU (Load Balancer Capacity Units)
/// charges apply based on usage.
pub struct AlbBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    // LCU usage for monthly cost calculation
    lcu_hours: Option<u64>,
}

impl<'a> AlbBuilder<'a> {
    /// Create a new ALB builder
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            lcu_hours: None,
        }
    }

    /// Set the AWS region (e.g., "us-east-1")
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the API key for this request.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the default fallback price.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set the LCU-hours per month (required for `fetch_monthly`).
    ///
    /// LCU (Load Balancer Capacity Unit) is a dimension that represents the resources
    /// needed to process your traffic. ALB pricing consists of:
    /// - Hourly charge (~$0.0225/hour = ~$16.43/month for 730 hours)
    /// - LCU charge (~$0.008/LCU-hour)
    ///
    /// The number of LCUs you need depends on your traffic patterns and is calculated
    /// based on the maximum of:
    /// - New connections per second
    /// - Active connections per minute
    /// - Processed bytes
    /// - Rule evaluations
    pub fn lcu_hours(mut self, lcu_hours: u64) -> Self {
        self.lcu_hours = Some(lcu_hours);
        self
    }

    /// Fetch just the price value.
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("alb")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            self.client,
            resource,
            "aws",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch total monthly cost based on hourly rate and LCU usage.
    ///
    /// Calculates: (hourly_rate * 730 hours) + (lcu_rate * lcu_hours)
    ///
    /// If `lcu_hours()` is not set, only the hourly cost is calculated.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.aws().alb()
    ///     .lcu_hours(10000)
    ///     .fetch_monthly().await?;
    /// // Cost = ($0.0225 * 730) + ($0.008 * 10000) = $16.43 + $80 = $96.43/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("alb")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let mut params = HashMap::new();
        if let Some(lcu) = self.lcu_hours {
            params.insert("lcu_hours".to_string(), lcu);
        }
        PricingEngine::fetch_monthly(
            self.client,
            resource,
            "aws",
            region,
            self.api_key.as_deref(),
            &params,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alb_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0225);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_hourly_only() {
        // ALB with no LCU usage specified - hourly cost only
        // Cost = $0.0225/hour * 730 hours = $16.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 16.425);
        assert_eq!(result.unit, "month");
        assert!(result.is_from_default());
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_with_lcu() {
        // ALB with 10,000 LCU-hours per month
        // Cost = ($0.0225 * 730) + ($0.008 * 10000)
        //      = $16.425 + $80 = $96.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(10000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 96.425);
        assert_eq!(result.unit, "month");
        assert!(result.is_from_default());
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_minimal_lcu() {
        // ALB with minimal LCU usage (730 LCU-hours = 1 LCU for whole month)
        // Cost = ($0.0225 * 730) + ($0.008 * 730)
        //      = $16.425 + $5.84 = $22.265/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(730)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 22.265);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_alb_fetch_monthly_zero_lcu() {
        // ALB with zero LCU-hours specified (same as not specifying)
        // Cost = $0.0225 * 730 = $16.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 16.425);
        assert_eq!(result.unit, "month");
    }
}
