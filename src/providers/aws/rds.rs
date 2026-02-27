//! AWS RDS (Relational Database Service) pricing.
//!
//! Supports instance compute pricing and storage pricing with multiple storage types.
//!
//! # Per-unit pricing (hourly rate)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let price = client.aws().rds("db.t3.micro")
//!     .fetch().await?;
//! println!("${}/hour", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost (instance + storage)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::aws::RdsStorageType;
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key")?;
//! let cost = client.aws().rds("db.t3.micro")
//!     .engine("mysql")
//!     .storage_type(RdsStorageType::Gp3)
//!     .allocated_storage_gb(100)
//!     .iops(6000)
//!     .storage_throughput_mbps(250)
//!     .fetch_monthly().await?;
//! println!("${}/month", cost.price);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::catalog::{aws_catalog, engine::PricingEngine};
use crate::{Client, Result};

use super::super::PriceResult;

// ============================================================
// Types
// ============================================================

/// RDS storage types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdsStorageType {
    /// General Purpose SSD (gp3) - baseline 3000 IOPS, 125 MBps throughput
    Gp3,
    /// General Purpose SSD (gp2) - burstable IOPS
    Gp2,
    /// Provisioned IOPS SSD (io1)
    Io1,
    /// Provisioned IOPS SSD (io2)
    Io2,
    /// Magnetic storage (previous generation)
    Magnetic,
}

impl RdsStorageType {
    /// Get the resource name for looking up in the YAML catalog.
    fn resource_name(&self) -> &'static str {
        match self {
            Self::Gp3 => "rds-storage/gp3",
            Self::Gp2 => "rds-storage/gp2",
            Self::Io1 => "rds-storage/io1",
            Self::Io2 => "rds-storage/io2",
            Self::Magnetic => "rds-storage/magnetic",
        }
    }

    /// Whether this storage type supports provisioned IOPS.
    fn supports_iops(&self) -> bool {
        matches!(self, Self::Gp3 | Self::Io1 | Self::Io2)
    }

    /// Whether this storage type supports provisioned throughput.
    fn supports_throughput(&self) -> bool {
        matches!(self, Self::Gp3)
    }

    /// Baseline IOPS included in the base price.
    fn baseline_iops(&self) -> u64 {
        match self {
            Self::Gp3 => 3000,
            _ => 0,
        }
    }

    /// Baseline throughput (MBps) included in the base price.
    fn baseline_throughput_mbps(&self) -> u64 {
        match self {
            Self::Gp3 => 125,
            _ => 0,
        }
    }
}

impl From<&str> for RdsStorageType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gp3" => Self::Gp3,
            "gp2" => Self::Gp2,
            "io1" => Self::Io1,
            "io2" => Self::Io2,
            "magnetic" | "standard" => Self::Magnetic,
            _ => Self::Gp3, // Default to gp3
        }
    }
}

impl From<String> for RdsStorageType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

// ============================================================
// Engine mapping
// ============================================================

