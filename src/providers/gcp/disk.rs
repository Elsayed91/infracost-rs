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

use crate::types::ProductFilter;
use crate::{Client, Result};

use super::super::{PriceResult, PriceSource};

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
}

impl DiskType {
    /// Get the description pattern for this disk type (used for API filtering)
    fn description(&self) -> &'static str {
        match self {
            Self::PdStandard => "Storage PD Capacity",
            Self::PdSsd => "SSD backed PD Capacity",
            Self::PdBalanced => "Balanced PD Capacity",
            Self::PdExtreme => "Extreme PD Capacity",
        }
    }

    /// Get the default storage price for this disk type (per GB-month)
    fn default_storage_price(&self) -> f64 {
        match self {
            Self::PdStandard => 0.04,
            Self::PdSsd => 0.17,
            Self::PdBalanced => 0.10,
            Self::PdExtreme => 0.125,
        }
    }

    /// Get the default price for this disk type (per GB-month)
    /// Alias for default_storage_price for backward compatibility
    fn default_price(&self) -> f64 {
        self.default_storage_price()
    }

    /// Get the unit for disk pricing
    fn unit(&self) -> &'static str {
        "GB-month"
    }

    /// Whether this disk type supports provisioned IOPS
    pub fn supports_iops(&self) -> bool {
        matches!(self, Self::PdExtreme)
    }

    /// Get the default IOPS price (per IOPS-month)
    /// Only pd-extreme supports provisioned IOPS
    pub fn default_iops_price(&self) -> Option<f64> {
        match self {
            Self::PdExtreme => Some(0.065),
            _ => None,
        }
    }
}

impl From<&str> for DiskType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().replace(['-', '_'], "").as_str() {
            "pdssd" | "ssd" => Self::PdSsd,
            "pdbalanced" | "balanced" => Self::PdBalanced,
            "pdextreme" | "extreme" => Self::PdExtreme,
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

    /// Fetch just the price value.
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    pub async fn fetch(self) -> Result<PriceResult> {
        let default_price = self
            .override_default
            .unwrap_or_else(|| self.disk_type.default_price());
        let unit = self.disk_type.unit();

        // Determine effective API key
        let effective_key = self.api_key.as_deref().or_else(|| {
            if self.client.has_api_key() {
                // Client has a key, we'll use it via query_products
                Some("")
            } else {
                None
            }
        });

        // No API key and not required → return default immediately
        if effective_key.is_none() && !self.client.error_on_fallback() {
            return Ok(PriceResult::from_default(default_price, unit));
        }

        // Try API
        let filter = self.build_filter();
        let api_key_for_query = self.api_key.as_deref();

        match self
            .client
            .query_products_with_key(filter, api_key_for_query)
            .await
        {
            Ok(products) if !products.is_empty() => {
                let price = products[0].first_nonzero_price_or(default_price);
                Ok(PriceResult::from_api(price, unit))
            }
            Ok(_) if !self.client.error_on_fallback() => {
                // No products found, use default
                Ok(PriceResult::from_default(default_price, unit))
            }
            Err(_) if !self.client.error_on_fallback() => {
                // API error, use default
                Ok(PriceResult::from_default(default_price, unit))
            }
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    /// Fetch total monthly cost based on disk specs.
    ///
    /// Requires `size_gb()` to be set. Optionally set `iops()` for pd-extreme disks.
    ///
    /// The calculation:
    /// - Storage cost = storage_price × size_gb
    /// - IOPS cost (pd-extreme only) = iops_price × iops
    /// - Total = storage_cost + iops_cost
    ///
    /// For non-extreme disk types, IOPS is ignored as they don't support provisioned IOPS.
    ///
    /// # Example
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
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let size_gb = self
            .size_gb
            .ok_or_else(|| crate::Error::validation("size_gb is required for fetch_monthly"))?;

        let region = self.region.as_deref().unwrap_or("us-central1");

        // Get price components
        let storage_price = self.fetch_storage_price(region).await?;
        let iops_price = self.fetch_iops_price(region).await?;

        // Calculate storage cost
        let storage_cost = size_gb as f64 * storage_price;

        // Calculate IOPS cost (only for pd-extreme)
        let iops_cost = if self.disk_type.supports_iops() {
            let provisioned_iops = self.iops.unwrap_or(0);
            provisioned_iops as f64 * iops_price.unwrap_or(0.0)
        } else {
            0.0
        };

        let total = storage_cost + iops_cost;

        // Determine source based on whether we got API prices
        let source = if self.client.has_api_key() || self.api_key.is_some() {
            PriceSource::Api
        } else {
            PriceSource::Default
        };

        Ok(PriceResult {
            price: total,
            unit: "month".to_string(),
            source,
        })
    }

    /// Fetch storage price per GB-month
    async fn fetch_storage_price(&self, region: &str) -> Result<f64> {
        let default = self.disk_type.default_storage_price();

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("Compute Engine")
            .region(region)
            .product_family("Storage")
            .attribute("description", self.disk_type.description())
            .build();

        match self
            .client
            .query_products_with_key(filter, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => Ok(products[0].first_nonzero_price_or(default)),
            _ if !self.client.error_on_fallback() => Ok(default),
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    /// Fetch IOPS price per IOPS-month (only for pd-extreme)
    async fn fetch_iops_price(&self, region: &str) -> Result<Option<f64>> {
        if !self.disk_type.supports_iops() {
            return Ok(None);
        }

        let default = self.disk_type.default_iops_price();

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        let filter = ProductFilter::builder()
            .vendor("gcp")
            .service("Compute Engine")
            .region(region)
            .product_family("Storage")
            .attribute("description", "Extreme PD IOPS")
            .build();

        match self
            .client
            .query_products_with_key(filter, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => Ok(Some(
                products[0].first_nonzero_price_or(default.unwrap_or(0.0)),
            )),
            _ if !self.client.error_on_fallback() => Ok(default),
            Err(e) => Err(e),
            Ok(_) => Ok(default),
        }
    }

    fn build_filter(&self) -> ProductFilter {
        ProductFilter::builder()
            .vendor("gcp")
            .service("Compute Engine")
            .region(self.region.as_deref().unwrap_or("us-central1"))
            .product_family("Storage")
            .attribute("description", self.disk_type.description())
            .build()
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
        assert_eq!(result.unit, "GB-month");
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
}
