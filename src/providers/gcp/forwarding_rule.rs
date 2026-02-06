//! GCP Forwarding Rule (Load Balancer) pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//!
//! # Per-unit pricing (hourly uptime)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.gcp().forwarding_rule().fetch().await?;
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
//! let cost = client.gcp().forwarding_rule()
//!     .data_processed_gb(1000)  // 1000 GB of data processed per month
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use crate::types::ProductFilter;
use crate::{Client, Result};

use super::super::{PriceResult, PriceSource};

// ============================================================
// Defaults
// ============================================================

/// Default hourly price for forwarding rule (minimum service charge)
const DEFAULT_HOURLY_PRICE: f64 = 0.025;
/// Default price for data processing (per GiB)
const DEFAULT_DATA_PROCESSING_PRICE: f64 = 0.008;
const UNIT: &str = "hour";

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP Forwarding Rule prices.
pub struct ForwardingRuleBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    // Data volume for monthly cost calculation
    data_processed_gb: Option<u64>,
}

impl<'a> ForwardingRuleBuilder<'a> {
    /// Create a new Forwarding Rule builder
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
        let default_price = self.override_default.unwrap_or(DEFAULT_HOURLY_PRICE);

        // Determine effective API key
        let effective_key = self.api_key.as_deref().or_else(|| {
            if self.client.has_api_key() {
                Some("")
            } else {
                None
            }
        });

        // No API key and not required → return default immediately
        if effective_key.is_none() && !self.client.error_on_fallback() {
            return Ok(PriceResult::from_default(default_price, UNIT));
        }

        // Try API
        let filter = self.build_filter();
        let api_key_for_query = self.api_key.as_deref();

        match self
            .client
            .query_products_with_key(filter, api_key_for_query)
            .await
        {
            Ok(products) if !products.is_empty() => {
                // Filter for Regional External Forwarding Rule Minimum
                // (excludes Internal, Cross-Regional, and Global load balancers)
                let matching_product = products.iter().find(|product| {
                    product.attributes.iter().any(|attr| {
                        attr.key == "description"
                            && attr
                                .value
                                .as_ref()
                                .map(|v| {
                                    v.contains("Regional External")
                                        && v.contains("Forwarding Rule Minimum")
                                })
                                .unwrap_or(false)
                    })
                });

                let price = matching_product
                    .map(|p| p.first_nonzero_price_or(default_price))
                    .unwrap_or(default_price);
                Ok(PriceResult::from_api(price, UNIT))
            }
            Ok(_) if !self.client.error_on_fallback() => {
                Ok(PriceResult::from_default(default_price, UNIT))
            }
            Err(_) if !self.client.error_on_fallback() => {
                Ok(PriceResult::from_default(default_price, UNIT))
            }
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
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
    /// let cost = client.gcp().forwarding_rule()
    ///     .region("us-central1")
    ///     .data_processed_gb(1000)
    ///     .fetch_monthly().await?;
    /// // Cost = ($0.025 * 730) + ($0.008 * 1000) = $26.25/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let region = self.region.as_deref().unwrap_or("us-central1");

        // Get price components
        let hourly_price = self.fetch_hourly_price(region).await?;
        let data_price = self.fetch_data_processing_price(region).await?;

        // Calculate costs
        // 730 hours = average hours per month (365 days * 24 hours / 12 months)
        let uptime_cost = hourly_price * 730.0;

        // Data processing cost
        let data_gb = self.data_processed_gb.unwrap_or(0);
        let data_cost = data_gb as f64 * data_price;

        let total = uptime_cost + data_cost;

        // Determine source based on whether we got API prices
        let source = if self.client.has_api_key() || self.api_key.is_some() {
            PriceSource::Api
        } else {
            PriceSource::Default
        };

        Ok(PriceResult {
            price: total,
            unit: "month".to_string(),
            source,
        })
    }

    /// Fetch hourly uptime price
    async fn fetch_hourly_price(&self, region: &str) -> Result<f64> {
        let default = DEFAULT_HOURLY_PRICE;

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        // Use resourceGroup for cross-region compatibility
        // Filter for Regional External load balancers (most common use case)
        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("Networking")
            .region(region)
            .product_family("Network")
            .attribute("resourceGroup", "LoadBalancing")
            .build();

        match self
            .client
            .query_products_with_key(filter, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => {
                // Filter for Regional External Forwarding Rule Minimum
                // (excludes Internal, Cross-Regional, and Global load balancers)
                let matching_product = products.iter().find(|product| {
                    product.attributes.iter().any(|attr| {
                        attr.key == "description"
                            && attr
                                .value
                                .as_ref()
                                .map(|v| {
                                    v.contains("Regional External")
                                        && v.contains("Forwarding Rule Minimum")
                                })
                                .unwrap_or(false)
                    })
                });

                let price = matching_product
                    .map(|p| p.first_nonzero_price_or(default))
                    .unwrap_or(default);
                Ok(price)
            }
            _ if !self.client.error_on_fallback() => Ok(default),
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    /// Fetch data processing price per GiB
    async fn fetch_data_processing_price(&self, region: &str) -> Result<f64> {
        let default = DEFAULT_DATA_PROCESSING_PRICE;

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        // Use resourceGroup for cross-region compatibility
        // Filter for Regional External Outbound data processing (most common use case)
        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("Networking")
            .region(region)
            .product_family("Network")
            .attribute("resourceGroup", "LoadBalancing")
            .build();

        match self
            .client
            .query_products_with_key(filter, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => {
                // Filter for Regional External Outbound Data Processing
                // (excludes Internal, Inbound, Cross-Regional, and Global)
                let matching_product = products.iter().find(|product| {
                    product.attributes.iter().any(|attr| {
                        attr.key == "description"
                            && attr
                                .value
                                .as_ref()
                                .map(|v| {
                                    v.contains("Regional External")
                                        && v.contains("Outbound Data Processing")
                                })
                                .unwrap_or(false)
                    })
                });

                let price = matching_product
                    .map(|p| p.first_nonzero_price_or(default))
                    .unwrap_or(default);
                Ok(price)
            }
            _ if !self.client.error_on_fallback() => Ok(default),
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    fn build_filter(&self) -> ProductFilter {
        // Use resourceGroup for cross-region compatibility
        // Returns all load balancing products; code filtering selects the right one
        ProductFilter::builder()
            .vendor("gcp")
            .service("Networking")
            .region(self.region.as_deref().unwrap_or("us-central1"))
            .product_family("Network")
            .attribute("resourceGroup", "LoadBalancing")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_forwarding_rule_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.025);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_forwarding_rule_fetch_monthly_with_data_processing() {
        // Forwarding Rule with 1000 GB of data processed
        // Cost = ($0.025 * 730 hours) + ($0.008 * 1000 GB) = $18.25 + $8.0 = $26.25/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 26.25);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_forwarding_rule_fetch_monthly_hourly_only() {
        // Forwarding Rule with no data processing (0 GB)
        // Cost = $0.025 * 730 hours = $18.25/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .data_processed_gb(0)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 18.25);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_forwarding_rule_fetch_monthly_without_data_defaults_to_zero() {
        // Forwarding Rule without specifying data_processed_gb defaults to 0
        // Cost = $0.025 * 730 hours = $18.25/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 18.25);
        assert_eq!(result.unit, "month");
    }
}
