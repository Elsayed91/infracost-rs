//! Blocking AWS provider for querying AWS resource prices.
//!
//! This module provides synchronous wrappers around the async AWS provider builders.
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

use crate::providers::aws::EbsType;
use std::sync::Arc;

/// Blocking AWS provider for querying AWS resource prices.
pub struct BlockingAwsProvider {
    pub(crate) client: crate::Client,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

impl BlockingAwsProvider {
    /// Query AWS EBS volume pricing.
    pub fn ebs(self, ebs_type: impl Into<EbsType>) -> BlockingAwsEbsBuilder {
        BlockingAwsEbsBuilder {
            inner: self.client.aws().ebs(ebs_type),
            runtime: self.runtime,
        }
    }

    /// Parse an AWS EBS volume JSON (from `aws ec2 describe-volumes`) into a blocking EbsBuilder.
    pub fn ebs_from_json(self, json: &serde_json::Value) -> crate::Result<BlockingAwsEbsBuilder> {
        let parsed = crate::providers::aws::from_json::parse_ebs_json(json)?;
        let mut b = self.client.aws().ebs(parsed.ebs_type);
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        if let Some(s) = parsed.size_gb {
            b = b.size_gb(s);
        }
        if let Some(i) = parsed.iops {
            b = b.iops(i);
        }
        if let Some(t) = parsed.throughput_mibps {
            b = b.throughput_mibps(t);
        }
        Ok(BlockingAwsEbsBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query AWS EBS Snapshot pricing.
    pub fn snapshot(self) -> BlockingAwsSnapshotBuilder {
        BlockingAwsSnapshotBuilder {
            inner: self.client.aws().snapshot(),
            runtime: self.runtime,
        }
    }

    /// Parse an AWS snapshot JSON (from `aws ec2 describe-snapshots`) into a blocking SnapshotBuilder.
    pub fn snapshot_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAwsSnapshotBuilder> {
        let parsed = crate::providers::aws::from_json::parse_snapshot_json(json)?;
        let mut b = self.client.aws().snapshot();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        if let Some(s) = parsed.size_gb {
            b = b.size_gb(s);
        }
        Ok(BlockingAwsSnapshotBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query AWS Elastic IP pricing (idle/unused).
    pub fn elastic_ip(self) -> BlockingAwsElasticIpBuilder {
        BlockingAwsElasticIpBuilder {
            inner: self.client.aws().elastic_ip(),
            runtime: self.runtime,
        }
    }

    /// Parse an AWS Elastic IP JSON (from `aws ec2 describe-addresses`) into a blocking ElasticIpBuilder.
    pub fn elastic_ip_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAwsElasticIpBuilder> {
        let parsed = crate::providers::aws::from_json::parse_elastic_ip_json(json)?;
        let mut b = self.client.aws().elastic_ip();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingAwsElasticIpBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query AWS NAT Gateway pricing (hourly).
    pub fn nat_gateway(self) -> BlockingAwsNatGatewayBuilder {
        BlockingAwsNatGatewayBuilder {
            inner: self.client.aws().nat_gateway(),
            runtime: self.runtime,
        }
    }

    /// Parse an AWS NAT Gateway JSON (from `aws ec2 describe-nat-gateways`) into a blocking NatGatewayBuilder.
    pub fn nat_gateway_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingAwsNatGatewayBuilder> {
        let parsed = crate::providers::aws::from_json::parse_nat_gateway_json(json)?;
        let mut b = self.client.aws().nat_gateway();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingAwsNatGatewayBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query AWS Application Load Balancer pricing.
    pub fn alb(self) -> BlockingAwsAlbBuilder {
        BlockingAwsAlbBuilder {
            inner: self.client.aws().alb(),
            runtime: self.runtime,
        }
    }

    /// Parse an AWS ALB JSON (from `aws elbv2 describe-load-balancers`) into a blocking AlbBuilder.
    pub fn alb_from_json(self, json: &serde_json::Value) -> crate::Result<BlockingAwsAlbBuilder> {
        let parsed = crate::providers::aws::from_json::parse_alb_json(json)?;
        let mut b = self.client.aws().alb();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingAwsAlbBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }
}

// ============================================================
// Blocking Builders (generated via macro)
// ============================================================

blocking_builder! {
    /// Blocking builder for querying AWS EBS prices.
    pub struct BlockingAwsEbsBuilder wraps crate::providers::aws::EbsBuilder {
        fn size_gb(u64);
        fn iops(u64);
        fn throughput_mibps(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying AWS EBS Snapshot prices.
    pub struct BlockingAwsSnapshotBuilder wraps crate::providers::aws::SnapshotBuilder {
        fn size_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying AWS Elastic IP prices.
    pub struct BlockingAwsElasticIpBuilder wraps crate::providers::aws::ElasticIpBuilder {
    }
}

blocking_builder! {
    /// Blocking builder for querying AWS NAT Gateway prices.
    pub struct BlockingAwsNatGatewayBuilder wraps crate::providers::aws::NatGatewayBuilder {
        fn data_processed_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying AWS Application Load Balancer prices.
    pub struct BlockingAwsAlbBuilder wraps crate::providers::aws::AlbBuilder {
        fn lcu_hours(u64);
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
