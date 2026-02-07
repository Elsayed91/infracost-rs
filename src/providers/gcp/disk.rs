//! GCP Persistent Disk pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//!
//! # Per-unit pricing (original behavior)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::DiskType;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.gcp().disk(DiskType::PdSsd).fetch().await?;
//! println!("${}/GB-month", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with specs (pd-extreme with IOPS)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::DiskType;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.gcp().disk(DiskType::PdExtreme)
//!     .size_gb(500)
//!     .iops(15000)  // Provisioned IOPS for pd-extreme
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

/// GCP Persistent Disk types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskType {
    /// Standard persistent disk (HDD)
    PdStandard,
    /// SSD persistent disk
    PdSsd,
    /// Balanced persistent disk
    PdBalanced,
    /// Extreme persistent disk (highest IOPS)
    PdExtreme,
    /// Hyperdisk Balanced (high performance balanced disk)
    HyperdiskBalanced,
    /// Hyperdisk Extreme (ultra-high performance with configurable IOPS)
    HyperdiskExtreme,
    /// Hyperdisk Throughput (optimized for high throughput workloads)
    HyperdiskThroughput,
    /// Hyperdisk ML (optimized for machine learning workloads)
    HyperdiskMl,
}

impl DiskType {
    /// Get the YAML catalog resource name for this disk type.
    fn resource_name(&self) -> &'static str {
        match self {
            Self::PdStandard => "disk/pd-standard",
            Self::PdSsd => "disk/pd-ssd",
            Self::PdBalanced => "disk/pd-balanced",
            Self::PdExtreme => "disk/pd-extreme",
            Self::HyperdiskBalanced => "disk/hyperdisk-balanced",
            Self::HyperdiskExtreme => "disk/hyperdisk-extreme",
            Self::HyperdiskThroughput => "disk/hyperdisk-throughput",
            Self::HyperdiskMl => "disk/hyperdisk-ml",
        }
    }

    /// Get the description pattern for this disk type (used for API filtering)
    pub fn description(&self) -> &'static str {
        match self {
            Self::PdStandard => "Storage PD Capacity",
            Self::PdSsd => "SSD backed PD Capacity",
            Self::PdBalanced => "Balanced PD Capacity",
            Self::PdExtreme => "Extreme PD Capacity",
            Self::HyperdiskBalanced => "Hyperdisk Balanced Capacity",
            Self::HyperdiskExtreme => "Hyperdisk Extreme Capacity",
            Self::HyperdiskThroughput => "Hyperdisk Throughput Capacity",
            Self::HyperdiskMl => "Hyperdisk ML Capacity",
        }
    }

    /// Get the resourceGroup for this disk type (more reliable for cross-region queries)
    pub fn resource_group(&self) -> &'static str {
        match self {
            Self::PdStandard => "PDStandard",
            // All SSD-based types (PD and Hyperdisk) share resourceGroup="SSD" in the API
            Self::PdSsd
            | Self::PdBalanced
            | Self::PdExtreme
            | Self::HyperdiskBalanced
            | Self::HyperdiskExtreme
            | Self::HyperdiskThroughput
            | Self::HyperdiskMl => "SSD",
        }
    }

    /// Get the default storage price for this disk type (per GB-month)
    pub fn default_storage_price(&self) -> f64 {
        match self {
            Self::PdStandard => 0.04,
            Self::PdSsd => 0.17,
            Self::PdBalanced => 0.10,
            Self::PdExtreme => 0.125,
            Self::HyperdiskBalanced => 0.08,
            Self::HyperdiskExtreme => 0.125,
            Self::HyperdiskThroughput => 0.005,
            Self::HyperdiskMl => 0.08,
        }
    }

    /// Get the default price for this disk type (per GB-month)
    /// Alias for default_storage_price for backward compatibility
    pub fn default_price(&self) -> f64 {
        self.default_storage_price()
    }

    /// Get the unit for disk pricing
    /// Note: GCP API returns prices in gibibyte (GiB), not gigabyte (GB)
    /// 1 GiB = 1.073741824 GB
    pub fn unit(&self) -> &'static str {
        "GiB-month"
    }

    /// Whether this disk type supports provisioned IOPS
    pub fn supports_iops(&self) -> bool {
        matches!(
            self,
            Self::PdExtreme | Self::HyperdiskBalanced | Self::HyperdiskExtreme
        )
    }

    /// Get the default IOPS price (per IOPS-month)
    pub fn default_iops_price(&self) -> Option<f64> {
        match self {
            Self::PdExtreme => Some(0.065),
            Self::HyperdiskBalanced => Some(0.005),
            Self::HyperdiskExtreme => Some(0.032),
            _ => None,
        }
    }

    /// Whether this disk type supports provisioned throughput
    pub fn supports_throughput(&self) -> bool {
        matches!(
            self,
            Self::HyperdiskBalanced | Self::HyperdiskThroughput | Self::HyperdiskMl
        )
    }

    /// Get the default throughput price (per MiB/s-month)
    pub fn default_throughput_price(&self) -> Option<f64> {
        match self {
            Self::HyperdiskBalanced => Some(0.04),
            Self::HyperdiskThroughput => Some(0.25),
            Self::HyperdiskMl => Some(0.12),
            _ => None,
        }
    }
}

