//! GCP Snapshot pricing.
//!
//! Supports both standard and archive snapshots.
//!
//! # Standard snapshot pricing
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::SnapshotType;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let price = client.gcp().snapshot(SnapshotType::Standard).fetch().await?;
//! println!("${}/GiB-month", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Archive snapshot pricing
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::SnapshotType;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let cost = client.gcp().snapshot(SnapshotType::Archive)
//!     .size_gb(500)
//!     .retrieval_size_gb(100)  // Optional: expected monthly retrieval volume
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
// Types
// ============================================================

/// GCP Snapshot types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotType {
    /// Standard snapshot ($0.05/GiB-month)
    Standard,
    /// Archive snapshot ($0.019/GiB-month storage + $0.019/GiB retrieval)
    Archive,
}

impl SnapshotType {
    /// Get the YAML catalog resource name for this snapshot type.
    fn resource_name(&self) -> &'static str {
        match self {
            Self::Standard => "snapshot/standard",
            Self::Archive => "snapshot/archive",
        }
    }
}

impl From<&str> for SnapshotType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "archive" => Self::Archive,
            _ => Self::Standard,
        }
    }
}

impl From<String> for SnapshotType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP snapshot prices.
pub struct SnapshotBuilder {
    client: Client,
    snapshot_type: SnapshotType,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
    retrieval_size_gb: Option<u64>,
}

impl SnapshotBuilder {
    /// Create a new snapshot builder.
    pub(crate) fn new(client: Client, snapshot_type: SnapshotType) -> Self {
        Self {
            client,
            snapshot_type,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
            retrieval_size_gb: None,
        }
    }

    /// Set the GCP region (e.g., "us-central1").
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the API key for this request.
    ///
    /// If not set and the client has no default key, returns built-in defaults.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the default fallback price.
    ///
    /// By default, the library uses built-in prices when the API is unavailable.
    /// Use this to specify a custom fallback.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set the snapshot size in GB (required for `fetch_monthly`).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Set the expected monthly retrieval volume in GB (optional, archive snapshots only).
    ///
    /// Archive snapshot retrieval costs $0.019/GiB in us-central1.
    /// Set this to the amount of data you expect to retrieve per month.
    /// The retrieval cost is added to the monthly storage cost in `fetch_monthly`.
    pub fn retrieval_size_gb(mut self, size: u64) -> Self {
        self.retrieval_size_gb = Some(size);
        self
    }

    /// Fetch just the price value (per-GiB storage rate).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    /// Returns the per-GiB storage rate for the primary component.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find(self.snapshot_type.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            &self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch total monthly cost based on snapshot size.
    ///
    /// Requires `size_gb()` to be set. Optionally set:
    /// - `retrieval_size_gb()` for archive snapshots (expected monthly retrieval volume)
    ///
    /// The calculation:
    /// - Standard: storage_price x size_gb
    /// - Archive: (storage_price x size_gb) + (retrieval_price x retrieval_size_gb)
    ///
    /// Note: `retrieval_size_gb` represents the expected monthly retrieval volume,
    /// not a one-time cost. The pricing engine multiplies the per-GiB retrieval
    /// price by the quantity you provide, consistent with how all usage-based
    /// components work in the codebase.
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let size_gb = self
            .size_gb
            .ok_or_else(|| crate::Error::validation("size_gb is required for fetch_monthly"))?;

        let resource = gcp_catalog().find(self.snapshot_type.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);

        let mut params = HashMap::new();
        params.insert("size_gb".to_string(), size_gb);
        if let Some(retrieval) = self.retrieval_size_gb {
            params.insert("retrieval_size_gb".to_string(), retrieval);
        }

        PricingEngine::fetch_monthly(
            &self.client,
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

    // ============================================================
    // SnapshotType tests
    // ============================================================

    #[test]
    fn test_snapshot_type_from_str() {
        assert_eq!(SnapshotType::from("standard"), SnapshotType::Standard);
        assert_eq!(SnapshotType::from("STANDARD"), SnapshotType::Standard);
        assert_eq!(SnapshotType::from("archive"), SnapshotType::Archive);
        assert_eq!(SnapshotType::from("ARCHIVE"), SnapshotType::Archive);
        // Unknown defaults to Standard
        assert_eq!(SnapshotType::from("unknown"), SnapshotType::Standard);
    }

    #[test]
    fn test_snapshot_type_resource_name() {
        assert_eq!(SnapshotType::Standard.resource_name(), "snapshot/standard");
        assert_eq!(SnapshotType::Archive.resource_name(), "snapshot/archive");
    }

    // ============================================================
    // Standard snapshot tests
    // ============================================================

    #[tokio::test]
    async fn test_standard_snapshot_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert!(
            (result.price - 0.05).abs() < 0.001,
            "expected ~0.05, got {}",
            result.price
        );
        assert_eq!(result.unit, "GiB-month");
    }

    #[tokio::test]
    async fn test_standard_snapshot_fetch_monthly() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .size_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // 0.05 x 100 = 5.00
        assert!(
            (result.price - 5.00).abs() < 0.001,
            "expected ~5.00, got {}",
            result.price
        );
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_standard_snapshot_fetch_monthly_requires_size() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .fetch_monthly()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size_gb is required"));
    }

    #[tokio::test]
    async fn test_standard_snapshot_override_default() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .override_default(0.07)
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert!(
            (result.price - 0.07).abs() < 0.001,
            "expected ~0.07, got {}",
            result.price
        );
    }

    #[tokio::test]
    async fn test_standard_snapshot_string_type() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot("standard")
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(
            (result.price - 0.05).abs() < 0.001,
            "expected ~0.05, got {}",
            result.price
        );
    }

    // ============================================================
    // Archive snapshot tests
    // ============================================================

    #[tokio::test]
    async fn test_archive_snapshot_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert!(
            (result.price - 0.019).abs() < 0.001,
            "expected ~0.019, got {}",
            result.price
        );
        assert_eq!(result.unit, "GiB-month");
    }

    #[tokio::test]
    async fn test_archive_snapshot_fetch_monthly_storage_only() {
        // 500 GB archive snapshot, no retrieval
        // Cost = 0.019 x 500 = 9.50
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert!(
            (result.price - 9.50).abs() < 0.001,
            "expected ~9.50, got {}",
            result.price
        );
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_archive_snapshot_fetch_monthly_with_retrieval() {
        // 500 GB archive snapshot with 100 GB retrieval
        // Cost = (0.019 x 500) + (0.019 x 100) = 9.50 + 1.90 = 11.40
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .size_gb(500)
            .retrieval_size_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert!(
            (result.price - 11.40).abs() < 0.001,
            "expected ~11.40, got {}",
            result.price
        );
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_archive_snapshot_fetch_monthly_requires_size() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .fetch_monthly()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size_gb is required"));
    }

    #[tokio::test]
    async fn test_archive_snapshot_string_type() {
        let client = Client::anonymous().unwrap();
        let result = client
            .gcp()
            .snapshot("archive")
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(
            (result.price - 0.019).abs() < 0.001,
            "expected ~0.019, got {}",
            result.price
        );
    }

    // ============================================================
    // Cost comparison test
    // ============================================================

    #[tokio::test]
    async fn test_archive_cheaper_than_standard() {
        let client = Client::anonymous().unwrap();

        let standard = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        let archive = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(archive.price < standard.price);
    }
}
