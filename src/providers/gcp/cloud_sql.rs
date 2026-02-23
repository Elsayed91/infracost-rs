//! GCP Cloud SQL pricing.
//!
//! Supports custom instances with separate CPU and RAM pricing, plus storage
//! and backup components. Handles MySQL, PostgreSQL, and SQL Server engines
//! with Zonal and Regional (HA) availability types.
//!
//! # Per-unit pricing (CPU hourly rate)
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::{CloudSqlEngine, CloudSqlAvailability};
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let price = client.gcp().cloud_sql()
//!     .engine(CloudSqlEngine::PostgreSql)
//!     .fetch().await?;
//! println!("${}/hour per vCPU", price.price);
//! # Ok(())
//! # }
//! ```
//!
//! # Total monthly cost
//! ```rust,no_run
//! # use infracost_rs::Client;
//! # use infracost_rs::providers::gcp::{CloudSqlEngine, CloudSqlAvailability};
//! # async fn example() -> infracost_rs::Result<()> {
//! let client = Client::new("api-key");
//! let cost = client.gcp().cloud_sql()
//!     .engine(CloudSqlEngine::PostgreSql)
//!     .availability(CloudSqlAvailability::Regional)
//!     .cpu_count(4)
//!     .memory_gb(16)
//!     .storage_gb(100)
//!     .backup_storage_gb(50)
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

/// Cloud SQL database engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudSqlEngine {
    /// MySQL database engine
    MySql,
    /// PostgreSQL database engine
    PostgreSql,
    /// SQL Server database engine
    SqlServer,
}

impl CloudSqlEngine {
    /// Get the engine name as it appears in GCP API descriptions.
    fn api_name(&self) -> &'static str {
        match self {
            Self::MySql => "MySQL",
            Self::PostgreSql => "PostgreSQL",
            Self::SqlServer => "SQL Server",
        }
    }
}

impl From<&str> for CloudSqlEngine {
    fn from(s: &str) -> Self {
        match s.to_lowercase().replace(['-', '_', ' '], "").as_str() {
            "mysql" => Self::MySql,
            "postgresql" | "postgres" => Self::PostgreSql,
            "sqlserver" | "mssql" => Self::SqlServer,
            _ => Self::PostgreSql, // Default to PostgreSQL
        }
    }
}

impl From<String> for CloudSqlEngine {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Cloud SQL availability type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudSqlAvailability {
    /// Zonal instance (single zone)
    Zonal,
    /// Regional instance (high availability, 2x price)
    Regional,
}

impl CloudSqlAvailability {
    /// Get the availability type as it appears in GCP API descriptions.
    fn api_name(&self) -> &'static str {
        match self {
            Self::Zonal => "Zonal",
            Self::Regional => "Regional",
        }
    }
}

impl From<&str> for CloudSqlAvailability {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "regional" | "ha" | "high_availability" => Self::Regional,
            _ => Self::Zonal,
        }
    }
}

