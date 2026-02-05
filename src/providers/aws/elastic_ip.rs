//! AWS Elastic IP pricing.

use crate::types::ProductFilter;
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Defaults
// ============================================================

/// Default hourly price for idle/unused Elastic IP
const DEFAULT_PRICE: f64 = 0.005;
const UNIT: &str = "hour";

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS Elastic IP prices.
///
/// Returns the price for an idle (unused) Elastic IP address.
pub struct ElasticIpBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl<'a> ElasticIpBuilder<'a> {
    /// Create a new Elastic IP builder
    pub(crate) fn new(client: &'a Client) -> Self {
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

    /// Fetch the monthly price (hourly price × 730 hours).
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let hourly = self.fetch().await?;
        Ok(PriceResult {
            price: hourly.price * 730.0,
            unit: "month".to_string(),
            source: hourly.source,
        })
    }

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let default_price = self.override_default.unwrap_or(DEFAULT_PRICE);

        let effective_key = self.api_key.as_deref().or_else(|| {
            if self.client.has_api_key() {
                Some("")
            } else {
                None
            }
        });

        if effective_key.is_none() && !self.client.error_on_fallback() {
            return Ok(PriceResult::from_default(default_price, UNIT));
        }

        let filter = self.build_filter();
        let api_key_for_query = self.api_key.as_deref();

        match self
            .client
            .query_products_with_key(filter, api_key_for_query)
            .await
        {
            Ok(products) if !products.is_empty() => {
                // EIP has tiered pricing - first hour free, then $0.005/hour
                // We return the non-zero price (after first hour)
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
            .vendor("aws")
            .region(self.region.as_deref().unwrap_or("us-east-1"))
            .product_family("IP Address")
            .attribute("usagetype", "ElasticIP:IdleAddress")
            .attribute("servicecode", "AmazonEC2")
            .build()
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
