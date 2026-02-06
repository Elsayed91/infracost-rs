//! AWS EBS volume pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//!
//! # Per-unit pricing (original behavior)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.aws().ebs("gp3").fetch().await?;
//! println!("${}/GB-month", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with specs
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.aws().ebs("gp3")
//!     .size_gb(500)
//!     .iops(6000)           // 3000 extra billable (baseline is 3000)
//!     .throughput_mibps(250) // 125 extra billable (baseline is 125)
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

/// AWS EBS volume types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbsType {
    /// General Purpose SSD (gp3) - baseline 3000 IOPS
    Gp3,
    /// General Purpose SSD (gp2) - burstable IOPS
    Gp2,
    /// Provisioned IOPS SSD (io2) - high performance with tiered IOPS pricing
    /// - Tier 1: 1-32,000 IOPS at $0.065/IOPS-Mo
    /// - Tier 2: 32,001-64,000 IOPS at $0.0455/IOPS-Mo
    /// - Tier 3: 64,001+ IOPS at $0.03185/IOPS-Mo
    Io2,
    /// Throughput Optimized HDD (st1) - low cost, frequently accessed
    St1,
    /// Cold HDD (sc1) - lowest cost, infrequently accessed
    Sc1,
}

impl EbsType {
    /// Get the volume API name for this EBS type
    fn volume_api_name(&self) -> &'static str {
        match self {
            Self::Gp3 => "gp3",
            Self::Gp2 => "gp2",
            Self::Io2 => "io2",
            Self::St1 => "st1",
            Self::Sc1 => "sc1",
        }
    }

    /// Get the default storage price for this EBS type (per GB-month)
    fn default_storage_price(&self) -> f64 {
        match self {
            Self::Gp3 => 0.08,
            Self::Gp2 => 0.10,
            Self::Io2 => 0.125,
            Self::St1 => 0.045,
            Self::Sc1 => 0.015,
        }
    }

    /// Get the default price for this EBS type (per GB-month)
    /// Alias for default_storage_price for backward compatibility
    fn default_price(&self) -> f64 {
        self.default_storage_price()
    }

    /// Get the default IOPS price (per IOPS-month)
    fn default_iops_price(&self) -> Option<f64> {
        match self {
            Self::Gp3 => Some(0.005),
            Self::Io2 => Some(0.065), // tier 1 price
            _ => None,
        }
    }

    /// Get the default throughput price (per MiBps-month)
    fn default_throughput_price(&self) -> Option<f64> {
        match self {
            Self::Gp3 => Some(0.04),
            _ => None,
        }
    }

    /// Get the baseline IOPS (included in storage price)
    fn baseline_iops(&self) -> u64 {
        match self {
            Self::Gp3 => 3000,
            _ => 0,
        }
    }

    /// Get the baseline throughput in MiBps (included in storage price)
    fn baseline_throughput_mibps(&self) -> u64 {
        match self {
            Self::Gp3 => 125,
            _ => 0,
        }
    }

    /// Whether this volume type supports provisioned IOPS
    fn supports_iops(&self) -> bool {
        matches!(self, Self::Gp3 | Self::Io2)
    }

    /// Whether this volume type supports provisioned throughput
    fn supports_throughput(&self) -> bool {
        matches!(self, Self::Gp3)
    }

    /// Get the unit for EBS pricing
    fn unit(&self) -> &'static str {
        "GB-month"
    }
}

impl From<&str> for EbsType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gp3" => Self::Gp3,
            "gp2" => Self::Gp2,
            "io2" => Self::Io2,
            "st1" => Self::St1,
            "sc1" => Self::Sc1,
            _ => Self::Gp3, // Default to gp3
        }
    }
}

impl From<String> for EbsType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS EBS prices.
pub struct EbsBuilder<'a> {
    client: &'a Client,
    ebs_type: EbsType,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    // Volume specs for monthly cost calculation
    size_gb: Option<u64>,
    iops: Option<u64>,
    throughput_mibps: Option<u64>,
}

