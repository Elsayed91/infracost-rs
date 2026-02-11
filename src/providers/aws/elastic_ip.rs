//! AWS Elastic IP pricing.

use std::collections::HashMap;

use crate::catalog::{aws_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS Elastic IP prices.
///
/// Returns the price for an idle (unused) Elastic IP address.
pub struct ElasticIpBuilder {
    client: Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl ElasticIpBuilder {
    /// Create a new Elastic IP builder
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
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

    /// Fetch just the price value.
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the monthly price (hourly price x 730 hours).
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("elastic-ip")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let params = HashMap::new();
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

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("elastic-ip")?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_elastic_ip_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.005);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_elastic_ip_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        // $0.005/hour × 730 hours = $3.65/month
        assert_eq!(result.price, 3.65);
        assert_eq!(result.unit, "month");
    }
}
