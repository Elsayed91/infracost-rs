//! AWS NAT Gateway pricing.

use std::collections::HashMap;

use crate::catalog::{aws_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS NAT Gateway prices.
///
/// Returns the hourly rate for NAT Gateway. Additional data processing
/// charges apply ($0.045/GB).
pub struct NatGatewayBuilder {
    client: Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    // Data specs for monthly cost calculation
    data_processed_gb: Option<u64>,
}

impl NatGatewayBuilder {
    /// Create a new NAT Gateway builder
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
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

    /// Set the amount of data processed in GB per month.
    ///
    /// Required for `fetch_monthly()` to calculate total monthly cost including
    /// both hourly charges and data processing charges.
    pub fn data_processed_gb(mut self, gb: u64) -> Self {
        self.data_processed_gb = Some(gb);
        self
    }

    /// Fetch just the price value.
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("nat-gateway")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            &self.client,
            resource,
            "aws",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch total monthly cost for NAT Gateway.
    ///
    /// Calculates: (hourly_rate * 730 hours) + (data_processing_rate * gb_processed)
    ///
    /// If `data_processed_gb()` is not set, only returns the hourly cost for 730 hours.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.aws().nat_gateway()
    ///     .region("us-east-1")
    ///     .data_processed_gb(1000)
    ///     .fetch_monthly().await?;
    /// // Cost = ($0.045 * 730) + ($0.045 * 1000) = $77.85/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("nat-gateway")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let mut params = HashMap::new();
        if let Some(gb) = self.data_processed_gb {
            params.insert("data_processed_gb".to_string(), gb);
        }
        PricingEngine::fetch_monthly(
            &self.client,
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
    async fn test_nat_gateway_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.045);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_with_data_processing() {
        // NAT Gateway with 1000 GB data processed per month
        // Cost = ($0.045 * 730) + ($0.045 * 1000) = $32.85 + $45.00 = $77.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 77.85);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_hourly_only() {
        // NAT Gateway with no data processing specified
        // Cost = $0.045 * 730 = $32.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 32.85);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_zero_data() {
        // NAT Gateway with 0 GB data processed
        // Cost = ($0.045 * 730) + ($0.045 * 0) = $32.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .data_processed_gb(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 32.85);
        assert_eq!(result.unit, "month");
    }
}
