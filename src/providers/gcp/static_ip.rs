//! GCP Static IP pricing.

use std::collections::HashMap;

use crate::catalog::{engine::PricingEngine, gcp_catalog};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP static IP prices.
pub struct StaticIpBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl<'a> StaticIpBuilder<'a> {
    /// Create a new static IP builder
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
        }
    }

    /// Set the GCP region (e.g., "us-central1")
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

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("static-ip")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch monthly cost (hourly rate x 730 hours).
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("static-ip")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let params = HashMap::new();
        PricingEngine::fetch_monthly(
            self.client,
            resource,
            "gcp",
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
    async fn test_static_ip_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.01);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_static_ip_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 0.01 x 730 = 7.30
        assert_eq!(result.price, 7.30);
        assert_eq!(result.unit, "month");
    }
}
