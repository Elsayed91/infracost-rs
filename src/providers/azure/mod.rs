//! Azure resource pricing convenience API.
//!
//! # Example
//!
//! ```no_run
//! use infracost_rs::Client;
//! use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::anonymous();
//!
//! // Query managed disk pricing (P10 Premium SSD)
//! let price = client
//!     .azure()
//!     .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
//!     .region("eastus")
//!     .fetch_price()
//!     .await?;
//!
//! // Query snapshot pricing
//! let price = client
//!     .azure()
//!     .snapshot()
//!     .region("eastus")
//!     .fetch_price()
//!     .await?;
//!
//! // Query public IP pricing
//! let price = client
//!     .azure()
//!     .public_ip()
//!     .region("eastus")
//!     .fetch_price()
//!     .await?;
//! # Ok(())
//! # }
//! ```

mod managed_disk;
mod public_ip;
mod snapshot;

pub use managed_disk::{ManagedDiskBuilder, ManagedDiskSize, ManagedDiskType};
pub use public_ip::PublicIpBuilder;
pub use snapshot::SnapshotBuilder;

use crate::Client;

/// Azure provider for resource pricing queries.
pub struct AzureProvider<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> AzureProvider<'a> {
    /// Create a new Azure provider
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Query managed disk pricing.
    ///
    /// Azure managed disks use fixed size tiers (P1, P10, P30, etc.) with
    /// fixed monthly prices per disk.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .azure()
    ///     .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    ///     .region("eastus")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn managed_disk(
        self,
        disk_type: impl Into<ManagedDiskType>,
        size: impl Into<ManagedDiskSize>,
    ) -> ManagedDiskBuilder<'a> {
        ManagedDiskBuilder::new(self.client, disk_type.into(), size.into())
    }

    /// Query snapshot pricing.
    ///
    /// Returns the per-GB-month price for standard (HDD) snapshots.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .azure()
    ///     .snapshot()
    ///     .region("eastus")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn snapshot(self) -> SnapshotBuilder<'a> {
        SnapshotBuilder::new(self.client)
    }

    /// Query public IP pricing.
    ///
    /// Returns the per-hour price for a Standard static public IP.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .azure()
    ///     .public_ip()
    ///     .region("eastus")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn public_ip(self) -> PublicIpBuilder<'a> {
        PublicIpBuilder::new(self.client)
    }
}
