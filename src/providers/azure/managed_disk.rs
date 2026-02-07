//! Azure Managed Disk pricing.

use crate::catalog::{azure_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Types
// ============================================================

/// Azure managed disk types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedDiskType {
    /// Premium SSD (P-series) - high performance SSDs
    PremiumSsd,
    /// Standard SSD (E-series) - balanced price/performance
    StandardSsd,
    /// Standard HDD (S-series) - lowest cost
    StandardHdd,
}

impl ManagedDiskType {
    /// Parse Azure SKU name to ManagedDiskType.
    ///
    /// Handles: "Premium_LRS", "PremiumV2_LRS", "StandardSSD_LRS", "Standard_LRS", "UltraSSD_LRS"
    pub fn from_sku_name(sku: &str) -> crate::Result<Self> {
        match sku {
            "Premium_LRS" | "PremiumV2_LRS" => Ok(Self::PremiumSsd),
            "StandardSSD_LRS" => Ok(Self::StandardSsd),
            "Standard_LRS" => Ok(Self::StandardHdd),
            "UltraSSD_LRS" => Ok(Self::PremiumSsd), // closest match
            _ => Err(crate::Error::validation(format!(
                "Unknown Azure SKU: {}",
                sku
            ))),
        }
    }
}

impl std::str::FromStr for ManagedDiskType {
    type Err = crate::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "premium_ssd" | "premiumssd" | "premium-ssd" | "p" => Ok(Self::PremiumSsd),
            "standard_ssd" | "standardssd" | "standard-ssd" | "e" => Ok(Self::StandardSsd),
            "standard_hdd" | "standardhdd" | "standard-hdd" | "s" => Ok(Self::StandardHdd),
            _ => Err(crate::Error::config(format!(
                "Unknown Azure disk type: {}",
                s
            ))),
        }
    }
}

impl From<&str> for ManagedDiskType {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Self::PremiumSsd)
    }
}

/// Azure managed disk sizes.
///
/// Each size has a fixed capacity and monthly price.
/// Premium SSD uses P-series, Standard SSD uses E-series, Standard HDD uses S-series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedDiskSize {
    // Premium SSD sizes (P-series)
    /// 4 GB
    P1,
    /// 8 GB
    P2,
    /// 16 GB
    P3,
    /// 32 GB
    P4,
    /// 64 GB
    P6,
    /// 128 GB
    P10,
    /// 256 GB
    P15,
    /// 512 GB
    P20,
    /// 1 TB
    P30,
    /// 2 TB
    P40,
    /// 4 TB
    P50,

    // Standard SSD sizes (E-series)
    /// 4 GB
    E1,
    /// 8 GB
    E2,
    /// 16 GB
    E3,
    /// 32 GB
    E4,
    /// 64 GB
    E6,
    /// 128 GB
    E10,
    /// 256 GB
    E15,
    /// 512 GB
    E20,
    /// 1 TB
    E30,
    /// 2 TB
    E40,
    /// 4 TB
    E50,

    // Standard HDD sizes (S-series)
    /// 32 GB
    S4,
    /// 64 GB
    S6,
    /// 128 GB
    S10,
    /// 256 GB
    S15,
    /// 512 GB
    S20,
    /// 1 TB
    S30,
    /// 2 TB
    S40,
    /// 4 TB
    S50,
}

