//! GCP resource pricing with built-in defaults.
//!
//! # Example
//!
//! ```no_run
//! use infracost_rs::Client;
//! use infracost_rs::providers::gcp::DiskType;
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::anonymous();
//!
//! let price = client
//!     .gcp()
//!     .disk(DiskType::PdSsd)
//!     .region("us-central1")
//!     .fetch_price()
//!     .await?;
//! # Ok(())
//! # }
//! ```

mod disk;
mod forwarding_rule;
pub(crate) mod from_json;
mod nat_gateway;
mod snapshot;
mod static_ip;

pub use disk::{DiskBuilder, DiskType};
pub use forwarding_rule::ForwardingRuleBuilder;
pub use nat_gateway::NatGatewayBuilder;
pub use snapshot::SnapshotBuilder;
pub use static_ip::StaticIpBuilder;

use crate::{Client, Result};

/// GCP provider for querying GCP resource prices.
pub struct GcpProvider<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> GcpProvider<'a> {
    /// Create a new GCP provider
    pub(crate) fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Query GCP Persistent Disk pricing.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::gcp::DiskType;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .gcp()
    ///     .disk(DiskType::PdSsd)
    ///     .region("us-central1")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn disk(self, disk_type: impl Into<DiskType>) -> DiskBuilder<'a> {
        DiskBuilder::new(self.client, disk_type.into())
    }

    /// Query GCP Snapshot pricing.
    ///
    /// Default: $0.05/GB-month
    pub fn snapshot(self) -> SnapshotBuilder<'a> {
        SnapshotBuilder::new(self.client)
    }

    /// Query GCP Static IP pricing.
    ///
    /// Default: $0.01/hour (~$7.30/month)
    pub fn static_ip(self) -> StaticIpBuilder<'a> {
        StaticIpBuilder::new(self.client)
    }

    /// Query GCP NAT Gateway uptime pricing.
    ///
    /// Default: $0.0014/hour (~$1.02/month)
    /// Note: Additional data processing charges apply ($0.045/GB)
    pub fn nat_gateway(self) -> NatGatewayBuilder<'a> {
        NatGatewayBuilder::new(self.client)
    }

    /// Query GCP Forwarding Rule (Load Balancer) pricing.
    ///
    /// Default: $0.025/hour (~$18.25/month)
    /// Note: Additional data processing charges apply
    pub fn forwarding_rule(self) -> ForwardingRuleBuilder<'a> {
        ForwardingRuleBuilder::new(self.client)
    }

    /// Parse a GCP disk JSON (from `gcloud compute disks describe --format=json`) into a DiskBuilder.
    pub fn disk_from_json(self, json: &serde_json::Value) -> Result<DiskBuilder<'a>> {
        let parsed = from_json::parse_disk_json(json)?;
        let mut builder = DiskBuilder::new(self.client, parsed.disk_type);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        if let Some(s) = parsed.size_gb {
            builder = builder.size_gb(s);
        }
        if let Some(i) = parsed.iops {
            builder = builder.iops(i);
        }
        if let Some(t) = parsed.throughput {
            builder = builder.throughput(t);
        }
        if parsed.regional {
            builder = builder.regional(true);
        }
        Ok(builder)
    }

    /// Parse a GCP snapshot JSON (from `gcloud compute snapshots describe --format=json`) into a SnapshotBuilder.
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

    /// Parse a GCP static IP JSON (from `gcloud compute addresses describe --format=json`) into a StaticIpBuilder.
    pub fn static_ip_from_json(self, json: &serde_json::Value) -> Result<StaticIpBuilder<'a>> {
        let parsed = from_json::parse_static_ip_json(json)?;
        let mut builder = StaticIpBuilder::new(self.client);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        Ok(builder)
    }

    /// Parse a GCP NAT gateway JSON (from `gcloud compute routers nats describe --format=json`) into a NatGatewayBuilder.
    pub fn nat_gateway_from_json(self, json: &serde_json::Value) -> Result<NatGatewayBuilder<'a>> {
        let parsed = from_json::parse_nat_gateway_json(json)?;
        let mut builder = NatGatewayBuilder::new(self.client);
        if let Some(r) = parsed.region {
            builder = builder.region(r);
        }
        Ok(builder)
    }
}
