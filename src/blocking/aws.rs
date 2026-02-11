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

use crate::providers::aws::{EbsType, RdsStorageType};
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

    /// Query AWS EBS Snapshot pricing.
    pub fn snapshot(self) -> BlockingAwsSnapshotBuilder {
        BlockingAwsSnapshotBuilder {
            inner: self.client.aws().snapshot(),
            runtime: self.runtime,
        }
    }

    /// Query AWS Elastic IP pricing (idle/unused).
    pub fn elastic_ip(self) -> BlockingAwsElasticIpBuilder {
        BlockingAwsElasticIpBuilder {
            inner: self.client.aws().elastic_ip(),
            runtime: self.runtime,
        }
    }

    /// Query AWS NAT Gateway pricing (hourly).
    pub fn nat_gateway(self) -> BlockingAwsNatGatewayBuilder {
        BlockingAwsNatGatewayBuilder {
            inner: self.client.aws().nat_gateway(),
            runtime: self.runtime,
        }
    }

    /// Query AWS Application Load Balancer pricing.
    pub fn alb(self) -> BlockingAwsAlbBuilder {
        BlockingAwsAlbBuilder {
            inner: self.client.aws().alb(),
            runtime: self.runtime,
        }
    }

    /// Query AWS EC2 Instance pricing.
    pub fn ec2_instance(self, instance_type: impl Into<String>) -> BlockingAwsEc2InstanceBuilder {
        BlockingAwsEc2InstanceBuilder {
            inner: self.client.aws().ec2_instance(instance_type),
            runtime: self.runtime,
        }
    }

    /// Query AWS RDS pricing.
    pub fn rds(self, instance_class: impl Into<String>) -> BlockingAwsRdsBuilder {
        BlockingAwsRdsBuilder {
            inner: self.client.aws().rds(instance_class),
            runtime: self.runtime,
        }
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

blocking_builder! {
    /// Blocking builder for querying AWS EC2 Instance prices.
    pub struct BlockingAwsEc2InstanceBuilder wraps crate::providers::aws::Ec2InstanceBuilder {
        fn operating_system(&str);
        fn tenancy(&str);
        fn pre_installed_sw(&str);
    }
}

blocking_builder! {
    /// Blocking builder for querying AWS RDS prices.
    pub struct BlockingAwsRdsBuilder wraps crate::providers::aws::RdsBuilder {
        fn engine(&str);
        fn deployment_option(&str);
        fn storage_type(RdsStorageType);
        fn allocated_storage_gb(u64);
        fn iops(u64);
        fn storage_throughput_mbps(u64);
    }
}

impl BlockingAwsRdsBuilder {
    /// Enable Multi-AZ deployment.
    pub fn multi_az(mut self) -> Self {
        self.inner = self.inner.multi_az();
        self
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::blocking::Client;
    use crate::providers::aws::{EbsType, RdsStorageType};

    /// Smoke test: verify blocking_builder! macro works for all AWS builders.
    /// Detailed assertions are in the async builder tests.
    #[test]
    fn test_blocking_aws_smoke() {
        let client = Client::anonymous();

        // EBS builder - test all fetch methods
        let _ = client
            .aws()
            .ebs(EbsType::Gp3)
            .region("us-east-1")
            .fetch()
            .unwrap();
        let _ = client.aws().ebs(EbsType::Io2).fetch_price().unwrap();
        let _ = client
            .aws()
            .ebs(EbsType::Gp2)
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        // Snapshot builder
        let _ = client.aws().snapshot().region("us-east-1").fetch().unwrap();
        let _ = client
            .aws()
            .snapshot()
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        // Elastic IP builder
        let _ = client
            .aws()
            .elastic_ip()
            .region("us-east-1")
            .fetch()
            .unwrap();
        let _ = client.aws().elastic_ip().fetch_monthly().unwrap();

        // NAT Gateway builder
        let _ = client
            .aws()
            .nat_gateway()
            .region("us-east-1")
            .fetch()
            .unwrap();
        let _ = client
            .aws()
            .nat_gateway()
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // ALB builder
        let _ = client.aws().alb().region("us-east-1").fetch().unwrap();
        let _ = client.aws().alb().lcu_hours(10000).fetch_monthly().unwrap();

        // EC2 Instance builder
        let _ = client
            .aws()
            .ec2_instance("t3.micro")
            .region("us-east-1")
            .fetch()
            .unwrap();
        let _ = client
            .aws()
            .ec2_instance("t3.micro")
            .fetch_monthly()
            .unwrap();
        let _ = client
            .aws()
            .ec2_instance("m5.xlarge")
            .operating_system("Windows")
            .tenancy("Shared")
            .fetch()
            .unwrap();

        // RDS builder
        let _ = client
            .aws()
            .rds("db.t3.micro")
            .region("us-east-1")
            .fetch()
            .unwrap();
        let _ = client
            .aws()
            .rds("db.t3.micro")
            .engine("mysql")
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .fetch_monthly()
            .unwrap();
    }

    /// Verify blocking wrappers properly delegate complex builders.
    #[test]
    fn test_blocking_aws_complex_builder() {
        let client = Client::anonymous();

        // Test EBS with multiple parameters
        let result = client
            .aws()
            .ebs(EbsType::Gp3)
            .size_gb(500)
            .iops(6000)
            .throughput_mibps(250)
            .fetch_monthly()
            .unwrap();

        assert_eq!(result.price, 60.0);
        assert_eq!(result.unit, "month");
    }
}
