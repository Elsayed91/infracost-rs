//! Azure Managed Disk pricing.

use crate::types::ProductFilter;
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
    /// Returns the API product name for this disk type
    fn product_name(&self) -> &'static str {
        match self {
            Self::PremiumSsd => "Premium SSD Managed Disks",
            Self::StandardSsd => "Standard SSD Managed Disks",
            Self::StandardHdd => "Standard HDD Managed Disks",
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

    /// Returns the SKU name for LRS (Locally Redundant Storage)
    fn sku_name_lrs(&self) -> String {
        format!("{} LRS", self.sku_prefix())
    }

    /// Returns the meter name for the disk
    fn meter_name(&self) -> String {
        format!("{} LRS Disk", self.sku_prefix())
    }

    /// Returns the default price for this disk size (USD/month)
    fn default_price(&self) -> f64 {
        match self {
            // Premium SSD prices (eastus, as of 2024)
            Self::P1 => 0.60,
            Self::P2 => 1.20,
            Self::P3 => 2.40,
            Self::P4 => 4.80,
            Self::P6 => 9.60,
            Self::P10 => 19.71,
            Self::P15 => 38.02,
            Self::P20 => 73.22,
            Self::P30 => 135.17,
            Self::P40 => 259.05,
            Self::P50 => 496.91,

            // Standard SSD prices (eastus, as of 2024)
            Self::E1 => 0.30,
            Self::E2 => 0.60,
            Self::E3 => 1.20,
            Self::E4 => 2.40,
            Self::E6 => 4.80,
            Self::E10 => 9.60,
            Self::E15 => 19.20,
            Self::E20 => 38.40,
            Self::E30 => 76.80,
            Self::E40 => 153.60,
            Self::E50 => 307.20,

            // Standard HDD prices (eastus, as of 2024)
            Self::S4 => 1.54,
            Self::S6 => 3.01,
            Self::S10 => 5.89,
            Self::S15 => 11.33,
            Self::S20 => 21.76,
            Self::S30 => 40.96,
            Self::S40 => 77.82,
            Self::S50 => 143.36,
        }
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

const UNIT: &str = "month";

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
        let default_price = self.override_default.unwrap_or(self.size.default_price());

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
                // Filter for Consumption prices (not Reservation)
                let price = products[0]
                    .prices()
                    .purchase_option("Consumption")
                    .first_nonzero_f64_or(default_price);
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
            .vendor("azure")
            .region(self.region.as_deref().unwrap_or("eastus"))
            .attribute("productName", self.disk_type.product_name())
            .attribute("skuName", self.size.sku_name_lrs())
            .attribute("meterName", self.size.meter_name())
            .build()
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
    fn test_managed_disk_defaults() {
        assert_eq!(ManagedDiskSize::P10.default_price(), 19.71);
        assert_eq!(ManagedDiskSize::E10.default_price(), 9.60);
        assert_eq!(ManagedDiskSize::S10.default_price(), 5.89);
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
