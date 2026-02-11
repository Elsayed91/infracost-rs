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

mod backend_service;
mod compute_instance;
mod disk;
mod forwarding_rule;
mod nat_gateway;
mod snapshot;
mod static_ip;

pub use backend_service::{BackendServiceBuilder, BackendServiceTier};
pub use compute_instance::{ComputeInstanceBuilder, MachineFamily};
pub use disk::{DiskBuilder, DiskType};
pub use forwarding_rule::ForwardingRuleBuilder;
pub use nat_gateway::NatGatewayBuilder;
pub use snapshot::SnapshotBuilder;
pub use static_ip::StaticIpBuilder;

use crate::Client;

/// GCP provider for querying GCP resource prices.
pub struct GcpProvider {
    pub(crate) client: Client,
}

impl GcpProvider {
    /// Create a new GCP provider
    pub(crate) fn new(client: Client) -> Self {
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
    pub fn disk(self, disk_type: impl Into<DiskType>) -> DiskBuilder {
        DiskBuilder::new(self.client, disk_type.into())
    }

    /// Query GCP Snapshot pricing.
    ///
    /// Default: $0.05/GB-month
    pub fn snapshot(self) -> SnapshotBuilder {
        SnapshotBuilder::new(self.client)
    }

    /// Query GCP Static IP pricing.
    ///
    /// Default: $0.01/hour (~$7.30/month)
    pub fn static_ip(self) -> StaticIpBuilder {
        StaticIpBuilder::new(self.client)
    }

    /// Query GCP NAT Gateway uptime pricing.
    ///
    /// Default: $0.0014/hour (~$1.02/month)
    /// Note: Additional data processing charges apply ($0.045/GB)
    pub fn nat_gateway(self) -> NatGatewayBuilder {
        NatGatewayBuilder::new(self.client)
    }

    /// Query GCP Forwarding Rule (Load Balancer) pricing.
    ///
    /// Default: $0.025/hour (~$18.25/month)
    /// Note: Additional data processing charges apply
    pub fn forwarding_rule(self) -> ForwardingRuleBuilder {
        ForwardingRuleBuilder::new(self.client)
    }

    /// Query GCP Backend Service pricing.
    ///
    /// Backend services handle data processing for load balancers.
    /// - Premium tier (global): $0.008/GiB data processing
    /// - Standard tier (regional): $0.008/GiB data processing
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::gcp::BackendServiceTier;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .gcp()
    ///     .backend_service(BackendServiceTier::Premium)
    ///     .region("us-central1")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn backend_service(self, tier: impl Into<BackendServiceTier>) -> BackendServiceBuilder {
        BackendServiceBuilder::new(self.client, tier.into())
    }

    /// Query GCP Compute Instance pricing.
    ///
    /// Compute instances are priced by CPU cores and RAM separately.
    /// Use `cpu_cores()` and `memory_gib()` to calculate total monthly cost.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::gcp::MachineFamily;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client
    ///     .gcp()
    ///     .compute_instance(MachineFamily::N2)
    ///     .cpu_cores(4)
    ///     .memory_gib(16)
    ///     .fetch_monthly()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn compute_instance(
        self,
        machine_family: impl Into<MachineFamily>,
    ) -> ComputeInstanceBuilder {
        ComputeInstanceBuilder::new(self.client, machine_family.into())
    }
}
