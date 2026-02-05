//! AWS NAT Gateway pricing.

use crate::types::ProductFilter;
use crate::{Client, Result};

use super::super::{PriceResult, PriceSource};

// ============================================================
// Defaults
// ============================================================

/// Default hourly price for NAT Gateway
const DEFAULT_PRICE: f64 = 0.045;
const UNIT: &str = "hour";

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS NAT Gateway prices.
///
/// Returns the hourly rate for NAT Gateway. Additional data processing
/// charges apply ($0.045/GB).
pub struct NatGatewayBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    // Data specs for monthly cost calculation
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
            .product_family("NAT Gateway")
            .attribute("usagetype", "NatGateway-Hours")
            .build()
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
        let region = self.region.as_deref().unwrap_or("us-east-1");

        // Get price components
        let hourly_price = self.fetch_hourly_price(region).await?;
        let data_price = self.fetch_data_processing_price(region).await?;

        // Calculate hourly cost for 730 hours/month
        let hourly_cost = hourly_price * 730.0;

        // Calculate data processing cost
        let data_cost = if let Some(gb) = self.data_processed_gb {
            gb as f64 * data_price
        } else {
            0.0
        };

        let total = hourly_cost + data_cost;

        // Determine source based on whether we have API key
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

    /// Fetch hourly price for NAT Gateway
    async fn fetch_hourly_price(&self, region: &str) -> Result<f64> {
        let default = DEFAULT_PRICE;

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        let filter = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .product_family("NAT Gateway")
            .attribute("usagetype", "NatGateway-Hours")
            .build();

        match self
            .client
            .query_products_with_key(filter, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => Ok(products[0].first_nonzero_price_or(default)),
            _ if !self.client.error_on_fallback() => Ok(default),
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    /// Fetch data processing price per GB
    async fn fetch_data_processing_price(&self, region: &str) -> Result<f64> {
        let default = DEFAULT_PRICE; // Same as hourly: $0.045/GB

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        let filter = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .product_family("NAT Gateway")
            .attribute("usagetype", "NatGateway-Bytes")
            .build();

        match self
            .client
            .query_products_with_key(filter, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => Ok(products[0].first_nonzero_price_or(default)),
            _ if !self.client.error_on_fallback() => Ok(default),
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
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
