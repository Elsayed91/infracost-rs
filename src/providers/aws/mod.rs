//! AWS resource pricing with built-in defaults.
//!
//! # Example
//!
//! ```no_run
//! use infracost_rs::Client;
//! use infracost_rs::providers::aws::EbsType;
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::anonymous();
//!
//! let price = client
//!     .aws()
//!     .ebs(EbsType::Gp3)
//!     .region("us-east-1")
//!     .fetch_price()
//!     .await?;
//! # Ok(())
//! # }
//! ```

mod alb;
mod ebs;
mod elastic_ip;
pub(crate) mod from_json;
mod nat_gateway;
mod snapshot;

pub use alb::AlbBuilder;
pub use ebs::{EbsBuilder, EbsType};
pub use elastic_ip::ElasticIpBuilder;
pub use nat_gateway::NatGatewayBuilder;
pub use snapshot::SnapshotBuilder;

use crate::{Client, Result};

/// AWS provider for querying AWS resource prices.
pub struct AwsProvider<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> AwsProvider<'a> {
    /// Create a new AWS provider
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Query AWS EBS volume pricing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::aws::EbsType;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .aws()
    ///     .ebs(EbsType::Gp3)
    ///     .region("us-east-1")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn ebs(self, ebs_type: impl Into<EbsType>) -> EbsBuilder<'a> {
        EbsBuilder::new(self.client, ebs_type.into())
    }

    /// Query AWS EBS Snapshot pricing.
    ///
    /// Default: $0.05/GB-month
    pub fn snapshot(self) -> SnapshotBuilder<'a> {
        SnapshotBuilder::new(self.client)
    }

    /// Query AWS Elastic IP pricing (idle/unused).
    ///
    /// Default: $0.005/hour (~$3.65/month)
    pub fn elastic_ip(self) -> ElasticIpBuilder<'a> {
        ElasticIpBuilder::new(self.client)
    }

    /// Query AWS NAT Gateway pricing (hourly).
    ///
    /// Default: $0.045/hour (~$32.85/month)
    /// Note: Additional data processing charges apply ($0.045/GB)
    pub fn nat_gateway(self) -> NatGatewayBuilder<'a> {
        NatGatewayBuilder::new(self.client)
    }

    /// Query AWS Application Load Balancer pricing.
    ///
    /// Default: $0.0225/hour (~$16.43/month)
    /// Note: Additional LCU charges apply
    pub fn alb(self) -> AlbBuilder<'a> {
        AlbBuilder::new(self.client)
    }

    /// Parse an AWS EBS volume JSON (from `aws ec2 describe-volumes`) into an [`EbsBuilder`].
    pub fn ebs_from_json(self, json: &serde_json::Value) -> Result<EbsBuilder<'a>> {
        let parsed = from_json::parse_ebs_json(json)?;
        let mut builder = EbsBuilder::new(self.client, parsed.ebs_type);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        if let Some(s) = parsed.size_gb {
            builder = builder.size_gb(s);
        }
        if let Some(i) = parsed.iops {
            builder = builder.iops(i);
        }
        if let Some(t) = parsed.throughput_mibps {
            builder = builder.throughput_mibps(t);
        }
        Ok(builder)
    }

    /// Parse an AWS EBS Snapshot JSON (from `aws ec2 describe-snapshots`) into a [`SnapshotBuilder`].
    pub fn snapshot_from_json(self, json: &serde_json::Value) -> Result<SnapshotBuilder<'a>> {
        let parsed = from_json::parse_snapshot_json(json)?;
        let mut builder = SnapshotBuilder::new(self.client);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        if let Some(s) = parsed.size_gb {
            builder = builder.size_gb(s);
        }
        Ok(builder)
    }

    /// Parse an AWS Elastic IP JSON (from `aws ec2 describe-addresses`) into an [`ElasticIpBuilder`].
    pub fn elastic_ip_from_json(self, json: &serde_json::Value) -> Result<ElasticIpBuilder<'a>> {
        let parsed = from_json::parse_elastic_ip_json(json)?;
        let mut builder = ElasticIpBuilder::new(self.client);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        Ok(builder)
    }

    /// Parse an AWS NAT Gateway JSON (from `aws ec2 describe-nat-gateways`) into a [`NatGatewayBuilder`].
    pub fn nat_gateway_from_json(self, json: &serde_json::Value) -> Result<NatGatewayBuilder<'a>> {
        let parsed = from_json::parse_nat_gateway_json(json)?;
        let mut builder = NatGatewayBuilder::new(self.client);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        Ok(builder)
    }

    /// Parse an AWS ALB JSON (from `aws elbv2 describe-load-balancers`) into an [`AlbBuilder`].
    pub fn alb_from_json(self, json: &serde_json::Value) -> Result<AlbBuilder<'a>> {
        let parsed = from_json::parse_alb_json(json)?;
        let mut builder = AlbBuilder::new(self.client);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        Ok(builder)
    }
}
