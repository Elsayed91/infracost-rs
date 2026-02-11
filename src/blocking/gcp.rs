//! Blocking GCP provider for querying GCP resource prices.
//!
//! This module provides synchronous wrappers around the async GCP provider API.
//! All builders construct the async builders on-the-fly and execute them using
//! the provided Tokio runtime.
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

use crate::error::Result;
use crate::providers::PriceResult;
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::blocking::Client;
    /// use infracost_rs::providers::gcp::DiskType;
    ///
    /// # fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .gcp()
    ///     .disk(DiskType::PdSsd)
    ///     .region("us-central1")
    ///     .fetch()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn disk(self, disk_type: impl Into<DiskType>) -> BlockingGcpDiskBuilder {
        BlockingGcpDiskBuilder {
            client: self.client,
            runtime: self.runtime,
            disk_type: disk_type.into(),
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
            iops: None,
            throughput_mb_per_sec: None,
            regional: false,
        }
    }

    /// Parse a GCP disk JSON (from `gcloud compute disks describe --format=json`) into a blocking DiskBuilder.
    pub fn disk_from_json(self, json: &serde_json::Value) -> crate::Result<BlockingGcpDiskBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_disk_json(json)?;
        Ok(BlockingGcpDiskBuilder {
            client: self.client,
            runtime: self.runtime,
            disk_type: parsed.disk_type,
            region: parsed.region,
            api_key: None,
            override_default: None,
            size_gb: parsed.size_gb,
            iops: parsed.iops,
            throughput_mb_per_sec: parsed.throughput,
            regional: parsed.regional,
        })
    }

    /// Parse a GCP snapshot JSON (from `gcloud compute snapshots describe --format=json`) into a blocking SnapshotBuilder.
    pub fn snapshot_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpSnapshotBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_snapshot_json(json)?;
        Ok(BlockingGcpSnapshotBuilder {
            client: self.client,
            runtime: self.runtime,
            region: parsed.region,
            api_key: None,
            override_default: None,
            size_gb: parsed.size_gb,
        })
    }

    /// Parse a GCP static IP JSON (from `gcloud compute addresses describe --format=json`) into a blocking StaticIpBuilder.
    pub fn static_ip_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpStaticIpBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_static_ip_json(json)?;
        Ok(BlockingGcpStaticIpBuilder {
            client: self.client,
            runtime: self.runtime,
            region: parsed.region,
            api_key: None,
            override_default: None,
        })
    }

    /// Parse a GCP NAT gateway JSON into a blocking NatGatewayBuilder.
    pub fn nat_gateway_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpNatGatewayBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_nat_gateway_json(json)?;
        Ok(BlockingGcpNatGatewayBuilder {
            client: self.client,
            runtime: self.runtime,
            region: parsed.region,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
        })
    }

    /// Query GCP Snapshot pricing.
    ///
    /// Default: $0.05/GB-month
    pub fn snapshot(self) -> BlockingGcpSnapshotBuilder {
        BlockingGcpSnapshotBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            size_gb: None,
        }
    }

    /// Query GCP Static IP pricing.
    ///
    /// Default: $0.01/hour (~$7.30/month)
    pub fn static_ip(self) -> BlockingGcpStaticIpBuilder {
        BlockingGcpStaticIpBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
        }
    }

    /// Query GCP NAT Gateway uptime pricing.
    ///
    /// Default: $0.0014/hour (~$1.02/month)
    /// Note: Additional data processing charges apply ($0.045/GB)
    pub fn nat_gateway(self) -> BlockingGcpNatGatewayBuilder {
        BlockingGcpNatGatewayBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
        }
    }

    /// Query GCP Forwarding Rule (Load Balancer) pricing.
    ///
    /// Default: $0.025/hour (~$18.25/month)
    /// Note: Additional data processing charges apply
    pub fn forwarding_rule(self) -> BlockingGcpForwardingRuleBuilder {
        BlockingGcpForwardingRuleBuilder {
            client: self.client,
            runtime: self.runtime,
            region: None,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
        }
    }

    /// Query GCP Backend Service pricing.
    ///
    /// Backend services handle data processing for load balancers.
    /// - Premium tier (global): $0.008/GiB data processing
    /// - Standard tier (regional): $0.008/GiB data processing
    /// - Optionally include forwarding rule charges ($0.025/hour per rule)
    pub fn backend_service(
        self,
        tier: impl Into<BackendServiceTier>,
    ) -> BlockingGcpBackendServiceBuilder {
        BlockingGcpBackendServiceBuilder {
            client: self.client,
            runtime: self.runtime,
            tier: tier.into(),
            region: None,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
            forwarding_rules: None,
        }
    }

    /// Parse a GCP backend service JSON (from `gcloud compute backend-services describe --format=json`) into a blocking BackendServiceBuilder.
    pub fn backend_service_from_json(
        self,
        json: &serde_json::Value,
    ) -> crate::Result<BlockingGcpBackendServiceBuilder> {
        let parsed = crate::providers::gcp::from_json::parse_backend_service_json(json)?;
        Ok(BlockingGcpBackendServiceBuilder {
            client: self.client,
            runtime: self.runtime,
            tier: parsed.tier,
            region: parsed.region,
            api_key: None,
            override_default: None,
            data_processed_gb: None,
            forwarding_rules: None,
        })
    }
}

