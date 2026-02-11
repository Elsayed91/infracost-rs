//! GCP Compute Instance pricing.
//!
//! Supports both per-unit pricing and total monthly cost calculation.
//! Compute instances are priced by CPU cores and RAM separately.
//!
//! # Per-unit pricing (CPU hourly rate)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::MachineFamily;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.gcp().compute_instance(MachineFamily::N2).fetch().await?;
//! println!("${}/hour per core", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost with specs
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::MachineFamily;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.gcp().compute_instance(MachineFamily::N2)
//!     .cpu_cores(4)
//!     .memory_gib(16)
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::catalog::{engine::PricingEngine, gcp_catalog};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Types
// ============================================================

/// GCP Compute Instance machine families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineFamily {
    /// N2 (2nd Gen Intel) On-Demand
    N2,
    /// N2 (2nd Gen Intel) Spot/Preemptible
    N2Spot,
    /// E2 (Cost-optimized) On-Demand
    E2,
    /// E2 (Cost-optimized) Spot/Preemptible
    E2Spot,
}

impl MachineFamily {
    /// Get the YAML catalog resource name for this machine family.
    fn resource_name(&self) -> &'static str {
        match self {
            Self::N2 => "compute-instance/n2",
            Self::N2Spot => "compute-instance/n2-spot",
            Self::E2 => "compute-instance/e2",
            Self::E2Spot => "compute-instance/e2-spot",
        }
    }

    /// Get the default CPU price per hour for this machine family.
    pub fn default_cpu_price(&self) -> f64 {
        match self {
            Self::N2 => 0.031611,
            Self::N2Spot => 0.00985,
            Self::E2 => 0.02181159,
            Self::E2Spot => 0.01007,
        }
    }

    /// Get the default RAM price per GiB-hour for this machine family.
    pub fn default_ram_price(&self) -> f64 {
        match self {
            Self::N2 => 0.004237,
            Self::N2Spot => 0.001318,
            Self::E2 => 0.00292353,
            Self::E2Spot => 0.00135,
        }
    }
}

impl From<&str> for MachineFamily {
    fn from(s: &str) -> Self {
        match s.to_lowercase().replace(['-', '_'], "").as_str() {
            "n2spot" | "n2preemptible" => Self::N2Spot,
            "e2" => Self::E2,
            "e2spot" | "e2preemptible" => Self::E2Spot,
            _ => Self::N2,
        }
    }
}

impl From<String> for MachineFamily {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP Compute Instance prices.
pub struct ComputeInstanceBuilder {
    client: Client,
    machine_family: MachineFamily,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    cpu_cores: Option<u64>,
    memory_gib: Option<u64>,
}

impl ComputeInstanceBuilder {
    /// Create a new compute instance builder.
    pub(crate) fn new(client: Client, machine_family: MachineFamily) -> Self {
        Self {
            client,
            machine_family,
            region: None,
            api_key: None,
            override_default: None,
            cpu_cores: None,
            memory_gib: None,
        }
    }

    /// Set the GCP region (e.g., "us-central1").
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

    /// Override the default fallback price for the primary (CPU) component.
    ///
    /// By default, the library uses built-in prices when the API is unavailable.
    /// Use this to specify a custom fallback.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set the number of CPU cores (required for `fetch_monthly`).
    pub fn cpu_cores(mut self, cores: u64) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Set the amount of memory in GiB (required for `fetch_monthly`).
    pub fn memory_gib(mut self, gib: u64) -> Self {
        self.memory_gib = Some(gib);
        self
    }

