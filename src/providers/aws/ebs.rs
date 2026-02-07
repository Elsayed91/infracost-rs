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

use std::collections::HashMap;

use crate::catalog::{aws_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

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
    /// Get the resource name for looking up in the YAML catalog
    fn resource_name(&self) -> &'static str {
        match self {
            Self::Gp3 => "ebs/gp3",
            Self::Gp2 => "ebs/gp2",
            Self::Io2 => "ebs/io2",
            Self::St1 => "ebs/st1",
            Self::Sc1 => "ebs/sc1",
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
        let resource = aws_catalog().find(self.ebs_type.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            self.client,
            resource,
            "aws",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
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

        let resource = aws_catalog().find(self.ebs_type.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);

        let mut params = HashMap::new();
        params.insert("size_gb".to_string(), size_gb);

        // For gp3: IOPS defaults to baseline 3000, throughput defaults to baseline 125
        // For io2: IOPS defaults to 0 (no baseline)
        if self.ebs_type.supports_iops() {
            let iops = self.iops.unwrap_or(self.ebs_type.baseline_iops());
            params.insert("iops".to_string(), iops);
        }
        if self.ebs_type.supports_throughput() {
            let throughput = self
                .throughput_mibps
                .unwrap_or(self.ebs_type.baseline_throughput_mibps());
            params.insert("throughput_mibps".to_string(), throughput);
        }

        if self.ebs_type == EbsType::Io2 {
            PricingEngine::fetch_monthly_with_tiered_queries(
                self.client,
                resource,
                "aws",
                region,
                self.api_key.as_deref(),
                &params,
            )
            .await
        } else {
            PricingEngine::fetch_monthly(
                self.client,
                resource,
                "aws",
                region,
                self.api_key.as_deref(),
                &params,
            )
            .await
        }
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
