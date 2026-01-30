//! Rust client for the Infracost Cloud Pricing API.
//!
//! ```no_run
//! use infracost::Client;
//!
//! # async fn example() -> Result<(), infracost::Error> {
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

mod client;
mod error;
mod graphql;
mod query;
mod types;

pub mod mock;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use client::{Client, ClientBuilder, DEFAULT_ENDPOINT, DEFAULT_TIMEOUT, PricingClient};
pub use error::{Error, Result};
pub use query::ProductQueryBuilder;
pub use types::{
    Attribute, AttributeFilter, Price, PriceFilter, Product, ProductFilter, ProductFilterBuilder,
};
