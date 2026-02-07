//! Blocking AWS provider for querying AWS resource prices.
//!
//! This module provides synchronous wrappers around the async AWS provider builders.
//! Each builder stores parameters and uses `runtime.block_on()` to execute async operations.
//!
//! # Example
//!
//! ```no_run
//! use infracost_rs::blocking::Client;
//! use infracost_rs::providers::aws::EbsType;
//!
//! # fn main() -> Result<(), infracost_rs::Error> {
//! let client = Client::anonymous();
//!
//! let price = client
//!     .aws()
//!     .ebs(EbsType::Gp3)
//!     .region("us-east-1")
//!     .fetch_price()?;
//!
//! println!("EBS gp3 price: ${}/GB-month", price);
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use crate::providers::PriceResult;
use crate::providers::aws::EbsType;
use std::sync::Arc;

/// Blocking AWS provider for querying AWS resource prices.
pub struct BlockingAwsProvider {
    pub(crate) client: crate::Client,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

impl BlockingAwsProvider {
    /// Query AWS EBS volume pricing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    /// use infracost_rs::providers::aws::EbsType;
    ///
    /// # fn main() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .aws()
    ///     .ebs(EbsType::Gp3)
    ///     .region("us-east-1")
    ///     .fetch_price()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn ebs(self, ebs_type: impl Into<EbsType>) -> BlockingAwsEbsBuilder {
        BlockingAwsEbsBuilder {
            client: self.client,
            runtime: self.runtime,
            ebs_type: ebs_type.into(),
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
            iops: None,
            throughput_mibps: None,
        }
    }

    /// Parse an AWS EBS volume JSON (from `aws ec2 describe-volumes`) into a blocking EbsBuilder.
    pub fn ebs_from_json(self, json: &serde_json::Value) -> crate::Result<BlockingAwsEbsBuilder> {
        let parsed = crate::providers::aws::from_json::parse_ebs_json(json)?;
        Ok(BlockingAwsEbsBuilder {
            client: self.client,
            runtime: self.runtime,
            ebs_type: parsed.ebs_type,
            region: parsed.region,
            api_key: None,
            override_default: None,
            size_gb: parsed.size_gb,
            iops: parsed.iops,
            throughput_mibps: parsed.throughput_mibps,
        })
    }

    /// Parse an AWS snapshot JSON (from `aws ec2 describe-snapshots`) into a blocking SnapshotBuilder.
    pub fn snapshot_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAwsSnapshotBuilder> {
        let parsed = crate::providers::aws::from_json::parse_snapshot_json(json)?;
        Ok(BlockingAwsSnapshotBuilder {
            client: self.client,
            runtime: self.runtime,
            region: parsed.region,
            api_key: None,
            override_default: None,
            size_gb: parsed.size_gb,
        })
    }

    /// Parse an AWS Elastic IP JSON (from `aws ec2 describe-addresses`) into a blocking ElasticIpBuilder.
    pub fn elastic_ip_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAwsElasticIpBuilder> {
        let parsed = crate::providers::aws::from_json::parse_elastic_ip_json(json)?;
        Ok(BlockingAwsElasticIpBuilder {
            client: self.client,
            runtime: self.runtime,
            region: parsed.region,
            api_key: None,
            override_default: None,
        })
    }

    /// Parse an AWS NAT Gateway JSON (from `aws ec2 describe-nat-gateways`) into a blocking NatGatewayBuilder.
    pub fn nat_gateway_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAwsNatGatewayBuilder> {
        let parsed = crate::providers::aws::from_json::parse_nat_gateway_json(json)?;
        Ok(BlockingAwsNatGatewayBuilder {
            client: self.client,
            runtime: self.runtime,
            region: parsed.region,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
        })
    }

    /// Parse an AWS ALB JSON (from `aws elbv2 describe-load-balancers`) into a blocking AlbBuilder.
    pub fn alb_from_json(self, json: &serde_json::Value) -> crate::Result<BlockingAwsAlbBuilder> {
        let parsed = crate::providers::aws::from_json::parse_alb_json(json)?;
        Ok(BlockingAwsAlbBuilder {
            client: self.client,
            runtime: self.runtime,
            region: parsed.region,
            api_key: None,
            override_default: None,
            lcu_hours: None,
        })
    }

