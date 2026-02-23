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
mod bigquery_storage;
mod cloud_sql;
mod compute_instance;
mod disk;
mod forwarding_rule;
mod nat_gateway;
mod snapshot;
mod static_ip;

pub use backend_service::{BackendServiceBuilder, BackendServiceTier};
pub use bigquery_storage::BigQueryStorageBuilder;
pub use cloud_sql::{CloudSqlAvailability, CloudSqlBuilder, CloudSqlEngine};
pub use compute_instance::{ComputeInstanceBuilder, PurchaseOption};
pub use disk::{DiskBuilder, DiskType};
pub use forwarding_rule::ForwardingRuleBuilder;
pub use nat_gateway::NatGatewayBuilder;
pub use snapshot::{SnapshotBuilder, SnapshotType};
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
    /// Supports standard ($0.05/GiB-month) and archive ($0.019/GiB-month) snapshots.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::gcp::SnapshotType;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .gcp()
    ///     .snapshot(SnapshotType::Standard)
    ///     .region("us-central1")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn snapshot(self, snapshot_type: impl Into<SnapshotType>) -> SnapshotBuilder {
        SnapshotBuilder::new(self.client, snapshot_type.into())
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

    /// Query GCP Cloud SQL pricing.
    ///
    /// Supports MySQL, PostgreSQL, and SQL Server engines with Zonal and
    /// Regional (HA) availability types. Prices are per vCPU/hour for the
    /// primary component.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::gcp::{CloudSqlEngine, CloudSqlAvailability};
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client
    ///     .gcp()
    ///     .cloud_sql()
    ///     .engine(CloudSqlEngine::PostgreSql)
    ///     .availability(CloudSqlAvailability::Zonal)
    ///     .cpu_count(4)
    ///     .memory_gb(16)
    ///     .storage_gb(100)
    ///     .fetch_monthly()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn cloud_sql(self) -> CloudSqlBuilder {
        CloudSqlBuilder::new(self.client)
    }

    /// Query GCP BigQuery Storage pricing.
    ///
    /// Supports logical and physical storage billing models.
    /// - Active logical: $0.023/GiB-month
    /// - Long-term logical: $0.016/GiB-month
    /// - Active physical: $0.04/GiB-month
    /// - Long-term physical: $0.02/GiB-month
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client
    ///     .gcp()
    ///     .bigquery_storage()
    ///     .active_logical_storage_gb(500)
    ///     .long_term_logical_storage_gb(200)
    ///     .fetch_monthly()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn bigquery_storage(self) -> BigQueryStorageBuilder {
        BigQueryStorageBuilder::new(self.client)
    }

    /// Query GCP Compute Instance pricing.
    ///
    /// Supports parsing machine types (e.g., "n2-standard-4") or manual specs.
    /// Compute instances are priced by CPU cores and RAM separately.
    ///
    /// # Example with machine type
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client
    ///     .gcp()
    ///     .compute_instance()
    ///     .machine_type("n2-standard-4")
    ///     .fetch_monthly()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example with manual specs
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let cost = client
    ///     .gcp()
    ///     .compute_instance()
    ///     .machine_family("n2")
    ///     .cpu_cores(4)
    ///     .memory_gib(16)
    ///     .fetch_monthly()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn compute_instance(self) -> ComputeInstanceBuilder {
        ComputeInstanceBuilder::new(self.client)
    }
}
