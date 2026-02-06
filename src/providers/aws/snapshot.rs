//! AWS EBS Snapshot pricing.

use crate::types::ProductFilter;
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Defaults
// ============================================================

const DEFAULT_PRICE: f64 = 0.05;
const UNIT: &str = "GB-month";

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS EBS Snapshot prices.
pub struct SnapshotBuilder<'a> {
    client: &'a Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
}

impl<'a> SnapshotBuilder<'a> {
    /// Create a new snapshot builder
    pub(crate) fn new(client: &'a Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
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

    /// Set the snapshot size in GB (required for `fetch_monthly`).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Fetch just the price value.
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the monthly cost (rate × size_gb).
    ///
    /// Requires `size_gb` to be set.
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let size = self
            .size_gb
            .ok_or_else(|| crate::Error::validation("size_gb is required for fetch_monthly"))?;
        let rate = self.fetch().await?;
        Ok(PriceResult {
            price: rate.price * size as f64,
            unit: "month".to_string(),
            source: rate.source,
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
                // Filter for standard snapshot (not archive/outposts)
                // productFamily query returns multiple snapshot types
                let standard_snapshot = products.iter().find(|p| {
                    p.attributes.iter().any(|attr| {
                        attr.key == "usagetype"
                            && attr
                                .value
                                .as_ref()
                                .map(|v| {
                                    v.ends_with("EBS:SnapshotUsage")
                                        && !v.contains("Archive")
                                        && !v.contains("outposts")
                                })
                                .unwrap_or(false)
                    })
                });

                let price = standard_snapshot
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

    fn build_filter(&self) -> ProductFilter {
        // Use productFamily for cross-region compatibility
        // usagetype varies by region (EU-, APS1-, etc. prefixes)
        ProductFilter::builder()
            .vendor("aws")
            .region(self.region.as_deref().unwrap_or("us-east-1"))
            .product_family("Storage Snapshot")
            .attribute("servicecode", "AmazonEC2")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_snapshot_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
        assert_eq!(result.unit, "GB-month");
    }

    #[tokio::test]
    async fn test_snapshot_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .size_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        // $0.05/GB-month × 100 GB = $5.00/month
        assert_eq!(result.price, 5.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_snapshot_fetch_monthly_requires_size() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .fetch_monthly()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size_gb is required"));
    }
}