impl<'a> EbsBuilder<'a> {
    /// Create a new EBS builder
    pub(crate) fn new(client: &'a Client, ebs_type: EbsType) -> Self {
        Self {
            client,
            ebs_type,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
            iops: None,
            throughput_mibps: None,
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

    /// Set the volume size in GB (required for `fetch_monthly`).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Set provisioned IOPS (for gp3/io2 volumes).
    ///
    /// For gp3: baseline 3000 IOPS is included; you only pay for IOPS above that.
    /// For io2: all provisioned IOPS are billed.
    pub fn iops(mut self, iops: u64) -> Self {
        self.iops = Some(iops);
        self
    }

    /// Set provisioned throughput in MiBps (for gp3 volumes).
    ///
    /// Baseline 125 MiBps is included; you only pay for throughput above that.
    pub fn throughput_mibps(mut self, throughput: u64) -> Self {
        self.throughput_mibps = Some(throughput);
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
            .unwrap_or_else(|| self.ebs_type.default_price());
        let unit = self.ebs_type.unit();

        // Determine effective API key
        let effective_key = self.api_key.as_deref().or_else(|| {
            if self.client.has_api_key() {
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
                Ok(PriceResult::from_default(default_price, unit))
            }
            Err(_) if !self.client.error_on_fallback() => {
                Ok(PriceResult::from_default(default_price, unit))
            }
            Err(e) => Err(e),
            Ok(_) => Err(crate::Error::no_products()),
        }
    }

    /// Fetch total monthly cost based on volume specs.
    ///
    /// Requires `size_gb()` to be set. Optionally set `iops()` and `throughput_mibps()`
    /// for volumes that support provisioned performance (gp3, io2).
    ///
    /// The calculation applies baseline allocations:
    /// - gp3: 3000 IOPS and 125 MiBps included in base price
    /// - io2: all IOPS are billed (no baseline)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.aws().ebs("gp3")
    ///     .size_gb(500)
    ///     .iops(6000)
    ///     .throughput_mibps(250)
    ///     .fetch_monthly().await?;
    /// // Cost = (500 * $0.08) + (3000 * $0.005) + (125 * $0.04) = $60/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let size_gb = self
            .size_gb
            .ok_or_else(|| crate::Error::validation("size_gb is required for fetch_monthly"))?;

        let region = self.region.as_deref().unwrap_or("us-east-1");
        let volume_type = self.ebs_type.volume_api_name();

        // Get price components
        let storage_price = self.fetch_storage_price(region, volume_type).await?;
        let throughput_price = self.fetch_throughput_price(region, volume_type).await?;

        // Calculate storage cost
        let storage_cost = size_gb as f64 * storage_price;

        // Calculate IOPS cost
        let iops_cost = if self.ebs_type.supports_iops() {
            let provisioned_iops = self.iops.unwrap_or(self.ebs_type.baseline_iops());
            let billable_iops = provisioned_iops.saturating_sub(self.ebs_type.baseline_iops());

            if self.ebs_type == EbsType::Io2 && billable_iops > 0 {
                // io2 uses tiered pricing
                let (tier1_price, tier2_price, tier3_price) =
                    self.fetch_io2_tiered_iops_price(region).await?;
                Self::calculate_io2_iops_cost(billable_iops, tier1_price, tier2_price, tier3_price)
            } else {
                // gp3 uses flat pricing
                let iops_price = self.fetch_iops_price(region, volume_type).await?;
                billable_iops as f64 * iops_price.unwrap_or(0.0)
            }
        } else {
            0.0
        };

        // Calculate throughput cost (subtract baseline for gp3)
        let throughput_cost = if self.ebs_type.supports_throughput() {
            let provisioned_throughput = self
                .throughput_mibps
                .unwrap_or(self.ebs_type.baseline_throughput_mibps());
            let billable_throughput =
                provisioned_throughput.saturating_sub(self.ebs_type.baseline_throughput_mibps());
            billable_throughput as f64 * throughput_price.unwrap_or(0.0)
        } else {
            0.0
        };

        let total = storage_cost + iops_cost + throughput_cost;

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
    async fn fetch_storage_price(&self, region: &str, volume_type: &str) -> Result<f64> {
        let default = self.ebs_type.default_storage_price();

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        // Use volumeApiName for cross-region compatibility
        // usagetype varies by region (EU-, APS1-, etc. prefixes)
        let filter = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .product_family("Storage")
            .attribute("volumeApiName", volume_type)
            .attribute("servicecode", "AmazonEC2")
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

    /// Fetch IOPS price per IOPS-month
    /// For io2, this returns only the tier 1 price. Use fetch_io2_tiered_iops_price for full tiered calculation.
    async fn fetch_iops_price(&self, region: &str, volume_type: &str) -> Result<Option<f64>> {
        if !self.ebs_type.supports_iops() {
            return Ok(None);
        }

        let default = self.ebs_type.default_iops_price();

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        // Use group attribute for cross-region compatibility
        // usagetype varies by region (EU-, APS1-, etc. prefixes)
        let filter = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .attribute("group", "EBS IOPS")
            .attribute("volumeApiName", volume_type)
            .attribute("servicecode", "AmazonEC2")
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

    /// Fetch all three tiers of io2 IOPS pricing
    async fn fetch_io2_tiered_iops_price(&self, region: &str) -> Result<(f64, f64, f64)> {
        // Default prices for the three tiers
        let default_tier1 = 0.065;
        let default_tier2 = 0.0455;
        let default_tier3 = 0.03185;

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok((default_tier1, default_tier2, default_tier3));
        }

        // Use group attribute for cross-region compatibility
        // usagetype varies by region (EU-, APS1-, etc. prefixes)

        // Fetch tier 1 (1-32,000 IOPS)
        let filter_tier1 = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .attribute("group", "EBS IOPS")
            .attribute("volumeApiName", "io2")
            .attribute_regex("description", ".*tier 1|^((?!tier).)*$") // tier 1 or no tier mentioned
            .attribute("servicecode", "AmazonEC2")
            .build();

        // Fetch tier 2 (32,001-64,000 IOPS)
        let filter_tier2 = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .attribute("group", "EBS IOPS")
            .attribute("volumeApiName", "io2")
            .attribute_regex("description", "tier 2")
            .attribute("servicecode", "AmazonEC2")
            .build();

        // Fetch tier 3 (64,001+ IOPS)
        let filter_tier3 = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .attribute("group", "EBS IOPS")
            .attribute("volumeApiName", "io2")
            .attribute_regex("description", "tier 3")
            .attribute("servicecode", "AmazonEC2")
            .build();

        let tier1_price = match self
            .client
            .query_products_with_key(filter_tier1, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => {
                products[0].first_nonzero_price_or(default_tier1)
            }
            _ => default_tier1,
        };

        let tier2_price = match self
            .client
            .query_products_with_key(filter_tier2, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => {
                products[0].first_nonzero_price_or(default_tier2)
            }
            _ => default_tier2,
        };

        let tier3_price = match self
            .client
            .query_products_with_key(filter_tier3, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => {
                products[0].first_nonzero_price_or(default_tier3)
            }
            _ => default_tier3,
        };

        Ok((tier1_price, tier2_price, tier3_price))
    }

    /// Calculate io2 IOPS cost with tiered pricing
    fn calculate_io2_iops_cost(
        iops: u64,
        tier1_price: f64,
        tier2_price: f64,
        tier3_price: f64,
    ) -> f64 {
        let mut cost = 0.0;
        let mut remaining = iops;

        // Tier 1: 1-32,000 IOPS at tier1_price
        if remaining > 0 {
            let tier1_iops = remaining.min(32000);
            cost += tier1_iops as f64 * tier1_price;
            remaining = remaining.saturating_sub(32000);
        }

        // Tier 2: 32,001-64,000 IOPS at tier2_price
        if remaining > 0 {
            let tier2_iops = remaining.min(32000);
            cost += tier2_iops as f64 * tier2_price;
            remaining = remaining.saturating_sub(32000);
        }

        // Tier 3: 64,001+ IOPS at tier3_price
        if remaining > 0 {
            cost += remaining as f64 * tier3_price;
        }

        cost
    }

    /// Fetch throughput price per MiBps-month
    async fn fetch_throughput_price(&self, region: &str, volume_type: &str) -> Result<Option<f64>> {
        if !self.ebs_type.supports_throughput() {
            return Ok(None);
        }

        let default = self.ebs_type.default_throughput_price();

        if !self.client.has_api_key() && self.api_key.is_none() && !self.client.error_on_fallback()
        {
            return Ok(default);
        }

        // Use volumeApiName for cross-region compatibility
        // usagetype varies by region (EU-, APS1-, etc. prefixes)
        let filter = ProductFilter::builder()
            .vendor("aws")
            .region(region)
            .attribute("group", "EBS Throughput")
            .attribute("volumeApiName", volume_type)
            .attribute("servicecode", "AmazonEC2")
            .build();

        match self
            .client
            .query_products_with_key(filter, self.api_key.as_deref())
            .await
        {
            Ok(products) if !products.is_empty() => {
                // API returns price in GiBps, convert to MiBps (divide by 1024)
                let price_gibps =
                    products[0].first_nonzero_price_or(default.unwrap_or(0.0) * 1024.0);
                Ok(Some(price_gibps / 1024.0))
            }
            _ if !self.client.error_on_fallback() => Ok(default),
            Err(e) => Err(e),
            Ok(_) => Ok(default),
        }
    }

    fn build_filter(&self) -> ProductFilter {
        ProductFilter::builder()
            .vendor("aws")
            .region(self.region.as_deref().unwrap_or("us-east-1"))
            .product_family("Storage")
            .attribute("volumeApiName", self.ebs_type.volume_api_name())
            .attribute("servicecode", "AmazonEC2")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebs_type_from_str() {
        assert_eq!(EbsType::from("gp3"), EbsType::Gp3);
        assert_eq!(EbsType::from("GP3"), EbsType::Gp3);
        assert_eq!(EbsType::from("gp2"), EbsType::Gp2);
        assert_eq!(EbsType::from("io2"), EbsType::Io2);
        assert_eq!(EbsType::from("st1"), EbsType::St1);
        assert_eq!(EbsType::from("sc1"), EbsType::Sc1);
        assert_eq!(EbsType::from("unknown"), EbsType::Gp3);
    }

    #[test]
    fn test_ebs_type_defaults() {
        assert_eq!(EbsType::Gp3.default_price(), 0.08);
        assert_eq!(EbsType::Gp2.default_price(), 0.10);
        assert_eq!(EbsType::Io2.default_price(), 0.125);
        assert_eq!(EbsType::St1.default_price(), 0.045);
        assert_eq!(EbsType::Sc1.default_price(), 0.015);
    }

    #[test]
    fn test_ebs_type_volume_api_name() {
        assert_eq!(EbsType::Gp3.volume_api_name(), "gp3");
        assert_eq!(EbsType::Gp2.volume_api_name(), "gp2");
        assert_eq!(EbsType::Io2.volume_api_name(), "io2");
        assert_eq!(EbsType::St1.volume_api_name(), "st1");
        assert_eq!(EbsType::Sc1.volume_api_name(), "sc1");
    }

    #[tokio::test]
    async fn test_ebs_builder_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.08);
        assert_eq!(result.unit, "GB-month");
    }

    #[tokio::test]
    async fn test_ebs_builder_string_type() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs("gp3")
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert_eq!(result.price, 0.08);
    }

    #[tokio::test]
    async fn test_gp3_fetch_monthly_storage_only() {
        // 500 GB gp3 with baseline IOPS/throughput
        // Cost = 500 * $0.08 = $40/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 40.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_gp3_fetch_monthly_with_extra_iops() {
        // 500 GB gp3 with 6000 IOPS (3000 extra billable)
        // Cost = (500 * $0.08) + (3000 * $0.005) = $40 + $15 = $55/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .size_gb(500)
            .iops(6000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 55.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_gp3_fetch_monthly_with_extra_throughput() {
        // 500 GB gp3 with 250 MiBps throughput (125 extra billable)
        // Cost = (500 * $0.08) + (125 * $0.04) = $40 + $5 = $45/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .size_gb(500)
            .throughput_mibps(250)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 45.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_gp3_fetch_monthly_full_spec() {
        // 500 GB gp3 with 6000 IOPS and 250 MiBps throughput
        // Cost = (500 * $0.08) + (3000 * $0.005) + (125 * $0.04)
        //      = $40 + $15 + $5 = $60/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .size_gb(500)
            .iops(6000)
            .throughput_mibps(250)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 60.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_gp3_fetch_monthly_baseline_iops_no_charge() {
        // 500 GB gp3 with exactly baseline IOPS (3000) - no extra charge
        // Cost = 500 * $0.08 = $40/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .size_gb(500)
            .iops(3000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 40.0);
    }

    #[tokio::test]
    async fn test_gp2_fetch_monthly_storage_only() {
        // gp2 has no provisioned IOPS/throughput - storage only
        // 500 GB * $0.10 = $50/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp2)
            .size_gb(500)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 50.0);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_fetch_monthly_requires_size_gb() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .iops(6000)
            .fetch_monthly()
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_ebs_type_baseline_values() {
        assert_eq!(EbsType::Gp3.baseline_iops(), 3000);
        assert_eq!(EbsType::Gp3.baseline_throughput_mibps(), 125);
        assert_eq!(EbsType::Gp2.baseline_iops(), 0);
        assert_eq!(EbsType::Io2.baseline_iops(), 0);
    }

    #[test]
    fn test_ebs_type_supports() {
        assert!(EbsType::Gp3.supports_iops());
        assert!(EbsType::Gp3.supports_throughput());
        assert!(EbsType::Io2.supports_iops());
        assert!(!EbsType::Io2.supports_throughput());
        assert!(!EbsType::Gp2.supports_iops());
        assert!(!EbsType::Gp2.supports_throughput());
    }

    #[test]
    fn test_io2_tiered_iops_calculation() {
        // Tier 1 only: 10,000 IOPS
        // Cost = 10,000 * $0.065 = $650
        let cost = EbsBuilder::calculate_io2_iops_cost(10000, 0.065, 0.0455, 0.03185);
        assert_eq!(cost, 650.0);

        // Tier 1 + Tier 2: 50,000 IOPS
        // Cost = (32,000 * $0.065) + (18,000 * $0.0455) = $2,080 + $819 = $2,899
        let cost = EbsBuilder::calculate_io2_iops_cost(50000, 0.065, 0.0455, 0.03185);
        assert_eq!(cost, 2899.0);

        // All 3 tiers: 100,000 IOPS
        // Cost = (32,000 * $0.065) + (32,000 * $0.0455) + (36,000 * $0.03185)
        //      = $2,080 + $1,456 + $1,146.6 = $4,682.6
        let cost = EbsBuilder::calculate_io2_iops_cost(100000, 0.065, 0.0455, 0.03185);
        assert_eq!(cost, 4682.6);

        // Exactly at tier boundary: 32,000 IOPS
        // Cost = 32,000 * $0.065 = $2,080
        let cost = EbsBuilder::calculate_io2_iops_cost(32000, 0.065, 0.0455, 0.03185);
        assert_eq!(cost, 2080.0);

        // Just over tier 1: 32,001 IOPS
        // Cost = (32,000 * $0.065) + (1 * $0.0455) = $2,080 + $0.0455 = $2,080.0455
        let cost = EbsBuilder::calculate_io2_iops_cost(32001, 0.065, 0.0455, 0.03185);
        assert_eq!(cost, 2080.0455);

        // Exactly at tier 2 boundary: 64,000 IOPS
        // Cost = (32,000 * $0.065) + (32,000 * $0.0455) = $2,080 + $1,456 = $3,536
        let cost = EbsBuilder::calculate_io2_iops_cost(64000, 0.065, 0.0455, 0.03185);
        assert_eq!(cost, 3536.0);

        // Just over tier 2: 64,001 IOPS
        // Cost = (32,000 * $0.065) + (32,000 * $0.0455) + (1 * $0.03185) = $3,536.03185
        let cost = EbsBuilder::calculate_io2_iops_cost(64001, 0.065, 0.0455, 0.03185);
        assert_eq!(cost, 3536.03185);
    }

    #[tokio::test]
    async fn test_io2_fetch_monthly_storage_only() {
        // 100 GB io2 with no provisioned IOPS (baseline is 0 for io2)
        // Cost = 100 * $0.125 = $12.50/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Io2)
            .size_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 12.5);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_io2_fetch_monthly_tier1_iops() {
        // 100 GB io2 with 10,000 IOPS (tier 1)
        // Cost = (100 * $0.125) + (10,000 * $0.065)
        //      = $12.5 + $650 = $662.5/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Io2)
            .size_gb(100)
            .iops(10000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 662.5);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_io2_fetch_monthly_tier1_and_tier2_iops() {
        // 100 GB io2 with 50,000 IOPS (spans tier 1 and tier 2)
        // Cost = (100 * $0.125) + (32,000 * $0.065) + (18,000 * $0.0455)
        //      = $12.5 + $2,080 + $819 = $2,911.5/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Io2)
            .size_gb(100)
            .iops(50000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 2911.5);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_io2_fetch_monthly_all_tiers() {
        // 100 GB io2 with 100,000 IOPS (spans all 3 tiers)
        // Cost = (100 * $0.125) + (32,000 * $0.065) + (32,000 * $0.0455) + (36,000 * $0.03185)
        //      = $12.5 + $2,080 + $1,456 + $1,146.6 = $4,695.1/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Io2)
            .size_gb(100)
            .iops(100000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 4695.1);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_io2_no_baseline_all_iops_billed() {
        // Verify io2 has no baseline - all IOPS are billed
        // 100 GB io2 with 1000 IOPS
        // Cost = (100 * $0.125) + (1000 * $0.065) = $12.5 + $65 = $77.5/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Io2)
            .size_gb(100)
            .iops(1000)
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 77.5);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_io2_does_not_support_throughput() {
        // io2 does not support throughput provisioning - should be ignored
        // 100 GB io2 with 10,000 IOPS and throughput (throughput should be ignored)
        // Cost = (100 * $0.125) + (10,000 * $0.065) = $662.5/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Io2)
            .size_gb(100)
            .iops(10000)
            .throughput_mibps(500) // should be ignored
            .fetch_monthly()
            .await
            .unwrap();

        assert_eq!(result.price, 662.5);
        assert_eq!(result.unit, "month");
    }
}