impl From<&str> for DiskType {
    fn from(s: &str) -> Self {
        // Handle full GCP URLs like "projects/xxx/zones/us-central1-a/diskTypes/pd-ssd"
        let name = if s.contains('/') {
            s.rsplit('/').next().unwrap_or(s)
        } else {
            s
        };
        match name.to_lowercase().replace(['-', '_'], "").as_str() {
            "pdssd" | "ssd" => Self::PdSsd,
            "pdbalanced" | "balanced" => Self::PdBalanced,
            "pdextreme" | "extreme" => Self::PdExtreme,
            "hyperdiskbalanced" => Self::HyperdiskBalanced,
            "hyperdiskextreme" => Self::HyperdiskExtreme,
            "hyperdiskthroughput" => Self::HyperdiskThroughput,
            "hyperdiskml" | "ml" => Self::HyperdiskMl,
            _ => Self::PdStandard,
        }
    }
}

impl From<String> for DiskType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP disk prices.
pub struct DiskBuilder<'a> {
    client: &'a Client,
    disk_type: DiskType,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    // Volume specs for monthly cost calculation
    size_gb: Option<u64>,
    iops: Option<u64>,
    throughput_mb_per_sec: Option<u64>,
    // Regional disk (replicated across zones) = 2x price
    regional: bool,
}

impl<'a> DiskBuilder<'a> {
    /// Create a new disk builder
    pub(crate) fn new(client: &'a Client, disk_type: DiskType) -> Self {
        Self {
            client,
            disk_type,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
            iops: None,
            throughput_mb_per_sec: None,
            regional: false,
        }
    }

    /// Set the GCP region (e.g., "us-central1")
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

    /// Set the disk size in GB (required for `fetch_monthly`).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Set provisioned IOPS (for pd-extreme disks only).
    ///
    /// For pd-extreme: all provisioned IOPS are billed at $0.065/IOPS-month.
    /// For other disk types: IOPS is ignored (not supported).
    pub fn iops(mut self, iops: u64) -> Self {
        self.iops = Some(iops);
        self
    }

    /// Set whether this is a regional disk (replicated across zones).
    ///
    /// Regional disks cost 2x the price of zonal disks.
    pub fn regional(mut self, regional: bool) -> Self {
        self.regional = regional;
        self
    }

    /// Set provisioned throughput in MiB/s (for Hyperdisk types).
    ///
    /// Supported disk types and their throughput pricing:
    /// - Hyperdisk Balanced: $0.04/MiB/s-month
    /// - Hyperdisk Throughput: $0.25/MiB/s-month
    /// - Hyperdisk ML: $0.12/MiB/s-month
    ///
    /// For other disk types: throughput is ignored (not supported).
    pub fn throughput(mut self, mb_per_sec: u64) -> Self {
        self.throughput_mb_per_sec = Some(mb_per_sec);
        self
    }