    /// Fetch just the price value (CPU hourly rate per core).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    /// Returns the primary (CPU) hourly rate per core.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find(self.machine_family.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        PricingEngine::fetch(
            &self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            self.override_default,
        )
        .await
    }

    /// Fetch total monthly cost based on instance specs.
    ///
    /// Requires `cpu_cores()` and `memory_gib()` to be set.
    ///
    /// The calculation:
    /// - CPU cost = cpu_hourly_price * 730 * cpu_cores
    /// - RAM cost = ram_hourly_price * 730 * memory_gib
    /// - Total = CPU cost + RAM cost
    ///
    /// # Examples
    ///
    /// N2 standard-4 (4 vCPUs, 16 GiB RAM):
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # use infracost_rs::providers::gcp::MachineFamily;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key");
    /// let cost = client.gcp().compute_instance(MachineFamily::N2)
    ///     .cpu_cores(4)
    ///     .memory_gib(16)
    ///     .fetch_monthly().await?;
    /// // Cost = (4 * $0.031611 * 730) + (16 * $0.004237 * 730)
    /// //      = $92.304 + $49.487 = $141.79/month
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let cpu_cores = self
            .cpu_cores
            .ok_or_else(|| crate::Error::validation("cpu_cores is required for fetch_monthly"))?;
        let memory_gib = self
            .memory_gib
            .ok_or_else(|| crate::Error::validation("memory_gib is required for fetch_monthly"))?;

        let resource = gcp_catalog().find(self.machine_family.resource_name())?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);

        // The YAML uses linear pricing (price * quantity), but we need
        // price * quantity * 730 since prices are hourly. We multiply each
        // quantity by 730 before passing to the engine.
        let mut params = HashMap::new();
        params.insert("cpu_cores".to_string(), cpu_cores * 730);
        params.insert("memory_gib".to_string(), memory_gib * 730);

        PricingEngine::fetch_monthly(
            &self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            &params,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    #[test]
    fn test_machine_family_from_str() {
        assert_eq!(MachineFamily::from("n2"), MachineFamily::N2);
        assert_eq!(MachineFamily::from("N2"), MachineFamily::N2);
        assert_eq!(MachineFamily::from("n2-spot"), MachineFamily::N2Spot);
        assert_eq!(MachineFamily::from("n2_preemptible"), MachineFamily::N2Spot);
        assert_eq!(MachineFamily::from("e2"), MachineFamily::E2);
        assert_eq!(MachineFamily::from("E2"), MachineFamily::E2);
        assert_eq!(MachineFamily::from("e2-spot"), MachineFamily::E2Spot);
        assert_eq!(MachineFamily::from("e2_preemptible"), MachineFamily::E2Spot);
        // Unknown defaults to N2
        assert_eq!(MachineFamily::from("unknown"), MachineFamily::N2);
    }

    #[test]
    fn test_machine_family_defaults() {
        assert_eq!(MachineFamily::N2.default_cpu_price(), 0.031611);
        assert_eq!(MachineFamily::N2.default_ram_price(), 0.004237);
        assert_eq!(MachineFamily::N2Spot.default_cpu_price(), 0.00985);
        assert_eq!(MachineFamily::N2Spot.default_ram_price(), 0.001318);
        assert_eq!(MachineFamily::E2.default_cpu_price(), 0.02181159);
        assert_eq!(MachineFamily::E2.default_ram_price(), 0.00292353);
        assert_eq!(MachineFamily::E2Spot.default_cpu_price(), 0.01007);
        assert_eq!(MachineFamily::E2Spot.default_ram_price(), 0.00135);
    }

    #[tokio::test]
    async fn test_n2_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::N2)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.031611);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_e2_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::E2)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.02181159);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_n2_spot_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::N2Spot)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.00985);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_n2_fetch_monthly() {
        // N2 standard-4: 4 vCPUs, 16 GiB RAM
        // CPU cost = 4 * $0.031611 * 730 = $92.30412
        // RAM cost = 16 * $0.004237 * 730 = $49.48816
        // Total = $141.79228
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::N2)
            .cpu_cores(4)
            .memory_gib(16)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        let expected = (4.0 * 0.031611 * 730.0) + (16.0 * 0.004237 * 730.0);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_e2_fetch_monthly() {
        // E2 standard-2: 2 vCPUs, 8 GiB RAM
        // CPU cost = 2 * $0.02181159 * 730 = $31.844922
        // RAM cost = 8 * $0.00292353 * 730 = $17.073414
        // Total = $48.918336 (approx)
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::E2)
            .cpu_cores(2)
            .memory_gib(8)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        let expected = (2.0 * 0.02181159 * 730.0) + (8.0 * 0.00292353 * 730.0);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_n2_spot_fetch_monthly() {
        // N2 Spot standard-4: 4 vCPUs, 16 GiB RAM
        // CPU cost = 4 * $0.00985 * 730 = $28.762
        // RAM cost = 16 * $0.001318 * 730 = $15.394
        // Total = $44.156 (approx)
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::N2Spot)
            .cpu_cores(4)
            .memory_gib(16)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        let expected = (4.0 * 0.00985 * 730.0) + (16.0 * 0.001318 * 730.0);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_e2_spot_fetch_monthly() {
        // E2 Spot standard-2: 2 vCPUs, 8 GiB RAM
        // CPU cost = 2 * $0.01007 * 730 = $14.7022
        // RAM cost = 8 * $0.00135 * 730 = $7.884
        // Total = $22.5862
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::E2Spot)
            .cpu_cores(2)
            .memory_gib(8)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        let expected = (2.0 * 0.01007 * 730.0) + (8.0 * 0.00135 * 730.0);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_fetch_monthly_requires_cpu_cores() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::N2)
            .memory_gib(16)
            .fetch_monthly()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("cpu_cores is required"));
    }

    #[tokio::test]
    async fn test_fetch_monthly_requires_memory_gib() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::N2)
            .cpu_cores(4)
            .fetch_monthly()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("memory_gib is required"));
    }

    #[tokio::test]
    async fn test_override_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance(MachineFamily::N2)
            .region("us-central1")
            .override_default(0.05)
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.05);
    }

    #[tokio::test]
    async fn test_string_type() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .compute_instance("e2")
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert_eq!(result.price, 0.02181159);
    }
}