impl ManagedDiskSize {
    /// Returns the SKU name prefix for this size (e.g., "P10" for Premium P10)
    fn sku_prefix(&self) -> &'static str {
        match self {
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
            Self::P4 => "P4",
            Self::P6 => "P6",
            Self::P10 => "P10",
            Self::P15 => "P15",
            Self::P20 => "P20",
            Self::P30 => "P30",
            Self::P40 => "P40",
            Self::P50 => "P50",
            Self::E1 => "E1",
            Self::E2 => "E2",
            Self::E3 => "E3",
            Self::E4 => "E4",
            Self::E6 => "E6",
            Self::E10 => "E10",
            Self::E15 => "E15",
            Self::E20 => "E20",
            Self::E30 => "E30",
            Self::E40 => "E40",
            Self::E50 => "E50",
            Self::S4 => "S4",
            Self::S6 => "S6",
            Self::S10 => "S10",
            Self::S15 => "S15",
            Self::S20 => "S20",
            Self::S30 => "S30",
            Self::S40 => "S40",
            Self::S50 => "S50",
        }
    }

    /// Map a disk size in GB to the smallest tier that can accommodate it.
    ///
    /// For Premium SSD: maps to P-series (4->P1, 8->P2, ..., 4096->P50)
    /// For Standard SSD: maps to E-series (4->E1, 8->E2, ..., 4096->E50)
    /// For Standard HDD: maps to S-series (no S1/S2/S3, starts at 32->S4)
    pub fn from_size_gb(disk_type: ManagedDiskType, gb: u64) -> crate::Result<Self> {
        match disk_type {
            ManagedDiskType::PremiumSsd => match gb {
                0..=4 => Ok(Self::P1),
                5..=8 => Ok(Self::P2),
                9..=16 => Ok(Self::P3),
                17..=32 => Ok(Self::P4),
                33..=64 => Ok(Self::P6),
                65..=128 => Ok(Self::P10),
                129..=256 => Ok(Self::P15),
                257..=512 => Ok(Self::P20),
                513..=1024 => Ok(Self::P30),
                1025..=2048 => Ok(Self::P40),
                2049..=4096 => Ok(Self::P50),
                _ => Err(crate::Error::validation(format!(
                    "Disk size {} GB exceeds maximum P50 tier (4096 GB)",
                    gb
                ))),
            },
            ManagedDiskType::StandardSsd => match gb {
                0..=4 => Ok(Self::E1),
                5..=8 => Ok(Self::E2),
                9..=16 => Ok(Self::E3),
                17..=32 => Ok(Self::E4),
                33..=64 => Ok(Self::E6),
                65..=128 => Ok(Self::E10),
                129..=256 => Ok(Self::E15),
                257..=512 => Ok(Self::E20),
                513..=1024 => Ok(Self::E30),
                1025..=2048 => Ok(Self::E40),
                2049..=4096 => Ok(Self::E50),
                _ => Err(crate::Error::validation(format!(
                    "Disk size {} GB exceeds maximum E50 tier (4096 GB)",
                    gb
                ))),
            },
            ManagedDiskType::StandardHdd => match gb {
                0..=32 => Ok(Self::S4),
                33..=64 => Ok(Self::S6),
                65..=128 => Ok(Self::S10),
                129..=256 => Ok(Self::S15),
                257..=512 => Ok(Self::S20),
                513..=1024 => Ok(Self::S30),
                1025..=2048 => Ok(Self::S40),
                2049..=4096 => Ok(Self::S50),
                _ => Err(crate::Error::validation(format!(
                    "Disk size {} GB exceeds maximum S50 tier (4096 GB)",
                    gb
                ))),
            },
        }
    }

    /// Returns the catalog resource name for this size and disk type.
    fn resource_name(&self, disk_type: &ManagedDiskType) -> String {
        let type_prefix = match disk_type {
            ManagedDiskType::PremiumSsd => "premium-ssd",
            ManagedDiskType::StandardSsd => "standard-ssd",
            ManagedDiskType::StandardHdd => "standard-hdd",
        };
        format!(
            "managed-disk/{}/{}",
            type_prefix,
            self.sku_prefix().to_lowercase()
        )
    }
}

impl std::str::FromStr for ManagedDiskSize {
    type Err = crate::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "P1" => Ok(Self::P1),
            "P2" => Ok(Self::P2),
            "P3" => Ok(Self::P3),
            "P4" => Ok(Self::P4),
            "P6" => Ok(Self::P6),
            "P10" => Ok(Self::P10),
            "P15" => Ok(Self::P15),
            "P20" => Ok(Self::P20),
            "P30" => Ok(Self::P30),
            "P40" => Ok(Self::P40),
            "P50" => Ok(Self::P50),
            "E1" => Ok(Self::E1),
            "E2" => Ok(Self::E2),
            "E3" => Ok(Self::E3),
            "E4" => Ok(Self::E4),
            "E6" => Ok(Self::E6),
            "E10" => Ok(Self::E10),
            "E15" => Ok(Self::E15),
            "E20" => Ok(Self::E20),
            "E30" => Ok(Self::E30),
            "E40" => Ok(Self::E40),
            "E50" => Ok(Self::E50),
            "S4" => Ok(Self::S4),
            "S6" => Ok(Self::S6),
            "S10" => Ok(Self::S10),
            "S15" => Ok(Self::S15),
            "S20" => Ok(Self::S20),
            "S30" => Ok(Self::S30),
            "S40" => Ok(Self::S40),
            "S50" => Ok(Self::S50),
            _ => Err(crate::Error::config(format!(
                "Unknown Azure disk size: {}",
                s
            ))),
        }
    }
}