    /// Query AWS EBS Snapshot pricing.
    ///
    /// Default: $0.05/GB-month
    pub fn snapshot(self) -> BlockingAwsSnapshotBuilder {
        BlockingAwsSnapshotBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
        }
    }

    /// Query AWS Elastic IP pricing (idle/unused).
    ///
    /// Default: $0.005/hour (~$3.65/month)
    pub fn elastic_ip(self) -> BlockingAwsElasticIpBuilder {
        BlockingAwsElasticIpBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
        }
    }

    /// Query AWS NAT Gateway pricing (hourly).
    ///
    /// Default: $0.045/hour (~$32.85/month)
    /// Note: Additional data processing charges apply ($0.045/GB)
    pub fn nat_gateway(self) -> BlockingAwsNatGatewayBuilder {
        BlockingAwsNatGatewayBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
        }
    }

    /// Query AWS Application Load Balancer pricing.
    ///
    /// Default: $0.0225/hour (~$16.43/month)
    /// Note: Additional LCU charges apply
    pub fn alb(self) -> BlockingAwsAlbBuilder {
        BlockingAwsAlbBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            lcu_hours: None,
        }
    }
}

// ============================================================
// BlockingAwsEbsBuilder
// ============================================================

/// Blocking builder for querying AWS EBS prices.
pub struct BlockingAwsEbsBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    ebs_type: EbsType,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
    iops: Option<u64>,
    throughput_mibps: Option<u64>,
}

impl BlockingAwsEbsBuilder {
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

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.aws().ebs(self.ebs_type);
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
        if let Some(v) = self.iops {
            b = b.iops(v);
        }
        if let Some(v) = self.throughput_mibps {
            b = b.throughput_mibps(v);
        }
        self.runtime.block_on(b.fetch())
    }

    /// Fetch just the price value.
    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
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
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    /// use infracost_rs::providers::aws::EbsType;
    ///
    /// # fn main() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client.aws().ebs(EbsType::Gp3)
    ///     .size_gb(500)
    ///     .iops(6000)
    ///     .throughput_mibps(250)
    ///     .fetch_monthly()?;
    /// // Cost = (500 * $0.08) + (3000 * $0.005) + (125 * $0.04) = $60/month
    /// # Ok(())
    /// # }
    /// ```
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.aws().ebs(self.ebs_type);
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
        if let Some(v) = self.iops {
            b = b.iops(v);
        }
        if let Some(v) = self.throughput_mibps {
            b = b.throughput_mibps(v);
        }
        self.runtime.block_on(b.fetch_monthly())
    }
}

// ============================================================
// BlockingAwsSnapshotBuilder
// ============================================================

/// Blocking builder for querying AWS EBS Snapshot prices.
pub struct BlockingAwsSnapshotBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
}

impl BlockingAwsSnapshotBuilder {
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

    /// Set the snapshot size in GB (required for `fetch_monthly`).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.aws().snapshot();
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

    /// Fetch the monthly cost (rate × size_gb).
    ///
    /// Requires `size_gb` to be set.
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.aws().snapshot();
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
// BlockingAwsElasticIpBuilder
// ============================================================

/// Blocking builder for querying AWS Elastic IP prices.
///
/// Returns the price for an idle (unused) Elastic IP address.
pub struct BlockingAwsElasticIpBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl BlockingAwsElasticIpBuilder {
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

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.aws().elastic_ip();
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

    /// Fetch the monthly price (hourly price × 730 hours).
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.aws().elastic_ip();
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
// BlockingAwsNatGatewayBuilder
// ============================================================

/// Blocking builder for querying AWS NAT Gateway prices.
///
/// Returns the hourly rate for NAT Gateway. Additional data processing
/// charges apply ($0.045/GB).
pub struct BlockingAwsNatGatewayBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    data_processed_gb: Option<u64>,
}

impl BlockingAwsNatGatewayBuilder {
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

    /// Set the amount of data processed in GB per month.
    ///
    /// Required for `fetch_monthly()` to calculate total monthly cost including
    /// both hourly charges and data processing charges.
    pub fn data_processed_gb(mut self, gb: u64) -> Self {
        self.data_processed_gb = Some(gb);
        self
    }

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.aws().nat_gateway();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        if let Some(v) = self.data_processed_gb {
            b = b.data_processed_gb(v);
        }
        self.runtime.block_on(b.fetch())
    }