/// Map user-friendly engine names to the API databaseEngine values.
fn map_engine(engine: &str) -> &'static str {
    match engine.to_lowercase().as_str() {
        "mysql" => "MySQL",
        "postgres" | "postgresql" => "PostgreSQL",
        "mariadb" => "MariaDB",
        "oracle-se2" | "oracle-se" | "oracle" => "Oracle",
        "oracle-ee" => "Oracle",
        "sqlserver-ee" | "sqlserver-se" | "sqlserver-ex" | "sqlserver-web" | "sql-server"
        | "sqlserver" => "SQL Server",
        "aurora-mysql" => "Aurora MySQL",
        "aurora-postgresql" | "aurora-postgres" => "Aurora PostgreSQL",
        other => {
            // If already in API format (e.g., "MySQL"), use as-is
            match other {
                "mysql" => "MySQL",
                "postgresql" => "PostgreSQL",
                _ => "MySQL", // fallback
            }
        }
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying AWS RDS prices.
pub struct RdsBuilder {
    client: Client,
    instance_class: String,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    engine: String,
    deployment_option: String,
    storage_type: RdsStorageType,
    // Monthly params
    allocated_storage_gb: Option<u64>,
    iops: Option<u64>,
    storage_throughput_mbps: Option<u64>,
}

impl RdsBuilder {
    /// Create a new RDS builder.
    pub(crate) fn new(client: Client, instance_class: impl Into<String>) -> Self {
        Self {
            client,
            instance_class: instance_class.into(),
            region: None,
            api_key: None,
            override_default: None,
            engine: "MySQL".to_string(),
            deployment_option: "Single-AZ".to_string(),
            storage_type: RdsStorageType::Gp3,
            allocated_storage_gb: None,
            iops: None,
            storage_throughput_mbps: None,
        }
    }

    /// Set the AWS region (e.g., "us-east-1").
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

    /// Set the database engine (e.g., "mysql", "postgres", "mariadb", "oracle-se2", "sqlserver-ee").
    ///
    /// The engine name is mapped to the API value automatically:
    /// - "mysql" -> "MySQL"
    /// - "postgres" -> "PostgreSQL"
    /// - "mariadb" -> "MariaDB"
    /// - "oracle-se2", "oracle-ee" -> "Oracle"
    /// - "sqlserver-ee", "sqlserver-se", etc. -> "SQL Server"
    /// - "aurora-mysql" -> "Aurora MySQL"
    /// - "aurora-postgresql" -> "Aurora PostgreSQL"
    pub fn engine(mut self, engine: impl Into<String>) -> Self {
        let engine_str = engine.into();
        self.engine = map_engine(&engine_str).to_string();
        self
    }

    /// Set the deployment option.
    ///
    /// Options: "Single-AZ" (default), "Multi-AZ"
    pub fn deployment_option(mut self, option: impl Into<String>) -> Self {
        self.deployment_option = option.into();
        self
    }

    /// Enable Multi-AZ deployment.
    pub fn multi_az(mut self) -> Self {
        self.deployment_option = "Multi-AZ".to_string();
        self
    }

    /// Set the storage type (default: Gp3).
    pub fn storage_type(mut self, storage_type: impl Into<RdsStorageType>) -> Self {
        self.storage_type = storage_type.into();
        self
    }

    /// Set the allocated storage in GB (required for `fetch_monthly` storage calculation).
    pub fn allocated_storage_gb(mut self, size: u64) -> Self {
        self.allocated_storage_gb = Some(size);
        self
    }

    /// Set provisioned IOPS (for gp3/io1/io2 storage types).
    ///
    /// For gp3: baseline 3000 IOPS is included; you only pay for IOPS above that.
    /// For io1/io2: all provisioned IOPS are billed.
    pub fn iops(mut self, iops: u64) -> Self {
        self.iops = Some(iops);
        self
    }

    /// Set provisioned throughput in MBps (for gp3 storage only).
    ///
    /// Baseline 125 MBps is included; you only pay for throughput above that.
    pub fn storage_throughput_mbps(mut self, throughput: u64) -> Self {
        self.storage_throughput_mbps = Some(throughput);
        self
    }

    /// Build the string parameters map for query attribute substitution.
    fn build_string_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("engine".to_string(), self.engine.clone());
        params.insert("instance_class".to_string(), self.instance_class.clone());
        params.insert(
            "deployment_option".to_string(),
            self.deployment_option.clone(),
        );
        params
    }

    /// Get the deployment option multiplier for default price fallbacks.
    /// Multi-AZ instances/storage cost 2x Single-AZ.
    fn deployment_multiplier(&self) -> f64 {
        if self.deployment_option == "Multi-AZ" {
            2.0
        } else {
            1.0
        }
    }

    /// Fetch just the price value (hourly instance rate).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    /// Returns the hourly on-demand instance price.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = aws_catalog().find("rds")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let string_params = self.build_string_params();

        let default_price = self
            .override_default
            .unwrap_or(resource.cost_components[0].default_price * self.deployment_multiplier());
        let component = &resource.cost_components[0];

        PricingEngine::fetch_component_price(
            &self.client,
            component,
            "aws",
            region,
            self.api_key.as_deref(),
            default_price,
            Some(&string_params),
        )
        .await
    }

    /// Fetch total monthly cost (instance + storage + IOPS + throughput).
    ///
    /// Instance cost is always included (hourly price * 730).
    /// Storage cost requires `allocated_storage_gb()` to be set.
    /// IOPS and throughput costs are included when the storage type supports them.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use infracost_rs::Client;
    /// # use infracost_rs::providers::aws::RdsStorageType;
    /// # async fn example() -> infracost_rs::Result<()> {
    /// let client = Client::new("api-key")?;
    /// let cost = client.aws().rds("db.t3.micro")
    ///     .engine("mysql")
    ///     .storage_type(RdsStorageType::Gp3)
    ///     .allocated_storage_gb(100)
    ///     .iops(6000)
    ///     .storage_throughput_mbps(250)
    ///     .fetch_monthly().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let string_params = self.build_string_params();
        let instance_resource = aws_catalog().find("rds")?;
        let region = self
            .region
            .as_deref()
            .unwrap_or(&instance_resource.default_region)
            .to_string();

        // 1. Fetch instance monthly cost (hourly * 730)
        let instance_params = HashMap::new(); // No quantity params needed for hourly_to_monthly
        let multiplier = self.deployment_multiplier();

        // Compute deployment-option-aware defaults (Multi-AZ = 2x Single-AZ)
        let mut instance_defaults = HashMap::new();
        instance_defaults.insert(
            "instance".to_string(),
            instance_resource.cost_components[0].default_price * multiplier,
        );

        let instance_result = PricingEngine::fetch_monthly_with_string_params(
            &self.client,
            instance_resource,
            "aws",
            &region,
            self.api_key.as_deref(),
            &instance_params,
            Some(&string_params),
            Some(&instance_defaults),
        )
        .await?;

        let mut total = instance_result.price;
        let mut all_from_api = instance_result.is_from_api();

        // 2. Fetch storage monthly cost (if allocated_storage_gb is set)
        if let Some(allocated_storage_gb) = self.allocated_storage_gb {
            let storage_resource = aws_catalog().find(self.storage_type.resource_name())?;

            let mut storage_params = HashMap::new();
            storage_params.insert("allocated_storage_gb".to_string(), allocated_storage_gb);

            // For storage types that support IOPS
            if self.storage_type.supports_iops() {
                let iops_val = self.iops.unwrap_or(self.storage_type.baseline_iops());
                storage_params.insert("iops".to_string(), iops_val);
            }

            // For storage types that support throughput
            if self.storage_type.supports_throughput() {
                let throughput_val = self
                    .storage_throughput_mbps
                    .unwrap_or(self.storage_type.baseline_throughput_mbps());
                storage_params.insert("storage_throughput_mbps".to_string(), throughput_val);
            }

            // Compute deployment-option-aware storage defaults
            let mut storage_defaults = HashMap::new();
            for comp in &storage_resource.cost_components {
                storage_defaults.insert(comp.name.clone(), comp.default_price * multiplier);
            }

            let storage_result = PricingEngine::fetch_monthly_with_string_params(
                &self.client,
                storage_resource,
                "aws",
                &region,
                self.api_key.as_deref(),
                &storage_params,
                Some(&string_params),
                Some(&storage_defaults),
            )
            .await?;

            total += storage_result.price;
            if !storage_result.is_from_api() {
                all_from_api = false;
            }
        }

        let source = if all_from_api {
            crate::providers::PriceSource::Api
        } else {
            crate::providers::PriceSource::Default
        };

        Ok(PriceResult {
            price: total,
            unit: "month".to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rds_storage_type_from_str() {
        assert_eq!(RdsStorageType::from("gp3"), RdsStorageType::Gp3);
        assert_eq!(RdsStorageType::from("GP3"), RdsStorageType::Gp3);
        assert_eq!(RdsStorageType::from("gp2"), RdsStorageType::Gp2);
        assert_eq!(RdsStorageType::from("io1"), RdsStorageType::Io1);
        assert_eq!(RdsStorageType::from("io2"), RdsStorageType::Io2);
        assert_eq!(RdsStorageType::from("magnetic"), RdsStorageType::Magnetic);
        assert_eq!(RdsStorageType::from("standard"), RdsStorageType::Magnetic);
        assert_eq!(RdsStorageType::from("unknown"), RdsStorageType::Gp3);
    }

    #[test]
    fn test_rds_storage_type_supports() {
        assert!(RdsStorageType::Gp3.supports_iops());
        assert!(RdsStorageType::Gp3.supports_throughput());
        assert!(!RdsStorageType::Gp2.supports_iops());
        assert!(!RdsStorageType::Gp2.supports_throughput());
        assert!(RdsStorageType::Io1.supports_iops());
        assert!(!RdsStorageType::Io1.supports_throughput());
        assert!(RdsStorageType::Io2.supports_iops());
        assert!(!RdsStorageType::Io2.supports_throughput());
        assert!(!RdsStorageType::Magnetic.supports_iops());
        assert!(!RdsStorageType::Magnetic.supports_throughput());
    }

    #[test]
    fn test_rds_storage_type_baselines() {
        assert_eq!(RdsStorageType::Gp3.baseline_iops(), 3000);
        assert_eq!(RdsStorageType::Gp3.baseline_throughput_mbps(), 125);
        assert_eq!(RdsStorageType::Io1.baseline_iops(), 0);
        assert_eq!(RdsStorageType::Io2.baseline_iops(), 0);
    }

    #[test]
    fn test_engine_mapping() {
        assert_eq!(map_engine("mysql"), "MySQL");
        assert_eq!(map_engine("MySQL"), "MySQL");
        assert_eq!(map_engine("postgres"), "PostgreSQL");
        assert_eq!(map_engine("postgresql"), "PostgreSQL");
        assert_eq!(map_engine("mariadb"), "MariaDB");
        assert_eq!(map_engine("oracle-se2"), "Oracle");
        assert_eq!(map_engine("oracle-ee"), "Oracle");
        assert_eq!(map_engine("sqlserver-ee"), "SQL Server");
        assert_eq!(map_engine("sqlserver-se"), "SQL Server");
        assert_eq!(map_engine("aurora-mysql"), "Aurora MySQL");
        assert_eq!(map_engine("aurora-postgresql"), "Aurora PostgreSQL");
    }

    #[tokio::test]
    async fn test_rds_returns_default_without_api_key() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .region("us-east-1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.017);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_instance_only() {
        // Instance only (no storage specified)
        // $0.017/hr * 730 = $12.41/month
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .region("us-east-1")
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        let expected = 0.017 * 730.0;
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_with_gp3_storage() {
        // Instance: $0.017/hr * 730 = $12.41
        // Storage: 100 GB * $0.115 = $11.50
        // IOPS: baseline 3000, no extra
        // Throughput: baseline 125, no extra
        // Total: $12.41 + $11.50 = $23.91
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        let expected = (0.017 * 730.0) + (100.0 * 0.115);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_gp3_with_extra_iops() {
        // Instance: $0.017/hr * 730 = $12.41
        // Storage: 100 GB * $0.115 = $11.50
        // IOPS: 6000 - 3000 baseline = 3000 extra * $0.02 = $60.00
        // Throughput: baseline 125, no extra
        // Total: $12.41 + $11.50 + $60.00 = $83.91
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .iops(6000)
            .fetch_monthly()
            .await
            .unwrap();

        let expected = (0.017 * 730.0) + (100.0 * 0.115) + (3000.0 * 0.02);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_gp3_with_extra_throughput() {
        // Instance: $0.017/hr * 730 = $12.41
        // Storage: 100 GB * $0.115 = $11.50
        // IOPS: baseline 3000, no extra
        // Throughput: 250 - 125 baseline = 125 extra * $0.08 = $10.00
        // Total: $12.41 + $11.50 + $10.00 = $33.91
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .storage_throughput_mbps(250)
            .fetch_monthly()
            .await
            .unwrap();

        let expected = (0.017 * 730.0) + (100.0 * 0.115) + (125.0 * 0.08);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_gp3_full_spec() {
        // Instance: $0.017/hr * 730 = $12.41
        // Storage: 100 GB * $0.115 = $11.50
        // IOPS: 6000 - 3000 = 3000 extra * $0.02 = $60.00
        // Throughput: 250 - 125 = 125 extra * $0.08 = $10.00
        // Total: $12.41 + $11.50 + $60.00 + $10.00 = $93.91
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .iops(6000)
            .storage_throughput_mbps(250)
            .fetch_monthly()
            .await
            .unwrap();

        let expected = (0.017 * 730.0) + (100.0 * 0.115) + (3000.0 * 0.02) + (125.0 * 0.08);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_gp2_storage() {
        // Instance: $0.017/hr * 730 = $12.41
        // Storage: 100 GB * $0.115 = $11.50
        // Total: $23.91
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Gp2)
            .allocated_storage_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        let expected = (0.017 * 730.0) + (100.0 * 0.115);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_io1_storage_with_iops() {
        // Instance: $0.017/hr * 730 = $12.41
        // Storage: 100 GB * $0.125 = $12.50
        // IOPS: 1000 * $0.10 = $100.00 (io1 has no baseline)
        // Total: $12.41 + $12.50 + $100.00 = $124.91
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Io1)
            .allocated_storage_gb(100)
            .iops(1000)
            .fetch_monthly()
            .await
            .unwrap();

        let expected = (0.017 * 730.0) + (100.0 * 0.125) + (1000.0 * 0.10);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_fetch_monthly_magnetic_storage() {
        // Instance: $0.017/hr * 730 = $12.41
        // Storage: 100 GB * $0.10 = $10.00
        // Total: $22.41
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Magnetic)
            .allocated_storage_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        let expected = (0.017 * 730.0) + (100.0 * 0.10);
        assert!((result.price - expected).abs() < 0.01);
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_rds_with_engine_setting() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .engine("postgres")
            .fetch()
            .await
            .unwrap();

        // Without API key, still returns default
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.017);
    }

    #[tokio::test]
    async fn test_rds_with_multi_az() {
        let client = Client::anonymous().unwrap();
        let result = client
            .aws()
            .rds("db.t3.micro")
            .multi_az()
            .fetch()
            .await
            .unwrap();

        // Multi-AZ default should be 2x Single-AZ ($0.017 * 2 = $0.034)
        assert!(result.is_from_default());
        assert_eq!(result.price, 0.034);
    }

    #[tokio::test]
    async fn test_rds_multi_az_fetch_monthly_uses_doubled_defaults() {
        let client = Client::anonymous().unwrap();

        let single_az = client
            .aws()
            .rds("db.t3.micro")
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        let multi_az = client
            .aws()
            .rds("db.t3.micro")
            .multi_az()
            .storage_type(RdsStorageType::Gp3)
            .allocated_storage_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        // Multi-AZ should be exactly 2x Single-AZ for defaults
        let ratio = multi_az.price / single_az.price;
        assert!(
            (ratio - 2.0).abs() < 0.01,
            "Multi-AZ/Single-AZ ratio should be ~2.0, got {ratio:.4} (multi={}, single={})",
            multi_az.price,
            single_az.price
        );
    }
}
