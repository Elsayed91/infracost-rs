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
//!     let client = Client::anonymous();
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

use crate::providers::gcp::{BackendServiceTier, DiskType};
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

    /// Parse a GCP disk JSON (from `gcloud compute disks describe --format=json`) into a blocking DiskBuilder.
    pub fn disk_from_json(self, json: &serde_json::Value) -> crate::Result<BlockingGcpDiskBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_disk_json(json)?;
        let mut b = self.client.gcp().disk(parsed.disk_type);
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        if let Some(s) = parsed.size_gb {
            b = b.size_gb(s);
        }
        if let Some(i) = parsed.iops {
            b = b.iops(i);
        }
        if let Some(t) = parsed.throughput {
            b = b.throughput(t);
        }
        if parsed.regional {
            b = b.regional(true);
        }
        Ok(BlockingGcpDiskBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query GCP Snapshot pricing.
    pub fn snapshot(self) -> BlockingGcpSnapshotBuilder {
        BlockingGcpSnapshotBuilder {
            inner: self.client.gcp().snapshot(),
            runtime: self.runtime,
        }
    }

    /// Parse a GCP snapshot JSON (from `gcloud compute snapshots describe --format=json`) into a blocking SnapshotBuilder.
    pub fn snapshot_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpSnapshotBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_snapshot_json(json)?;
        let mut b = self.client.gcp().snapshot();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        if let Some(s) = parsed.size_gb {
            b = b.size_gb(s);
        }
        Ok(BlockingGcpSnapshotBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query GCP Static IP pricing.
    pub fn static_ip(self) -> BlockingGcpStaticIpBuilder {
        BlockingGcpStaticIpBuilder {
            inner: self.client.gcp().static_ip(),
            runtime: self.runtime,
        }
    }

    /// Parse a GCP static IP JSON (from `gcloud compute addresses describe --format=json`) into a blocking StaticIpBuilder.
    pub fn static_ip_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpStaticIpBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_static_ip_json(json)?;
        let mut b = self.client.gcp().static_ip();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingGcpStaticIpBuilder {
            inner: b,
            runtime: self.runtime,
        })
    }

    /// Query GCP NAT Gateway uptime pricing.
    pub fn nat_gateway(self) -> BlockingGcpNatGatewayBuilder {
        BlockingGcpNatGatewayBuilder {
            inner: self.client.gcp().nat_gateway(),
            runtime: self.runtime,
        }
    }

    /// Parse a GCP NAT gateway JSON into a blocking NatGatewayBuilder.
    pub fn nat_gateway_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpNatGatewayBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_nat_gateway_json(json)?;
        let mut b = self.client.gcp().nat_gateway();
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingGcpNatGatewayBuilder {
            inner: b,
            runtime: self.runtime,
        })
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

    /// Parse a GCP backend service JSON (from `gcloud compute backend-services describe --format=json`) into a blocking BackendServiceBuilder.
    pub fn backend_service_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpBackendServiceBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_backend_service_json(json)?;
        let mut b = self.client.gcp().backend_service(parsed.tier);
        if let Some(r) = parsed.region {
            b = b.region(r);
        }
        Ok(BlockingGcpBackendServiceBuilder {
            inner: b,
            runtime: self.runtime,
        })
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

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocking::Client;

    // ============================================================
    // Disk Tests
    // ============================================================

    #[test]
    fn test_blocking_gcp_disk_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.17);
        assert_eq!(result.unit, "GiB-month");
    }

    #[test]
    fn test_blocking_gcp_disk_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .gcp()
            .disk(DiskType::PdSsd)
            .region("us-central1")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.17);
    }

    #[test]
    fn test_blocking_gcp_disk_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .size_gb(500)
            .fetch_monthly()
            .unwrap();

        assert_eq!(result.price, 85.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_disk_pd_extreme_with_iops() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdExtreme)
            .size_gb(500)
            .iops(15000)
            .fetch_monthly()
            .unwrap();

        // Cost = (500 * $0.125) + (15000 * $0.065) = $62.5 + $975 = $1037.5/month
        assert_eq!(result.price, 1037.5);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_disk_pd_balanced() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdBalanced)
            .size_gb(500)
            .fetch_monthly()
            .unwrap();

        assert_eq!(result.price, 50.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_disk_pd_standard() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdStandard)
            .size_gb(500)
            .fetch_monthly()
            .unwrap();

        assert_eq!(result.price, 20.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_disk_override_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .disk(DiskType::PdSsd)
            .override_default(0.20)
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.20);
    }

    // ============================================================
    // Snapshot Tests
    // ============================================================

    #[test]
    fn test_blocking_gcp_snapshot_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .snapshot()
            .region("us-central1")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
        assert_eq!(result.unit, "GB-month");
    }

    #[test]
    fn test_blocking_gcp_snapshot_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .gcp()
            .snapshot()
            .region("us-central1")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.05);
    }

    #[test]
    fn test_blocking_gcp_snapshot_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .snapshot()
            .size_gb(100)
            .fetch_monthly()
            .unwrap();

        // 0.05 × 100 = 5.00
        assert_eq!(result.price, 5.00);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_snapshot_override_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .snapshot()
            .override_default(0.06)
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.06);
    }

    // ============================================================
    // Static IP Tests
    // ============================================================

    #[test]
    fn test_blocking_gcp_static_ip_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.01);
        assert_eq!(result.unit, "hour");
    }

    #[test]
    fn test_blocking_gcp_static_ip_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.01);
    }

    #[test]
    fn test_blocking_gcp_static_ip_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .static_ip()
            .region("us-central1")
            .fetch_monthly()
            .unwrap();

        // 0.01 × 730 = 7.30
        assert_eq!(result.price, 7.30);
        assert_eq!(result.unit, "month");
    }

    // ============================================================
    // NAT Gateway Tests
    // ============================================================

    #[test]
    fn test_blocking_gcp_nat_gateway_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0014);
        assert_eq!(result.unit, "hour");
    }

    #[test]
    fn test_blocking_gcp_nat_gateway_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.0014);
    }

    #[test]
    fn test_blocking_gcp_nat_gateway_fetch_monthly_with_data() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Cost = ($0.0014 * 730) + ($0.045 * 1000) = $1.022 + $45.0 = $46.022/month
        assert_eq!(result.price, 46.022);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_nat_gateway_fetch_monthly_no_data() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .nat_gateway()
            .region("us-central1")
            .fetch_monthly()
            .unwrap();

        // Cost = $0.0014 * 730 = $1.022/month (no data processing)
        assert_eq!(result.price, 1.022);
        assert_eq!(result.unit, "month");
    }

    // ============================================================
    // Forwarding Rule Tests
    // ============================================================

    #[test]
    fn test_blocking_gcp_forwarding_rule_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.025);
        assert_eq!(result.unit, "hour");
    }

    #[test]
    fn test_blocking_gcp_forwarding_rule_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.025);
    }

    #[test]
    fn test_blocking_gcp_forwarding_rule_fetch_monthly_with_data() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Cost = ($0.025 * 730) + ($0.008 * 1000) = $18.25 + $8.0 = $26.25/month
        assert_eq!(result.price, 26.25);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_forwarding_rule_fetch_monthly_no_data() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .forwarding_rule()
            .region("us-central1")
            .fetch_monthly()
            .unwrap();

        // Cost = $0.025 * 730 = $18.25/month (no data processing)
        assert_eq!(result.price, 18.25);
        assert_eq!(result.unit, "month");
    }

    // ============================================================
    // Backend Service Tests
    // ============================================================

    #[test]
    fn test_blocking_gcp_backend_service_premium_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.008);
        assert_eq!(result.unit, "GiB");
    }

    #[test]
    fn test_blocking_gcp_backend_service_standard_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Standard)
            .region("us-central1")
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.008);
        assert_eq!(result.unit, "GiB");
    }

    #[test]
    fn test_blocking_gcp_backend_service_premium_fetch_price() {
        let client = Client::anonymous();
        let price = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .fetch_price()
            .unwrap();

        assert_eq!(price, 0.008);
    }

    #[test]
    fn test_blocking_gcp_backend_service_premium_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Cost = $0.008 * 1000 = $8.00/month
        assert_eq!(result.price, 8.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_backend_service_standard_fetch_monthly() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Standard)
            .region("us-central1")
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Cost = $0.008 * 1000 = $8.00/month
        assert_eq!(result.price, 8.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_backend_service_fetch_monthly_no_data() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .fetch_monthly()
            .unwrap();

        // Cost = $0.008 * 0 = $0.00/month
        assert_eq!(result.price, 0.0);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_backend_service_with_forwarding_rule() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .forwarding_rules(1)
            .fetch_monthly()
            .unwrap();

        // Cost = ($0.025 * 730) + ($0.008 * 0) = $18.25/month
        assert_eq!(result.price, 18.25);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_backend_service_with_forwarding_rule_and_data() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .forwarding_rules(1)
            .data_processed_gb(1000)
            .fetch_monthly()
            .unwrap();

        // Cost = ($0.025 * 730) + ($0.008 * 1000) = $18.25 + $8.00 = $26.25/month
        assert_eq!(result.price, 26.25);
        assert_eq!(result.unit, "month");
    }

    #[test]
    fn test_blocking_gcp_backend_service_override_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .backend_service(BackendServiceTier::Premium)
            .region("us-central1")
            .override_default(0.015)
            .fetch()
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.015);
    }
}