// ============================================================
// Disk Builder
// ============================================================

/// Blocking builder for querying GCP disk prices.
pub struct BlockingGcpDiskBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    disk_type: DiskType,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
    iops: Option<u64>,
    throughput_mb_per_sec: Option<u64>,
    regional: bool,
}

impl BlockingGcpDiskBuilder {
    /// Set the GCP region (e.g., "us-central1")
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the API key for this request.
    ///
    /// If not set and the client has no default key, returns built-in defaults.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the default fallback price.
    ///
    /// By default, the library uses built-in prices when the API is unavailable.
    /// Use this to specify a custom fallback.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set the disk size in GB (required for `fetch_monthly`).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Set provisioned IOPS (for pd-extreme disks only).
    ///
    /// For pd-extreme: all provisioned IOPS are billed at $0.065/IOPS-month.
    /// For other disk types: IOPS is ignored (not supported).
    pub fn iops(mut self, iops: u64) -> Self {
        self.iops = Some(iops);
        self
    }

    /// Set provisioned throughput in MiB/s (for Hyperdisk types).
    pub fn throughput(mut self, mb_per_sec: u64) -> Self {
        self.throughput_mb_per_sec = Some(mb_per_sec);
        self
    }

    /// Set whether this is a regional disk (replicated across zones, 2x price).
    pub fn regional(mut self, regional: bool) -> Self {
        self.regional = regional;
        self
    }

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().disk(self.disk_type);
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
        if let Some(v) = self.throughput_mb_per_sec {
            b = b.throughput(v);
        }
        if self.regional {
            b = b.regional(true);
        }
        self.runtime.block_on(b.fetch())
    }

    /// Fetch just the price value.
    pub fn fetch_price(self) -> Result<f64> {
        self.fetch().map(|r| r.price)
    }

    /// Fetch total monthly cost based on disk specs.
    ///
    /// Requires `size_gb()` to be set. Optionally set `iops()` and `throughput()`.
    /// Regional disks cost 2x.
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().disk(self.disk_type);
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
        if let Some(v) = self.throughput_mb_per_sec {
            b = b.throughput(v);
        }
        if self.regional {
            b = b.regional(true);
        }
        self.runtime.block_on(b.fetch_monthly())
    }
}

// ============================================================
// Snapshot Builder
// ============================================================

/// Blocking builder for querying GCP snapshot prices.
pub struct BlockingGcpSnapshotBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    size_gb: Option<u64>,
}

impl BlockingGcpSnapshotBuilder {
    /// Set the GCP region (e.g., "us-central1")
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

    /// Set the snapshot size in GB (required for fetch_monthly).
    pub fn size_gb(mut self, size: u64) -> Self {
        self.size_gb = Some(size);
        self
    }

