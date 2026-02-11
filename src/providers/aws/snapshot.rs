//! AWS EBS Snapshot pricing.

use std::collections::HashMap;

use crate::catalog::{aws_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS EBS Snapshot prices.
pub struct SnapshotBuilder {
    client: Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
}

impl SnapshotBuilder {
    /// Create a new snapshot builder
    pub(crate) fn new(client: Client) -> Self {
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

    /// Fetch the monthly cost (rate x size_gb).
    ///
    /// Requires `size_gb` to be set.
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let size = self
            .size_gb
            .ok_or_else(|| crate::Error::validation("size_gb is required for fetch_monthly"))?;
        let resource = aws_catalog().find("snapshot")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let mut params = HashMap::new();
        params.insert("size_gb".to_string(), size);
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
        let resource = aws_catalog().find("snapshot")?;
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