impl From<String> for CloudSqlAvailability {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

// ============================================================
// Builder
// ============================================================

/// Builder for querying GCP Cloud SQL prices.
pub struct CloudSqlBuilder {
    client: Client,
    region: Option<String>,
    api_key: Option<String>,
    override_default: Option<f64>,
    engine: CloudSqlEngine,
    availability: CloudSqlAvailability,
    cpu_count: Option<u64>,
    memory_gb: Option<u64>,
    storage_gb: Option<u64>,
    backup_storage_gb: Option<u64>,
}

impl CloudSqlBuilder {
    /// Create a new Cloud SQL builder.
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            region: None,
            api_key: None,
            override_default: None,
            engine: CloudSqlEngine::PostgreSql,
            availability: CloudSqlAvailability::Zonal,
            cpu_count: None,
            memory_gb: None,
            storage_gb: None,
            backup_storage_gb: None,
        }
    }

    /// Set the GCP region (e.g., "us-central1").
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the API key for this request.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Override the default fallback price for the primary (CPU) component.
    pub fn override_default(mut self, price: f64) -> Self {
        self.override_default = Some(price);
        self
    }

    /// Set the database engine (MySQL, PostgreSQL, SQL Server).
    ///
    /// Defaults to PostgreSQL.
    pub fn engine(mut self, engine: impl Into<CloudSqlEngine>) -> Self {
        self.engine = engine.into();
        self
    }

    /// Set the availability type (Zonal or Regional/HA).
    ///
    /// Regional provides high availability at approximately 2x the cost.
    /// Defaults to Zonal.
    pub fn availability(mut self, availability: impl Into<CloudSqlAvailability>) -> Self {
        self.availability = availability.into();
        self
    }

    /// Set the number of vCPUs.
    pub fn cpu_count(mut self, count: u64) -> Self {
        self.cpu_count = Some(count);
        self
    }

    /// Set the amount of RAM in GiB.
    pub fn memory_gb(mut self, gb: u64) -> Self {
        self.memory_gb = Some(gb);
        self
    }

    /// Set the SSD storage size in GiB.
    pub fn storage_gb(mut self, gb: u64) -> Self {
        self.storage_gb = Some(gb);
        self
    }

    /// Set the backup storage size in GiB.
    pub fn backup_storage_gb(mut self, gb: u64) -> Self {
        self.backup_storage_gb = Some(gb);
        self
    }

    /// Build the string parameters for template substitution.
    fn string_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert(
            "engine_name".to_string(),
            self.engine.api_name().to_string(),
        );
        params.insert(
            "availability".to_string(),
            self.availability.api_name().to_string(),
        );
        params
    }

    /// Fetch just the price value (CPU hourly rate per vCPU).
    pub async fn fetch_price(self) -> Result<f64> {
        self.fetch().await.map(|r| r.price)
    }

    /// Fetch the full price result including source information.
    /// Returns the primary (CPU) hourly rate per vCPU.
    pub async fn fetch(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("cloud-sql")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);
        let string_params = self.string_params();

        let default_price = self.override_default.unwrap_or(0.0413);

        let component = &resource.cost_components[0]; // CPU component is primary
        PricingEngine::fetch_component_price(
            &self.client,
            component,
            "gcp",
            region,
            self.api_key.as_deref(),
            default_price,
            Some(&string_params),
        )
        .await
    }

    /// Fetch total monthly cost based on instance specs.
    ///
    /// Requires `cpu_count()` and `memory_gb()` to be set.
    ///
    /// The calculation:
    /// - CPU cost = cpu_hourly_price * 730 * cpu_count
    /// - RAM cost = ram_hourly_price * 730 * memory_gb
    /// - Storage cost = storage_price * storage_gb
    /// - Backup cost = backup_price * backup_storage_gb
    /// - IP cost = ip_price * 730 (hourly to monthly)
    /// - Total = CPU + RAM + Storage + Backup + IP
    pub async fn fetch_monthly(self) -> Result<PriceResult> {
        let resource = gcp_catalog().find("cloud-sql")?;
        let region = self.region.as_deref().unwrap_or(&resource.default_region);

        let cpu_count = self
            .cpu_count
            .ok_or_else(|| crate::Error::validation("cpu_count is required for fetch_monthly"))?;
        let memory_gb = self
            .memory_gb
            .ok_or_else(|| crate::Error::validation("memory_gb is required for fetch_monthly"))?;

        let string_params = self.string_params();

        // Build numeric params for the pricing engine.
        // CPU and RAM are hourly prices, so multiply quantities by 730 for monthly.
        let mut params = HashMap::new();
        params.insert("cpu_count".to_string(), cpu_count * 730);
        params.insert("memory_gb".to_string(), memory_gb * 730);
        params.insert("storage_gb".to_string(), self.storage_gb.unwrap_or(0));
        params.insert(
            "backup_storage_gb".to_string(),
            self.backup_storage_gb.unwrap_or(0),
        );

        let mut default_overrides = HashMap::new();
        if let Some(override_price) = self.override_default {
            default_overrides.insert("cpu".to_string(), override_price);
        }

        PricingEngine::fetch_monthly_with_string_params(
            &self.client,
            resource,
            "gcp",
            region,
            self.api_key.as_deref(),
            &params,
            Some(&string_params),
            if default_overrides.is_empty() {
                None
            } else {
                Some(&default_overrides)
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    #[test]
    fn test_engine_from_str() {
        assert_eq!(CloudSqlEngine::from("mysql"), CloudSqlEngine::MySql);
        assert_eq!(CloudSqlEngine::from("MySQL"), CloudSqlEngine::MySql);
        assert_eq!(
            CloudSqlEngine::from("postgresql"),
            CloudSqlEngine::PostgreSql
        );
        assert_eq!(CloudSqlEngine::from("postgres"), CloudSqlEngine::PostgreSql);
        assert_eq!(
            CloudSqlEngine::from("sql-server"),
            CloudSqlEngine::SqlServer
        );
        assert_eq!(CloudSqlEngine::from("sqlserver"), CloudSqlEngine::SqlServer);
        assert_eq!(CloudSqlEngine::from("mssql"), CloudSqlEngine::SqlServer);
    }

    #[test]
    fn test_availability_from_str() {
        assert_eq!(
            CloudSqlAvailability::from("zonal"),
            CloudSqlAvailability::Zonal
        );
        assert_eq!(
            CloudSqlAvailability::from("regional"),
            CloudSqlAvailability::Regional
        );
        assert_eq!(
            CloudSqlAvailability::from("ha"),
            CloudSqlAvailability::Regional
        );
    }

    #[test]
    fn test_engine_api_names() {
        assert_eq!(CloudSqlEngine::MySql.api_name(), "MySQL");
        assert_eq!(CloudSqlEngine::PostgreSql.api_name(), "PostgreSQL");
        assert_eq!(CloudSqlEngine::SqlServer.api_name(), "SQL Server");
    }

    #[test]
    fn test_availability_api_names() {
        assert_eq!(CloudSqlAvailability::Zonal.api_name(), "Zonal");
        assert_eq!(CloudSqlAvailability::Regional.api_name(), "Regional");
    }

    #[tokio::test]
    async fn test_cloud_sql_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0413);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_cloud_sql_mysql_returns_default() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::MySql)
            .region("us-central1")
            .fetch()
            .await
            .unwrap();

        assert!(result.is_from_default());
        assert_eq!(result.price, 0.0413);
        assert_eq!(result.unit, "hour");
    }

    #[tokio::test]
    async fn test_cloud_sql_fetch_monthly_cpu_and_ram_only() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .cpu_count(4)
            .memory_gb(16)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // CPU cost = 4 * $0.0413 * 730 = $120.596
        // RAM cost = 16 * $0.007 * 730 = $81.76
        // Storage cost = 0 (not set)
        // Backup cost = 0 (not set)
        // IP cost = $0.01 * 730 = $7.30
        let expected = (4.0 * 0.0413 * 730.0) + (16.0 * 0.007 * 730.0) + (0.01 * 730.0);
        assert!(
            (result.price - expected).abs() < 0.01,
            "Expected {expected}, got {}",
            result.price
        );
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_cloud_sql_fetch_monthly_with_storage() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .cpu_count(2)
            .memory_gb(8)
            .storage_gb(100)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // CPU cost = 2 * $0.0413 * 730 = $60.298
        // RAM cost = 8 * $0.007 * 730 = $40.88
        // Storage cost = 100 * $0.17 = $17.0
        // Backup cost = 0 (not set)
        // IP cost = $0.01 * 730 = $7.30
        let expected =
            (2.0 * 0.0413 * 730.0) + (8.0 * 0.007 * 730.0) + (100.0 * 0.17) + (0.01 * 730.0);
        assert!(
            (result.price - expected).abs() < 0.01,
            "Expected {expected}, got {}",
            result.price
        );
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_cloud_sql_fetch_monthly_full() {
        let client = Client::anonymous();
        let result = client
            .gcp()
            .cloud_sql()
            .engine(CloudSqlEngine::PostgreSql)
            .availability(CloudSqlAvailability::Zonal)
            .cpu_count(4)
            .memory_gb(16)
            .storage_gb(100)
            .backup_storage_gb(50)
            .fetch_monthly()
            .await
            .unwrap();

        assert!(result.is_from_default());
        // CPU cost = 4 * $0.0413 * 730 = $120.596
        // RAM cost = 16 * $0.007 * 730 = $81.76
        // Storage cost = 100 * $0.17 = $17.0
        // Backup cost = 50 * $0.08 = $4.0
        // IP cost = $0.01 * 730 = $7.30
        let expected = (4.0 * 0.0413 * 730.0)
            + (16.0 * 0.007 * 730.0)
            + (100.0 * 0.17)
            + (50.0 * 0.08)
            + (0.01 * 730.0);
        assert!(
            (result.price - expected).abs() < 0.01,
            "Expected {expected}, got {}",
            result.price
        );
        assert_eq!(result.unit, "month");
    }

    #[tokio::test]
    async fn test_cloud_sql_requires_cpu_count_for_monthly() {
        let client = Client::anonymous();
        let result = client.gcp().cloud_sql().memory_gb(16).fetch_monthly().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cloud_sql_requires_memory_gb_for_monthly() {
        let client = Client::anonymous();
        let result = client.gcp().cloud_sql().cpu_count(4).fetch_monthly().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cloud_sql_string_params() {
        let builder = CloudSqlBuilder::new(Client::anonymous());
        let builder = builder
            .engine(CloudSqlEngine::SqlServer)
            .availability(CloudSqlAvailability::Regional);
        let params = builder.string_params();
        assert_eq!(params.get("engine_name").unwrap(), "SQL Server");
        assert_eq!(params.get("availability").unwrap(), "Regional");
    }
}
