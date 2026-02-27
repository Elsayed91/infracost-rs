//! Blocking GCP provider for querying GCP resource prices.
//!
//! This module provides synchronous wrappers around the async GCP provider API.
//!
//! # Example
//!
//! ```no_run
//! use infracost_rs::blocking::Client;
//! use infracost_rs::providers::gcp::DiskType;
//!
//! fn main() -> Result<(), infracost_rs::Error> {
//!     let client = Client::anonymous()?;
//!     let price = client
//!         .gcp()
//!         .disk(DiskType::PdSsd)
//!         .region("us-central1")
//!         .fetch()?;
//!
//!     println!("{}", price);
//!     Ok(())
//! }
//! ```

use crate::providers::gcp::{
    BackendServiceTier, CloudSqlAvailability, CloudSqlEngine, DiskType, PurchaseOption,
    SnapshotType,
};
use std::sync::Arc;

// ============================================================
// Provider
// ============================================================

/// Blocking GCP provider for querying GCP resource prices.
///
/// This is a synchronous wrapper around the async [`crate::providers::gcp::GcpProvider`].
pub struct BlockingGcpProvider {
    pub(crate) client: crate::Client,
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
}

impl BlockingGcpProvider {
    /// Query GCP Persistent Disk pricing.
    pub fn disk(self, disk_type: impl Into<DiskType>) -> BlockingGcpDiskBuilder {
        BlockingGcpDiskBuilder {
            inner: self.client.gcp().disk(disk_type),
            runtime: self.runtime,
        }
    }

    /// Query GCP Snapshot pricing.
    pub fn snapshot(self, snapshot_type: impl Into<SnapshotType>) -> BlockingGcpSnapshotBuilder {
        BlockingGcpSnapshotBuilder {
            inner: self.client.gcp().snapshot(snapshot_type),
            runtime: self.runtime,
        }
    }

    /// Query GCP Static IP pricing.
    pub fn static_ip(self) -> BlockingGcpStaticIpBuilder {
        BlockingGcpStaticIpBuilder {
            inner: self.client.gcp().static_ip(),
            runtime: self.runtime,
        }
    }

    /// Query GCP NAT Gateway uptime pricing.
    pub fn nat_gateway(self) -> BlockingGcpNatGatewayBuilder {
        BlockingGcpNatGatewayBuilder {
            inner: self.client.gcp().nat_gateway(),
            runtime: self.runtime,
        }
    }

    /// Query GCP Forwarding Rule (Load Balancer) pricing.
    pub fn forwarding_rule(self) -> BlockingGcpForwardingRuleBuilder {
        BlockingGcpForwardingRuleBuilder {
            inner: self.client.gcp().forwarding_rule(),
            runtime: self.runtime,
        }
    }

    /// Query GCP Backend Service pricing.
    pub fn backend_service(
        self,
        tier: impl Into<BackendServiceTier>,
    ) -> BlockingGcpBackendServiceBuilder {
        BlockingGcpBackendServiceBuilder {
            inner: self.client.gcp().backend_service(tier),
            runtime: self.runtime,
        }
    }

    /// Query GCP Cloud SQL pricing.
    pub fn cloud_sql(self) -> BlockingGcpCloudSqlBuilder {
        BlockingGcpCloudSqlBuilder {
            inner: self.client.gcp().cloud_sql(),
            runtime: self.runtime,
        }
    }

    /// Query GCP BigQuery Storage pricing.
    pub fn bigquery_storage(self) -> BlockingGcpBigQueryStorageBuilder {
        BlockingGcpBigQueryStorageBuilder {
            inner: self.client.gcp().bigquery_storage(),
            runtime: self.runtime,
        }
    }

    /// Query GCP Compute Instance pricing.
    pub fn compute_instance(self) -> BlockingGcpComputeInstanceBuilder {
        BlockingGcpComputeInstanceBuilder {
            inner: self.client.gcp().compute_instance(),
            runtime: self.runtime,
        }
    }
}

// ============================================================
// Blocking Builders (generated via macro)
// ============================================================