    /// Fetch just the price value.
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let regional = self.regional;
        let resource = gcp_catalog().find(self.disk_type.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let mut result = PricingEngine::fetch(
            self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await?;
        if regional {
            result.price *= 2.0;
        }
        Ok(result)
    }

    /// Fetch total monthly cost based on disk specs.
    ///
    /// Requires `size_gb()` to be set. Optionally set:
    /// - `iops()` for pd-extreme and hyperdisk types with IOPS support
    /// - `throughput()` for hyperdisk types with throughput support
    ///
    /// The calculation:
    /// - Storage cost = storage_price x size_gb
    /// - IOPS cost = iops_price x iops (for supported types)
    /// - Throughput cost = throughput_price x throughput (for supported types)
    /// - Total = storage_cost + iops_cost + throughput_cost
    ///
    /// # Examples
    ///
    /// pd-extreme with IOPS:
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # use infracost_rs::providers::gcp::DiskType;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.gcp().disk(DiskType::PdExtreme)
    ///     .size_gb(500)
    ///     .iops(15000)
    ///     .fetch_monthly().await?;
    /// // Cost = (500 * $0.125) + (15000 * $0.065) = $62.5 + $975 = $1037.5/month
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Hyperdisk Throughput with throughput:
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # use infracost_rs::providers::gcp::DiskType;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.gcp().disk(DiskType::HyperdiskThroughput)
    ///     .size_gb(1000)
    ///     .throughput(500)  // 500 MiB/s
    ///     .fetch_monthly().await?;
    /// // Cost = (1000 * $0.005) + (500 * $0.25) = $5 + $125 = $130/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let size_gb = self
            .size_gb
            .ok_or_else(|| crate::Error::validation("size_gb is required for fetch_monthly"))?;
        let regional = self.regional;

        let resource = gcp_catalog().find(self.disk_type.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);

        let mut params = HashMap::new();
        params.insert("size_gb".to_string(), size_gb);
        if let Some(iops) = self.iops {
            params.insert("iops".to_string(), iops);
        }
        if let Some(throughput) = self.throughput_mb_per_sec {
            params.insert("throughput_mibps".to_string(), throughput);
        }

        let mut result = PricingEngine::fetch_monthly(
            self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            &params,
        )
        .await?;
        if regional {
            result.price *= 2.0;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_type_from_str() {
        assert_eq!(DiskType::from("pd-ssd"), DiskType::PdSsd);
        assert_eq!(DiskType::from("PD-SSD"), DiskType::PdSsd);
        assert_eq!(DiskType::from("ssd"), DiskType::PdSsd);
        assert_eq!(DiskType::from("pd-balanced"), DiskType::PdBalanced);
        assert_eq!(DiskType::from("pd-extreme"), DiskType::PdExtreme);
        assert_eq!(DiskType::from("pd-standard"), DiskType::PdStandard);
        assert_eq!(DiskType::from("unknown"), DiskType::PdStandard);
    }

    #[test]
    fn test_disk_type_defaults() {
        assert_eq!(DiskType::PdStandard.default_price(), 0.04);
        assert_eq!(DiskType::PdSsd.default_price(), 0.17);
        assert_eq!(DiskType::PdBalanced.default_price(), 0.10);
        assert_eq!(DiskType::PdExtreme.default_price(), 0.125);
    }

    #[test]
    fn test_disk_type_description() {
        assert_eq!(DiskType::PdStandard.description(), "Storage PD Capacity");
        assert_eq!(DiskType::PdSsd.description(), "SSD backed PD Capacity");
        assert_eq!(DiskType::PdBalanced.description(), "Balanced PD Capacity");
        assert_eq!(DiskType::PdExtreme.description(), "Extreme PD Capacity");
    }

    #[tokio::test]
    async fn test_disk_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.17);
        assert_eq!(result.unit, "GiB-month");
    }