    /// Fetch the full price result including source information.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().snapshot();
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

    /// Fetch monthly cost (rate × size_gb).
    /// Requires size_gb to be set.
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().snapshot();
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
// Static IP Builder
// ============================================================

/// Blocking builder for querying GCP static IP prices.
pub struct BlockingGcpStaticIpBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
}

impl BlockingGcpStaticIpBuilder {
    /// Set the GCP region (e.g., "us-central1")
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
        let mut b = self.client.gcp().static_ip();
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

    /// Fetch monthly cost (hourly rate × 730 hours).
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().static_ip();
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
// NAT Gateway Builder
// ============================================================

/// Blocking builder for querying GCP NAT Gateway prices.
pub struct BlockingGcpNatGatewayBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    data_processed_gb: Option<u64>,
}

impl BlockingGcpNatGatewayBuilder {
    /// Set the GCP region (e.g., "us-central1")
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

    /// Set the amount of data processed in GB per month (required for `fetch_monthly`).
    pub fn data_processed_gb(mut self, gb: u64) -> Self {
        self.data_processed_gb = Some(gb);
        self
    }

    /// Fetch the full price result including source information.
    /// Returns the hourly uptime charge only (no data processing).
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().nat_gateway();
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

    /// Fetch total monthly cost based on data processing usage.
    ///
    /// Calculates: (hourly_rate * 730 hours) + (data_rate * gb_processed)
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().nat_gateway();
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
// Forwarding Rule Builder
// ============================================================

/// Blocking builder for querying GCP Forwarding Rule prices.
pub struct BlockingGcpForwardingRuleBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    data_processed_gb: Option<u64>,
}

impl BlockingGcpForwardingRuleBuilder {
    /// Set the GCP region (e.g., "us-central1")
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

    /// Set the amount of data processed in GB per month (required for `fetch_monthly`).
    pub fn data_processed_gb(mut self, gb: u64) -> Self {
        self.data_processed_gb = Some(gb);
        self
    }

    /// Fetch the full price result including source information.
    /// Returns the hourly uptime charge only (no data processing).
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().forwarding_rule();
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

    /// Fetch total monthly cost based on data processing usage.
    ///
    /// Calculates: (hourly_rate * 730 hours) + (data_rate * gb_processed)
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().forwarding_rule();
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
// Backend Service Builder
// ============================================================

/// Blocking builder for querying GCP Backend Service prices.
pub struct BlockingGcpBackendServiceBuilder {
    client: crate::Client,
    runtime: Arc<tokio::runtime::Runtime>,
    tier: BackendServiceTier,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    data_processed_gb: Option<u64>,
    forwarding_rules: Option<u64>,
}

impl BlockingGcpBackendServiceBuilder {
    /// Set the GCP region (e.g., "us-central1")
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

    /// Set the amount of data processed in GB per month (required for `fetch_monthly`).
    pub fn data_processed_gb(mut self, gb: u64) -> Self {
        self.data_processed_gb = Some(gb);
        self
    }

    /// Include forwarding rule hourly charges in `fetch_monthly`.
    ///
    /// GCP load balancers require at least one forwarding rule ($0.025/hour).
    /// This adds the forwarding rule cost to the monthly total.
    pub fn forwarding_rules(mut self, count: u64) -> Self {
        self.forwarding_rules = Some(count);
        self
    }

    /// Fetch the full price result including source information.
    /// Returns the per-GiB data processing rate.
    pub fn fetch(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().backend_service(self.tier);
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

    /// Fetch total monthly cost based on data processing and forwarding rules.
    ///
    /// Calculates: (forwarding_rule_hourly * 730 * count) + (data_rate * gb_processed)
    pub fn fetch_monthly(self) -> Result<PriceResult> {
        let mut b = self.client.gcp().backend_service(self.tier);
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
        if let Some(v) = self.forwarding_rules {
            b = b.forwarding_rules(v);
        }
        self.runtime.block_on(b.fetch_monthly())
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
