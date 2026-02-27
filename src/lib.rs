//! Rust client for the Infracost Cloud Pricing API.
//!
//! # Basic Usage
//!
//! ```no_run
//! use infracost_rs::Client;
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::from_env()?;
//! let products = client
//!     .products()
//!     .vendor("gcp")
//!     .service("Compute Engine")
//!     .region("us-central1")
//!     .fetch()
//!     .await?;
//! println!("${}", products[0].price_f64()?);
//! # Ok(())
//! # }
//! ```
//!
//! # Provider Convenience API
//!
//! For common cloud resources, use the provider-specific convenience methods
//! which include built-in default prices:
//!
//! ```no_run
//! use infracost_rs::Client;
//! use infracost_rs::providers::gcp::DiskType;
//!
//! # async fn example() -> Result<(), infracost_rs::Error> {
//! let client = Client::anonymous()?;
//!
//! // Returns built-in default ($0.17/GB-month) when no API key
//! let price = client
//!     .gcp()
//!     .disk(DiskType::PdSsd)
//!     .region("us-central1")
//!     .fetch_price()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Caching
//!
//! Enable caching to reduce API latency. Requires feature flags:
//!
//! ```toml
//! [dependencies]
//! infracost-rs = { version = "0.1", features = ["cache-memory"] }
//! ```
//!
//! ```ignore
//! use infracost_rs::{Client, MemoryCache};
//!
//! let client = Client::builder()
//!     .with_cache(MemoryCache::new())
//!     .build()?;
//! ```
//!
//! For shared caching across instances, use Redis:
//!
//! ```ignore
//! use infracost_rs::{Client, RedisCache};
//!
//! let client = Client::builder()
//!     .with_cache(RedisCache::new("redis://localhost:6379")?)
//!     .build()?;
//! ```

mod client;
mod error;
mod graphql;
mod query;
mod types;

pub mod cache;
pub mod catalog;
pub mod mock;
pub mod providers;

#[cfg(feature = "blocking")]
pub mod blocking;

#[cfg(feature = "cache-memory")]
pub use cache::MemoryCache;
#[cfg(feature = "cache-postgres")]
pub use cache::PostgresCache;
pub use cache::PriceCache;
#[cfg(feature = "cache-redis")]
pub use cache::RedisCache;
#[cfg(feature = "cache-sqlite")]
pub use cache::SqliteCache;
pub use client::{Client, ClientBuilder, DEFAULT_ENDPOINT, DEFAULT_TIMEOUT, PricingClient};
pub use error::{Error, Result};
pub use providers::{HOURS_PER_MONTH, PriceResult, PriceSource};
pub use query::ProductQueryBuilder;
pub use types::{
    Attribute, AttributeFilter, Price, PriceFilter, Product, ProductFilter, ProductFilterBuilder,
};
