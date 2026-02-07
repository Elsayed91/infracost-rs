//! Azure Public IP pricing.

use crate::catalog::azure_catalog;
use crate::catalog::engine::PricingEngine;
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying Azure Public IP prices.
///
/// Returns the per-hour price for a Standard static public IPv4 address.
pub struct PublicIpBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl<'a> PublicIpBuilder<'a> {
    /// Create a new public IP builder
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
        }
    }

    /// Set the Azure region (e.g., "eastus", "westus2")
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
        let resource = azure_catalog().find("public-ip")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            self.client,
            resource,
            "azure",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch the monthly price (hourly price * 730 hours).
    ///
    /// This is a convenience method for calculating monthly costs.
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        // Use fetch() to get hourly price (respects override_default), then multiply
        let unit_result = self.fetch().await?;
        let monthly_price = unit_result.price * 730.0;

        Ok(PriceResult {
            price: monthly_price,
            unit: "month".to_string(),
            source: unit_result.source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_public_ip_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .public_ip()
            .region("eastus")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.005);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_public_ip_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .public_ip()
            .region("eastus")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 0.005 × 730 = 3.65
        assert_eq!(result.price, 3.65);
        assert_eq!(result.unit, "month");
    }
}
