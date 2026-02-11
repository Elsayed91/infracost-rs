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

use crate::providers::azure::{ManagedDiskSize, ManagedDiskType};
use std::sync::Arc;

/// Blocking Azure provider for resource pricing queries.
pub struct BlockingAzureProvider {
    pub(crate) client: crate::Client,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

impl BlockingAzureProvider {
    /// Query managed disk pricing.
    pub fn managed_disk(
        self,
        disk_type: impl Into<ManagedDiskType>,
        size: impl Into<ManagedDiskSize>,
    ) -> BlockingAzureManagedDiskBuilder {
        BlockingAzureManagedDiskBuilder {
            inner: self.client.azure().managed_disk(disk_type, size),
            runtime: self.runtime,
        }
    }

    /// Create a managed disk builder from Azure CLI JSON output (`az disk show --output json`).
    pub fn managed_disk_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAzureManagedDiskBuilder> {
        let parsed = crate::providers::azure::from_json::parse_managed_disk_json(json)?;
        let mut b = self
            .client
            .azure()
            .managed_disk(parsed.disk_type, parsed.size);
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingAzureManagedDiskBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query snapshot pricing.
    pub fn snapshot(self) -> BlockingAzureSnapshotBuilder {
        BlockingAzureSnapshotBuilder {
            inner: self.client.azure().snapshot(),
            runtime: self.runtime,
        }
    }

    /// Create a snapshot builder from Azure CLI JSON output (`az snapshot show --output json`).
    pub fn snapshot_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAzureSnapshotBuilder> {
        let parsed = crate::providers::azure::from_json::parse_snapshot_json(json)?;
        let mut b = self.client.azure().snapshot();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        if let Some(s) = parsed.size_gb {
            b = b.size_gb(s);
        }
        Ok(BlockingAzureSnapshotBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query public IP pricing.
    pub fn public_ip(self) -> BlockingAzurePublicIpBuilder {
        BlockingAzurePublicIpBuilder {
            inner: self.client.azure().public_ip(),
            runtime: self.runtime,
        }
    }

    /// Create a public IP builder from Azure CLI JSON output (`az network public-ip show --output json`).
    pub fn public_ip_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAzurePublicIpBuilder> {
        let parsed = crate::providers::azure::from_json::parse_public_ip_json(json)?;
        let mut b = self.client.azure().public_ip();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingAzurePublicIpBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }
}

// ============================================================
// Blocking Builders (generated via macro)
// ============================================================

blocking_builder! {
    /// Blocking builder for querying Azure Managed Disk prices.
    pub struct BlockingAzureManagedDiskBuilder wraps crate::providers::azure::ManagedDiskBuilder {
    }
}

blocking_builder! {
    /// Blocking builder for querying Azure Snapshot prices.
    pub struct BlockingAzureSnapshotBuilder wraps crate::providers::azure::SnapshotBuilder {
        fn size_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying Azure Public IP prices.
    pub struct BlockingAzurePublicIpBuilder wraps crate::providers::azure::PublicIpBuilder {
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
