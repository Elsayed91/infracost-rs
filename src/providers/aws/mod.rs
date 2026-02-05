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
mod nat_gateway;
mod snapshot;

pub use alb::AlbBuilder;
pub use ebs::{EbsBuilder, EbsType};
pub use elastic_ip::ElasticIpBuilder;
pub use nat_gateway::NatGatewayBuilder;
pub use snapshot::SnapshotBuilder;

use crate::Client;

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
}
