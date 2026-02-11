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

    /// Query snapshot pricing.
    pub fn snapshot(self) -> BlockingAzureSnapshotBuilder {
        BlockingAzureSnapshotBuilder {
            inner: self.client.azure().snapshot(),
            runtime: self.runtime,
        }
    }

    /// Query public IP pricing.
    pub fn public_ip(self) -> BlockingAzurePublicIpBuilder {
        BlockingAzurePublicIpBuilder {
            inner: self.client.azure().public_ip(),
            runtime: self.runtime,
        }
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

    /// Smoke test: verify blocking_builder! macro works for all Azure builders.
    /// Detailed assertions are in the async builder tests.
    #[test]
    fn test_blocking_azure_smoke() {
        let client = Client::anonymous();

        // Managed Disk builder - test all fetch methods
        let _ = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .fetch()
            .unwrap();
        let _ = client
            .azure()
            .managed_disk(ManagedDiskType::StandardSsd, ManagedDiskSize::E10)
            .fetch_price()
            .unwrap();
        let _ = client
            .azure()
            .managed_disk(ManagedDiskType::StandardHdd, ManagedDiskSize::S10)
            .fetch_monthly()
            .unwrap();

        // Snapshot builder
        let _ = client.azure().snapshot().region("eastus").fetch().unwrap();
        let _ = client
            .azure()
            .snapshot()
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        // Public IP builder
        let _ = client.azure().public_ip().region("eastus").fetch().unwrap();
        let _ = client.azure().public_ip().fetch_monthly().unwrap();
    }

    /// Verify override_default works in blocking wrappers.
    #[test]
    fn test_blocking_azure_override() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .snapshot()
            .override_default(0.10)
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        assert_eq!(result.price, 10.00);
        assert_eq!(result.unit, "month");
    }
}