blocking_builder! {
    /// Blocking builder for querying GCP disk prices.
    pub struct BlockingGcpDiskBuilder wraps crate::providers::gcp::DiskBuilder {
        fn size_gb(u64);
        fn iops(u64);
        fn throughput(u64);
        fn regional(bool);
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP snapshot prices.
    pub struct BlockingGcpSnapshotBuilder wraps crate::providers::gcp::SnapshotBuilder {
        fn size_gb(u64);
        fn retrieval_size_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP static IP prices.
    pub struct BlockingGcpStaticIpBuilder wraps crate::providers::gcp::StaticIpBuilder {
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP NAT Gateway prices.
    pub struct BlockingGcpNatGatewayBuilder wraps crate::providers::gcp::NatGatewayBuilder {
        fn data_processed_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP Forwarding Rule prices.
    pub struct BlockingGcpForwardingRuleBuilder wraps crate::providers::gcp::ForwardingRuleBuilder {
        fn data_processed_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP Backend Service prices.
    pub struct BlockingGcpBackendServiceBuilder wraps crate::providers::gcp::BackendServiceBuilder {
        fn data_processed_gb(u64);
        fn forwarding_rules(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP Cloud SQL prices.
    pub struct BlockingGcpCloudSqlBuilder wraps crate::providers::gcp::CloudSqlBuilder {
        fn engine(CloudSqlEngine);
        fn availability(CloudSqlAvailability);
        fn cpu_count(u64);
        fn memory_gb(u64);
        fn storage_gb(u64);
        fn backup_storage_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP BigQuery Storage prices.
    pub struct BlockingGcpBigQueryStorageBuilder wraps crate::providers::gcp::BigQueryStorageBuilder {
        fn active_logical_storage_gb(u64);
        fn long_term_logical_storage_gb(u64);
        fn active_physical_storage_gb(u64);
        fn long_term_physical_storage_gb(u64);
    }
}

blocking_builder! {
    /// Blocking builder for querying GCP Compute Instance prices.
    pub struct BlockingGcpComputeInstanceBuilder wraps crate::providers::gcp::ComputeInstanceBuilder {
        fn machine_type(&str);
        fn machine_family(&str);
        fn cpu_cores(u64);
        fn memory_gib(u64);
        fn purchase_option(PurchaseOption);
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocking::Client;

    /// Smoke test: verify blocking_builder! macro works for all GCP builders.
    /// Detailed assertions are in the async builder tests - these just verify
    /// the blocking wrappers compile and execute without panicking.
    #[test]
    fn test_blocking_gcp_smoke() {
        let client = Client::anonymous().unwrap();

        // Disk builder - test all fetch methods
        let _ = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .fetch_price()
            .unwrap();
        let _ = client
            .gcp()
            .disk(DiskType::PdSsd)
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        // Snapshot builder (standard)
        let _ = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .snapshot(SnapshotType::Standard)
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        // Snapshot builder (archive)
        let _ = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .snapshot(SnapshotType::Archive)
            .size_gb(100)
            .retrieval_size_gb(50)
            .fetch_monthly()
            .unwrap();

        // Static IP builder
        let _ = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client.gcp().static_ip().fetch_monthly().unwrap();

        // NAT Gateway builder
        let _ = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .nat_gateway()
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Forwarding Rule builder
        let _ = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .forwarding_rule()
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Backend Service builder
        let _ = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .backend_service(BackendServiceTier::Standard)
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Cloud SQL builder
        let _ = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::MySql)
            .availability(CloudSqlAvailability::Zonal)
            .cpu_count(2)
            .memory_gb(8)
            .storage_gb(100)
            .fetch_monthly()
            .unwrap();

        // BigQuery Storage builder
        let _ = client
            .gcp()
            .bigquery_storage()
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .bigquery_storage()
            .active_logical_storage_gb(500)
            .long_term_logical_storage_gb(200)
            .fetch_monthly()
            .unwrap();

        // Compute Instance builder
        let _ = client
            .gcp()
            .compute_instance()
            .machine_type("n2-standard-4")
            .region("us-central1")
            .fetch()
            .unwrap();
        let _ = client
            .gcp()
            .compute_instance()
            .machine_type("n2-standard-4")
            .fetch_monthly()
            .unwrap();
        let _ = client
            .gcp()
            .compute_instance()
            .machine_type("e2-medium")
            .purchase_option(PurchaseOption::Preemptible)
            .fetch_monthly()
            .unwrap();
    }

    /// Verify blocking wrappers properly delegate to async builders.
    /// Test one complex case to ensure parameter passing works correctly.
    #[test]
    fn test_blocking_gcp_complex_builder() {
        let client = Client::anonymous().unwrap();

        // Test complex builder with multiple parameters
        let result = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .size_gb(500)
            .iops(15000)
            .regional(true)
            .fetch_monthly()
            .unwrap();

        // Verify it produces expected result (proves parameters flowed through correctly)
        assert_eq!(result.unit, "month");
        assert!(result.price > 1000.0); // Sanity check for pd-extreme with high IOPS
    }
}
