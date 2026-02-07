//! Blocking Azure resource pricing convenience API.
//!
//! # Example
//!
//! ```no_run
//! use infracost_rs::blocking::Client;
//! use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};
//!
//! fn example() -> Result<(), infracost_rs::Error> {
//!     let client = Client::anonymous();
//!
//!     // Query managed disk pricing (P10 Premium SSD)
//!     let price = client
//!         .azure()
//!         .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
//!         .region("eastus")
//!         .fetch_price()?;
//!
//!     // Query snapshot pricing
//!     let price = client
//!         .azure()
//!         .snapshot()
//!         .region("eastus")
//!         .fetch_price()?;
//!
//!     // Query public IP pricing
//!     let price = client
//!         .azure()
//!         .public_ip()
//!         .region("eastus")
//!         .fetch_price()?;
//!
//!     Ok(())
//! }
//! ```

use crate::error::Result;
use crate::providers::PriceResult;
use crate::providers::azure::{ManagedDiskSize, ManagedDiskType};
use std::sync::Arc;

/// Blocking Azure provider for resource pricing queries.
pub struct BlockingAzureProvider {
    pub(crate) client: crate::Client,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

impl BlockingAzureProvider {
    /// Query managed disk pricing.
    ///
    /// Azure managed disks use fixed size tiers (P1, P10, P30, etc.) with
    /// fixed monthly prices per disk.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    /// use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};
    ///
    /// fn example() -> Result<(), infracost_rs::Error> {
    ///     let client = Client::anonymous();
    ///     let price = client
    ///         .azure()
    ///         .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    ///         .region("eastus")
    ///         .fetch_price()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn managed_disk(
        self,
        disk_type: impl Into<ManagedDiskType>,
        size: impl Into<ManagedDiskSize>,
    ) -> BlockingAzureManagedDiskBuilder {
        BlockingAzureManagedDiskBuilder {
            client: self.client,
            runtime: self.runtime,
            disk_type: disk_type.into(),
            size: size.into(),
            region: None,
            api_key: None,
            override_default: None,
        }
    }

    /// Query snapshot pricing.
    ///
    /// Returns the per-GB-month price for standard (HDD) snapshots.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    ///
    /// fn example() -> Result<(), infracost_rs::Error> {
    ///     let client = Client::anonymous();
    ///     let price = client
    ///         .azure()
    ///         .snapshot()
    ///         .region("eastus")
    ///         .fetch_price()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn snapshot(self) -> BlockingAzureSnapshotBuilder {
        BlockingAzureSnapshotBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
        }
    }

    /// Query public IP pricing.
    ///
    /// Returns the per-hour price for a Standard static public IP.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    ///
    /// fn example() -> Result<(), infracost_rs::Error> {
    ///     let client = Client::anonymous();
    ///     let price = client
    ///         .azure()
    ///         .public_ip()
    ///         .region("eastus")
    ///         .fetch_price()?;
    ///     Ok(())
    /// }
    /// ```
    pub fn public_ip(self) -> BlockingAzurePublicIpBuilder {
        BlockingAzurePublicIpBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
        }
    }

    /// Create a managed disk builder from Azure CLI JSON output (`az disk show --output json`).
    pub fn managed_disk_from_json(
        self,
        json: &serde_json::Value,
    ) -> Result<BlockingAzureManagedDiskBuilder> {
        let parsed = crate::providers::azure::from_json::parse_managed_disk_json(json)?;
        let mut builder = BlockingAzureManagedDiskBuilder {
            client: self.client,
            runtime: self.runtime,
            disk_type: parsed.disk_type,
            size: parsed.size,
            region: None,
            api_key: None,
            override_default: None,
        };
        if let Some(r) = parsed.region {
            builder.region = Some(r);
        }
        Ok(builder)
    }

    /// Create a snapshot builder from Azure CLI JSON output (`az snapshot show --output json`).
    pub fn snapshot_from_json(
        self,
        json: &serde_json::Value,
    ) -> Result<BlockingAzureSnapshotBuilder> {
        let parsed = crate::providers::azure::from_json::parse_snapshot_json(json)?;
        let mut builder = BlockingAzureSnapshotBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
        };
        if let Some(r) = parsed.region {
            builder.region = Some(r);
        }
        if let Some(s) = parsed.size_gb {
            builder.size_gb = Some(s);
        }
        Ok(builder)
    }

    /// Create a public IP builder from Azure CLI JSON output (`az network public-ip show --output json`).
    pub fn public_ip_from_json(
        self,
        json: &serde_json::Value,
    ) -> Result<BlockingAzurePublicIpBuilder> {
        let parsed = crate::providers::azure::from_json::parse_public_ip_json(json)?;
        let mut builder = BlockingAzurePublicIpBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
        };
        if let Some(r) = parsed.region {
            builder.region = Some(r);
        }
        Ok(builder)
    }
}

// ============================================================
// Managed Disk Builder
// ============================================================

/// Blocking builder for querying Azure Managed Disk prices.
pub struct BlockingAzureManagedDiskBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    disk_type: ManagedDiskType,
    size: ManagedDiskSize,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl BlockingAzureManagedDiskBuilder {
    /// Set the Azure region (e.g., "eastus", "westus2").
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

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.azure().managed_disk(self.disk_type, self.size);
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        self.runtime.block_on(b.fetch())
    }

    /// Fetch just the price value.
    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
    }

    /// Fetch the monthly price.
    ///
    /// For managed disks, this is an alias for `fetch()` since Azure managed disks
    /// are already priced on a monthly basis.
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.azure().managed_disk(self.disk_type, self.size);
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        self.runtime.block_on(b.fetch_monthly())
    }
}