impl From<&str> for ManagedDiskSize {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Self::P10)
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying Azure Managed Disk prices.
pub struct ManagedDiskBuilder<'a> {
    client: &'a Client,
    disk_type: ManagedDiskType,
    size: ManagedDiskSize,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl<'a> ManagedDiskBuilder<'a> {
    /// Create a new managed disk builder
    pub(crate) fn new(
        client: &'a Client,
        disk_type: ManagedDiskType,
        size: ManagedDiskSize,
    ) -> Self {
        Self {
            client,
            disk_type,
            size,
            region: None,
            api_key: None,
            override_default: None,
        }
    }

    /// Set the Azure region (e.g., "eastus", "westus2")
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

    /// Fetch the monthly price.
    ///
    /// For managed disks, this is an alias for `fetch()` since Azure managed disks
    /// are already priced on a monthly basis.
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        self.fetch().await
    }

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource_name = self.size.resource_name(&self.disk_type);
        let resource = azure_catalog().find(&resource_name)?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            self.client,
            resource,
            "azure",
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

    #[test]
    fn test_managed_disk_type_from_str() {
        assert_eq!(
            "premium_ssd".parse::<ManagedDiskType>().unwrap(),
            ManagedDiskType::PremiumSsd
        );
        assert_eq!(
            "standard_ssd".parse::<ManagedDiskType>().unwrap(),
            ManagedDiskType::StandardSsd
        );
        assert_eq!(
            "standard_hdd".parse::<ManagedDiskType>().unwrap(),
            ManagedDiskType::StandardHdd
        );
    }

    #[test]
    fn test_managed_disk_size_from_str() {
        assert_eq!(
            "P10".parse::<ManagedDiskSize>().unwrap(),
            ManagedDiskSize::P10
        );
        assert_eq!(
            "E30".parse::<ManagedDiskSize>().unwrap(),
            ManagedDiskSize::E30
        );
        assert_eq!(
            "S50".parse::<ManagedDiskSize>().unwrap(),
            ManagedDiskSize::S50
        );
    }

    #[test]
    fn test_from_sku_name_premium() {
        assert_eq!(
            ManagedDiskType::from_sku_name("Premium_LRS").unwrap(),
            ManagedDiskType::PremiumSsd
        );
        assert_eq!(
            ManagedDiskType::from_sku_name("PremiumV2_LRS").unwrap(),
            ManagedDiskType::PremiumSsd
        );
    }

    #[test]
    fn test_from_sku_name_standard_ssd() {
        assert_eq!(
            ManagedDiskType::from_sku_name("StandardSSD_LRS").unwrap(),
            ManagedDiskType::StandardSsd
        );
    }

    #[test]
    fn test_from_sku_name_standard_hdd() {
        assert_eq!(
            ManagedDiskType::from_sku_name("Standard_LRS").unwrap(),
            ManagedDiskType::StandardHdd
        );
    }

    #[test]
    fn test_from_sku_name_ultra() {
        assert_eq!(
            ManagedDiskType::from_sku_name("UltraSSD_LRS").unwrap(),
            ManagedDiskType::PremiumSsd
        );
    }

    #[test]
    fn test_from_sku_name_invalid() {
        assert!(ManagedDiskType::from_sku_name("Unknown_LRS").is_err());
    }

    #[test]
    fn test_from_size_gb_premium_boundaries() {
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::PremiumSsd, 4).unwrap(),
            ManagedDiskSize::P1
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::PremiumSsd, 5).unwrap(),
            ManagedDiskSize::P2
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::PremiumSsd, 128).unwrap(),
            ManagedDiskSize::P10
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::PremiumSsd, 129).unwrap(),
            ManagedDiskSize::P15
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::PremiumSsd, 4096).unwrap(),
            ManagedDiskSize::P50
        );
    }

    #[test]
    fn test_from_size_gb_premium_exceeds_max() {
        assert!(ManagedDiskSize::from_size_gb(ManagedDiskType::PremiumSsd, 5000).is_err());
    }

    #[test]
    fn test_from_size_gb_standard_ssd() {
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::StandardSsd, 4).unwrap(),
            ManagedDiskSize::E1
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::StandardSsd, 128).unwrap(),
            ManagedDiskSize::E10
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::StandardSsd, 4096).unwrap(),
            ManagedDiskSize::E50
        );
    }

    #[test]
    fn test_from_size_gb_standard_hdd() {
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::StandardHdd, 32).unwrap(),
            ManagedDiskSize::S4
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::StandardHdd, 33).unwrap(),
            ManagedDiskSize::S6
        );
        assert_eq!(
            ManagedDiskSize::from_size_gb(ManagedDiskType::StandardHdd, 4096).unwrap(),
            ManagedDiskSize::S50
        );
    }

    #[tokio::test]
    async fn test_managed_disk_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 19.71);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_managed_disk_fetch_monthly() {
        let client = Client::anonymous();

        // Test that fetch_monthly returns the same result as fetch
        let fetch_result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .fetch()
            .await
            .unwrap();

        let monthly_result = client
            .azure()
            .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
            .region("eastus")
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(fetch_result.price, monthly_result.price);
        assert_eq!(fetch_result.unit, monthly_result.unit);
        assert_eq!(monthly_result.price, 19.71);
        assert_eq!(monthly_result.unit, "month");
    }
}
