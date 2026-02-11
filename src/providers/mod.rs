//! Cloud provider convenience APIs with built-in defaults.
//!
//! This module provides fluent APIs for querying common cloud resource prices
//! with sensible defaults when no API key is available.
//!
//! # Example
//!
//! ```no_run
//! use infracost_rs::Client;
//! use infracost_rs::providers::gcp::DiskType;
//! use infracost_rs::providers::aws::EbsType;
//! use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::anonymous();
//!
//! // GCP - returns built-in default when no API key
//! let price = client
//!     .gcp()
//!     .disk(DiskType::PdSsd)
//!     .region("us-central1")
//!     .fetch_price()
//!     .await?;
//!
//! // AWS - returns built-in default when no API key
//! let price = client
//!     .aws()
//!     .ebs(EbsType::Gp3)
//!     .region("us-east-1")
//!     .fetch_price()
//!     .await?;
//!
//! // Azure - returns built-in default when no API key
//! let price = client
//!     .azure()
//!     .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
//!     .region("eastus")
//!     .fetch_price()
//!     .await?;
//! # Ok(())
//! # }
//! ```

pub mod aws;
pub mod azure;
pub mod gcp;
pub(crate) mod macros;

/// Result of a price query, including the source of the price.
#[derive(Debug, Clone)]
pub struct PriceResult {
    /// The price value
    pub price: f64,
    /// The unit of the price (e.g., "GB-month", "month", "hours")
    pub unit: String,
    /// Where the price came from
    pub source: PriceSource,
}

/// Indicates where a price value originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceSource {
    /// Price was fetched from the Infracost API
    Api,
    /// Price is a built-in default (API not called or failed)
    Default,
    /// Price is a user-provided override
    UserOverride,
}

impl PriceResult {
    /// Create a PriceResult from an API response
    pub(crate) fn from_api(price: f64, unit: &str) -> Self {
        Self {
            price,
            unit: unit.to_string(),
            source: PriceSource::Api,
        }
    }

    /// Create a PriceResult from a built-in default
    pub(crate) fn from_default(price: f64, unit: &str) -> Self {
        Self {
            price,
            unit: unit.to_string(),
            source: PriceSource::Default,
        }
    }

    /// Create a PriceResult from a user override
    #[allow(dead_code)]
    pub(crate) fn from_user_override(price: f64, unit: &str) -> Self {
        Self {
            price,
            unit: unit.to_string(),
            source: PriceSource::UserOverride,
        }
    }

    /// Returns true if the price was fetched from the API
    pub fn is_from_api(&self) -> bool {
        self.source == PriceSource::Api
    }

    /// Returns true if the price is a built-in default
    pub fn is_from_default(&self) -> bool {
        self.source == PriceSource::Default
    }

    /// Returns true if the price is a user-provided override
    pub fn is_from_user_override(&self) -> bool {
        self.source == PriceSource::UserOverride
    }
}

impl std::fmt::Display for PriceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:.4}/{}", self.price, self.unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_result_from_api() {
        let result = PriceResult::from_api(0.17, "GB-month");
        assert_eq!(result.price, 0.17);
        assert_eq!(result.unit, "GB-month");
        assert!(result.is_from_api());
        assert!(!result.is_from_default());
    }

    #[test]
    fn test_price_result_from_default() {
        let result = PriceResult::from_default(0.04, "GB-month");
        assert!(result.is_from_default());
        assert!(!result.is_from_api());
    }

    #[test]
    fn test_price_result_display() {
        let result = PriceResult::from_api(0.17, "GB-month");
        assert_eq!(format!("{}", result), "$0.1700/GB-month");
    }
}