    #[tokio::test]
    async fn test_disk_builder_override_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .override_default(0.20)
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.20);
    }

    #[tokio::test]
    async fn test_disk_builder_string_type() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk("pd-ssd")
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert_eq!(result.price, 0.17);
    }

    // ============================================================
    // fetch_monthly tests
    // ============================================================

    #[tokio::test]
    async fn test_pd_extreme_fetch_monthly_storage_only() {
        // 500 GB pd-extreme with no provisioned IOPS
        // Cost = 500 * $0.125 = $62.5/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 62.5);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_extreme_fetch_monthly_with_iops() {
        // 500 GB pd-extreme with 15000 provisioned IOPS
        // Cost = (500 * $0.125) + (15000 * $0.065)
        //      = $62.5 + $975 = $1037.5/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .size_gb(500)
            .iops(15000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 1037.5);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_extreme_fetch_monthly_small_iops() {
        // 100 GB pd-extreme with 1000 provisioned IOPS
        // Cost = (100 * $0.125) + (1000 * $0.065)
        //      = $12.5 + $65 = $77.5/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .size_gb(100)
            .iops(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 77.5);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_ssd_fetch_monthly_storage_only() {
        // pd-ssd has no provisioned IOPS - storage only
        // 500 GB * $0.17 = $85/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 85.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_ssd_iops_ignored() {
        // pd-ssd does not support IOPS - should be ignored
        // 500 GB * $0.17 = $85/month (IOPS ignored)
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .size_gb(500)
            .iops(10000) // should be ignored
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 85.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_balanced_fetch_monthly_storage_only() {
        // pd-balanced has no provisioned IOPS - storage only
        // 500 GB * $0.10 = $50/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdBalanced)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 50.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_balanced_iops_ignored() {
        // pd-balanced does not support IOPS - should be ignored
        // 500 GB * $0.10 = $50/month (IOPS ignored)
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdBalanced)
            .size_gb(500)
            .iops(10000) // should be ignored
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 50.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_standard_fetch_monthly_storage_only() {
        // pd-standard has no provisioned IOPS - storage only
        // 500 GB * $0.04 = $20/month
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdStandard)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 20.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_pd_standard_iops_ignored() {
        // pd-standard does not support IOPS - should be ignored
        // 500 GB * $0.04 = $20/month (IOPS ignored)
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdStandard)
            .size_gb(500)
            .iops(10000) // should be ignored
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 20.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_fetch_monthly_requires_size_gb() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .iops(15000)
            .fetch_monthly()
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_disk_type_supports_iops() {
        assert!(DiskType::PdExtreme.supports_iops());
        assert!(!DiskType::PdSsd.supports_iops());
        assert!(!DiskType::PdBalanced.supports_iops());
        assert!(!DiskType::PdStandard.supports_iops());
    }

    #[test]
    fn test_disk_type_default_iops_price() {
        assert_eq!(DiskType::PdExtreme.default_iops_price(), Some(0.065));
        assert_eq!(DiskType::PdSsd.default_iops_price(), None);
        assert_eq!(DiskType::PdBalanced.default_iops_price(), None);
        assert_eq!(DiskType::PdStandard.default_iops_price(), None);
    }

    #[test]
    fn test_disk_type_from_url() {
        assert_eq!(
            DiskType::from("projects/my-project/zones/us-central1-a/diskTypes/pd-ssd"),
            DiskType::PdSsd
        );
        assert_eq!(
            DiskType::from("projects/my-project/zones/us-central1-a/diskTypes/pd-balanced"),
            DiskType::PdBalanced
        );
        assert_eq!(
            DiskType::from("projects/my-project/zones/us-central1-a/diskTypes/pd-extreme"),
            DiskType::PdExtreme
        );
        assert_eq!(
            DiskType::from("projects/my-project/zones/us-central1-a/diskTypes/pd-standard"),
            DiskType::PdStandard
        );
        assert_eq!(
            DiskType::from("projects/my-project/zones/us-central1-a/diskTypes/hyperdisk-balanced"),
            DiskType::HyperdiskBalanced
        );
    }

    #[tokio::test]
    async fn test_regional_disk_doubles_price() {
        let client = Client::anonymous();
        let zonal = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        let regional = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .regional(true)
            .fetch()
            .await
            .unwrap();

        assert_eq!(regional.price, zonal.price * 2.0);
    }

    #[tokio::test]
    async fn test_regional_disk_monthly_doubles_price() {
        let client = Client::anonymous();
        let zonal = client
            .gcp()
            .disk(DiskType::PdSsd)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        let regional = client
            .gcp()
            .disk(DiskType::PdSsd)
            .size_gb(500)
            .regional(true)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(regional.price, zonal.price * 2.0);
        assert_eq!(regional.price, 170.0); // 500 * 0.17 * 2
    }
}
