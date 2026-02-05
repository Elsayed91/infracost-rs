//! GCP Static IP pricing.

use crate::types::ProductFilter;
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Defaults
// ============================================================

/// Default hourly price for static IP
const DEFAULT_PRICE: f64 = 0.01;
const UNIT: &str = "hour";

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
        self.fetch_internal().await
    }

    /// Fetch monthly cost (hourly rate × 730 hours).
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let hourly = self.fetch_internal().await?;
        Ok(PriceResult {
            price: hourly.price * 730.0,
            unit: "month".to_string(),
            source: hourly.source,
        })
    }

    async fn fetch_internal(self) -> Result<PriceResult> {
        let default_price = self.override_default.unwrap_or(DEFAULT_PRICE);

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
                let price = products[0].first_nonzero_price_or(default_price);
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

    fn build_filter(&self) -> ProductFilter {
        ProductFilter::builder()
            .vendor("gcp")
            .service("Compute Engine")
            .region(self.region.as_deref().unwrap_or("us-central1"))
            .product_family("Network")
            .attribute("description", "Static Ip Charge")
            .build()
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
        // 0.01 × 730 = 7.30
        assert_eq!(result.price, 7.30);
        assert_eq!(result.unit, "month");
    }
}
