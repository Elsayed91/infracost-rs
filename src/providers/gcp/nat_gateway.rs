//! GCP NAT Gateway pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//!
//! # Per-unit pricing (hourly uptime)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.gcp().nat_gateway().fetch().await?;
//! println!("${}/hour", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with data processing
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.gcp().nat_gateway()
//!     .data_processed_gb(1000)  // 1000 GB of data processed per month
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::catalog::{engine::PricingEngine, gcp_catalog};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP NAT Gateway prices.
pub struct NatGatewayBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    // Data volume for monthly cost calculation
    data_processed_gb: Option<u64>,
}

impl<'a> NatGatewayBuilder<'a> {
    /// Create a new NAT Gateway builder
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
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

    /// Set the amount of data processed in GB per month (required for `fetch_monthly`).
    pub fn data_processed_gb(mut self, gb: u64) -> Self {
        self.data_processed_gb = Some(gb);
        self
    }

    /// Fetch just the price value.
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    /// Returns the hourly uptime charge only (no data processing).
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("nat-gateway")?;
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

    /// Fetch total monthly cost based on data processing usage.
    ///
    /// Calculates: (hourly_rate * 730 hours) + (data_rate * gb_processed)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.gcp().nat_gateway()
    ///     .region("us-central1")
    ///     .data_processed_gb(1000)
    ///     .fetch_monthly().await?;
    /// // Cost = ($0.0014 * 730) + ($0.045 * 1000) = $46.022/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("nat-gateway")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let data_gb = self.data_processed_gb.unwrap_or(0);
        let mut params = HashMap::new();
        params.insert("data_processed_gb".to_string(), data_gb);
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
    async fn test_nat_gateway_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0014);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_with_data_processing() {
        // NAT Gateway with 1000 GB of data processed
        // Cost = ($0.0014 * 730 hours) + ($0.045 * 1000 GB) = $1.022 + $45.0 = $46.022/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 46.022);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_hourly_only() {
        // NAT Gateway with no data processing (0 GB)
        // Cost = $0.0014 * 730 hours = $1.022/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .data_processed_gb(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 1.022);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_nat_gateway_fetch_monthly_without_data_defaults_to_zero() {
        // NAT Gateway without specifying data_processed_gb defaults to 0
        // Cost = $0.0014 * 730 hours = $1.022/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 1.022);
        assert_eq!(result.unit, "month");
    }
}