    /// Fetch just the price value.
    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
    }

    /// Fetch total monthly cost for NAT Gateway.
    ///
    /// Calculates: (hourly_rate * 730 hours) + (data_processing_rate * gb_processed)
    ///
    /// If `data_processed_gb()` is not set, only returns the hourly cost for 730 hours.
    ///
    /// # Example
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    ///
    /// # fn main() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client.aws().nat_gateway()
    ///     .region("us-east-1")
    ///     .data_processed_gb(1000)
    ///     .fetch_monthly()?;
    /// // Cost = ($0.045 * 730) + ($0.045 * 1000) = $77.85/month
    /// # Ok(())
    /// # }
    /// ```
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.aws().nat_gateway();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        if let Some(v) = self.data_processed_gb {
            b = b.data_processed_gb(v);
        }
        self.runtime.block_on(b.fetch_monthly())
    }
}

// ============================================================
// BlockingAwsAlbBuilder
// ============================================================

/// Blocking builder for querying AWS Application Load Balancer prices.
///
/// Returns the hourly rate for ALB. Additional LCU (Load Balancer Capacity Units)
/// charges apply based on usage.
pub struct BlockingAwsAlbBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    lcu_hours: Option<u64>,
}

impl BlockingAwsAlbBuilder {
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

    /// Set the LCU-hours per month (required for `fetch_monthly`).
    ///
    /// LCU (Load Balancer Capacity Unit) is a dimension that represents the resources
    /// needed to process your traffic. ALB pricing consists of:
    /// - Hourly charge (~$0.0225/hour = ~$16.43/month for 730 hours)
    /// - LCU charge (~$0.008/LCU-hour)
    ///
    /// The number of LCUs you need depends on your traffic patterns and is calculated
    /// based on the maximum of:
    /// - New connections per second
    /// - Active connections per minute
    /// - Processed bytes
    /// - Rule evaluations
    pub fn lcu_hours(mut self, lcu_hours: u64) -> Self {
        self.lcu_hours = Some(lcu_hours);
        self
    }

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.aws().alb();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        if let Some(v) = self.lcu_hours {
            b = b.lcu_hours(v);
        }
        self.runtime.block_on(b.fetch())
    }

    /// Fetch just the price value.
    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
    }

    /// Fetch total monthly cost based on hourly rate and LCU usage.
    ///
    /// Calculates: (hourly_rate * 730 hours) + (lcu_rate * lcu_hours)
    ///
    /// If `lcu_hours()` is not set, only the hourly cost is calculated.
    ///
    /// # Example
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    ///
    /// # fn main() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client.aws().alb()
    ///     .lcu_hours(10000)
    ///     .fetch_monthly()?;
    /// // Cost = ($0.0225 * 730) + ($0.008 * 10000) = $16.43 + $80 = $96.43/month
    /// # Ok(())
    /// # }
    /// ```
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.aws().alb();
        if let Some(v) = self.region {
            b = b.region(v);
        }
        if let Some(v) = self.api_key {
            b = b.api_key(v);
        }
        if let Some(v) = self.override_default {
            b = b.override_default(v);
        }
        if let Some(v) = self.lcu_hours {
            b = b.lcu_hours(v);
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
    use crate::providers::aws::EbsType;

    // ========================================
    // EBS Tests
    // ========================================

    #[test]
    fn test_blocking_aws_ebs_default() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .unwrap();
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.08);
        assert_eq!(result.unit, "GB-month");
    }

    #[test]
    fn test_blocking_aws_ebs_gp3_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .size_gb(500)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 40.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_aws_ebs_gp3_with_iops_and_throughput() {
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
            .unwrap();
        assert_eq!(result.price, 60.0);
    }

    #[test]
    fn test_blocking_aws_ebs_io2_tiered_iops() {
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
            .unwrap();
        assert_eq!(result.price, 2911.5);
    }

    #[test]
    fn test_blocking_aws_ebs_gp2_storage_only() {
        // gp2 has no provisioned IOPS/throughput - storage only
        // 500 GB * $0.10 = $50/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Gp2)
            .size_gb(500)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 50.0);
    }

    #[test]
    fn test_blocking_aws_ebs_st1() {
        // st1 throughput optimized HDD
        // 1000 GB * $0.045 = $45/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::St1)
            .size_gb(1000)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 45.0);
    }

    #[test]
    fn test_blocking_aws_ebs_sc1() {
        // sc1 cold HDD
        // 1000 GB * $0.015 = $15/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .ebs(EbsType::Sc1)
            .size_gb(1000)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 15.0);
    }

    #[test]
    fn test_blocking_aws_ebs_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .aws()
            .ebs(EbsType::Gp3)
            .region("us-east-1")
            .fetch_price()
            .unwrap();
        assert_eq!(price, 0.08);
    }

    #[test]
    fn test_blocking_aws_ebs_string_type() {
        let client = Client::anonymous();
        let result = client.aws().ebs("gp3").region("us-east-1").fetch().unwrap();
        assert_eq!(result.price, 0.08);
    }

    // ========================================
    // Snapshot Tests
    // ========================================

    #[test]
    fn test_blocking_aws_snapshot_default() {
        let client = Client::anonymous();
        let result = client.aws().snapshot().region("us-east-1").fetch().unwrap();
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
        assert_eq!(result.unit, "GB-month");
    }

    #[test]
    fn test_blocking_aws_snapshot_fetch_monthly() {
        // $0.05/GB-month × 100 GB = $5.00/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .size_gb(100)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 5.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_aws_snapshot_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .aws()
            .snapshot()
            .region("us-east-1")
            .fetch_price()
            .unwrap();
        assert_eq!(price, 0.05);
    }

    // ========================================
    // Elastic IP Tests
    // ========================================

    #[test]
    fn test_blocking_aws_elastic_ip_default() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch()
            .unwrap();
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.005);
        assert_eq!(result.unit, "hour");
    }

    #[test]
    fn test_blocking_aws_elastic_ip_fetch_monthly() {
        // $0.005/hour × 730 hours = $3.65/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 3.65);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_aws_elastic_ip_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch_price()
            .unwrap();
        assert_eq!(price, 0.005);
    }

    // ========================================
    // NAT Gateway Tests
    // ========================================

    #[test]
    fn test_blocking_aws_nat_gateway_default() {
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch()
            .unwrap();
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.045);
        assert_eq!(result.unit, "hour");
    }

    #[test]
    fn test_blocking_aws_nat_gateway_fetch_monthly_with_data() {
        // NAT Gateway with 1000 GB data processed per month
        // Cost = ($0.045 * 730) + ($0.045 * 1000) = $32.85 + $45.00 = $77.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 77.85);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_aws_nat_gateway_fetch_monthly_hourly_only() {
        // NAT Gateway with no data processing specified
        // Cost = $0.045 * 730 = $32.85/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 32.85);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_aws_nat_gateway_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch_price()
            .unwrap();
        assert_eq!(price, 0.045);
    }

    // ========================================
    // ALB Tests
    // ========================================

    #[test]
    fn test_blocking_aws_alb_default() {
        let client = Client::anonymous();
        let result = client.aws().alb().region("us-east-1").fetch().unwrap();
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0225);
        assert_eq!(result.unit, "hour");
    }

    #[test]
    fn test_blocking_aws_alb_fetch_monthly_hourly_only() {
        // ALB with no LCU usage specified - hourly cost only
        // Cost = $0.0225/hour * 730 hours = $16.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 16.425);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_aws_alb_fetch_monthly_with_lcu() {
        // ALB with 10,000 LCU-hours per month
        // Cost = ($0.0225 * 730) + ($0.008 * 10000)
        //      = $16.425 + $80 = $96.425/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(10000)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 96.425);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_aws_alb_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .aws()
            .alb()
            .region("us-east-1")
            .fetch_price()
            .unwrap();
        assert_eq!(price, 0.0225);
    }

    #[test]
    fn test_blocking_aws_alb_minimal_lcu() {
        // ALB with minimal LCU usage (730 LCU-hours = 1 LCU for whole month)
        // Cost = ($0.0225 * 730) + ($0.008 * 730)
        //      = $16.425 + $5.84 = $22.265/month
        let client = Client::anonymous();
        let result = client
            .aws()
            .alb()
            .region("us-east-1")
            .lcu_hours(730)
            .fetch_monthly()
            .unwrap();
        assert_eq!(result.price, 22.265);
    }
}