// ============================================================
// Snapshot Builder
// ============================================================

/// Blocking builder for querying Azure Snapshot prices.
///
/// Returns the per-GB-month price for Standard (HDD) snapshots.
pub struct BlockingAzureSnapshotBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
}

impl BlockingAzureSnapshotBuilder {
    /// Set the Azure region (e.g., "eastus", "westus2").
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

    /// Set the snapshot size in GB (required for fetch_monthly).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.azure().snapshot();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        if let Some(v) = self.size_gb {
            b = b.size_gb(v);
        }
        self.runtime.block_on(b.fetch())
    }

    /// Fetch just the price value.
    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
    }

    /// Fetch the monthly price (rate per GB-month * size_gb).
    ///
    /// This is a convenience method for calculating monthly costs.
    /// Requires size_gb to be set.
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.azure().snapshot();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        if let Some(v) = self.size_gb {
            b = b.size_gb(v);
        }
        self.runtime.block_on(b.fetch_monthly())
    }
}

// ============================================================
// Public IP Builder
// ============================================================

/// Blocking builder for querying Azure Public IP prices.
///
/// Returns the per-hour price for a Standard static public IPv4 address.
pub struct BlockingAzurePublicIpBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl BlockingAzurePublicIpBuilder {
    /// Set the Azure region (e.g., "eastus", "westus2").
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

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.azure().public_ip();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        self.runtime.block_on(b.fetch())
    }

    /// Fetch just the price value.
    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
    }

    /// Fetch the monthly price (hourly price * 730 hours).
    ///
    /// This is a convenience method for calculating monthly costs.
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.azure().public_ip();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        self.runtime.block_on(b.fetch_monthly())
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::blocking::Client;
    use crate::providers::azure::{ManagedDiskSize, ManagedDiskType};

    // ========== Managed Disk Tests ==========

    #[test]
    fn test_blocking_azure_managed_disk_default() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 19.71);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_managed_disk_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 19.71);
    }

    #[test]
    fn test_blocking_azure_managed_disk_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .fetch_monthly()
            .unwrap();

        // fetch_monthly is alias for fetch for managed disks
        assert_eq!(result.price, 19.71);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_managed_disk_standard_ssd() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::StandardSsd, ManagedDiskSize::E10)
            .region("westus")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 9.60);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_managed_disk_standard_hdd() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::StandardHdd, ManagedDiskSize::S10)
            .region("westus")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 5.89);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_managed_disk_override_default() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .override_default(25.00)
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 25.00);
        assert_eq!(result.unit, "month");
    }

    // ========== Snapshot Tests ==========

    #[test]
    fn test_blocking_azure_snapshot_default() {
        let client = Client::anonymous();
        let result = client.azure().snapshot().region("eastus").fetch().unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
        assert_eq!(result.unit, "GB-month");
    }

    #[test]
    fn test_blocking_azure_snapshot_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .azure()
            .snapshot()
            .region("eastus")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.05);
    }

    #[test]
    fn test_blocking_azure_snapshot_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .snapshot()
            .region("eastus")
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        assert!(result.is_from_default());
        // 0.05 × 100 = 5.00
        assert_eq!(result.price, 5.00);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_snapshot_fetch_monthly_large_size() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .snapshot()
            .region("eastus")
            .size_gb(1024)
            .fetch_monthly()
            .unwrap();

        assert!(result.is_from_default());
        // 0.05 × 1024 = 51.20
        assert_eq!(result.price, 51.20);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_snapshot_fetch_monthly_requires_size() {
        let client = Client::anonymous();
        let result = client.azure().snapshot().region("eastus").fetch_monthly();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("size_gb is required"));
    }

    #[test]
    fn test_blocking_azure_snapshot_override_default() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .snapshot()
            .region("eastus")
            .override_default(0.10)
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        assert!(result.is_from_default());
        // 0.10 × 100 = 10.00
        assert_eq!(result.price, 10.00);
        assert_eq!(result.unit, "month");
    }

    // ========== Public IP Tests ==========

    #[test]
    fn test_blocking_azure_public_ip_default() {
        let client = Client::anonymous();
        let result = client.azure().public_ip().region("eastus").fetch().unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.005);
        assert_eq!(result.unit, "hour");
    }

    #[test]
    fn test_blocking_azure_public_ip_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .azure()
            .public_ip()
            .region("eastus")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.005);
    }

    #[test]
    fn test_blocking_azure_public_ip_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .public_ip()
            .region("eastus")
            .fetch_monthly()
            .unwrap();

        assert!(result.is_from_default());
        // 0.005 × 730 = 3.65
        assert_eq!(result.price, 3.65);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_public_ip_override_default() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .public_ip()
            .region("eastus")
            .override_default(0.01)
            .fetch_monthly()
            .unwrap();

        assert!(result.is_from_default());
        // 0.01 × 730 = 7.30
        assert_eq!(result.price, 7.30);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_azure_public_ip_different_region() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .public_ip()
            .region("westus2")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.005);
        assert_eq!(result.unit, "hour");
    }

    // ========== Builder Chaining Tests ==========

    #[test]
    fn test_blocking_azure_builder_chaining() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .api_key("test-key")
            .override_default(20.00)
            .fetch()
            .unwrap();

        // With override, should use that value
        assert_eq!(result.price, 20.00);
    }
}
