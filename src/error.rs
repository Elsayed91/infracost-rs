//! Error types.

use thiserror::Error;

/// Error types for the Infracost client.
#[derive(Debug, Error)]
pub enum Error {
    /// API key not provided and INFRACOST_API_KEY not set
    #[error("API key not provided and INFRACOST_API_KEY not set")]
    MissingApiKey,

    /// HTTP error during request
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message from API
        message: String,
    },

    /// GraphQL query returned errors
    #[error("GraphQL error: {0}")]
    GraphQL(String),

    /// No products found matching the filter
    #[error("No products found")]
    NoProducts,

    /// No prices found for a product
    #[error("No prices found for product {sku}")]
    NoPrices {
        /// SKU of the product without prices
        sku: String,
    },

    /// Failed to parse a price value
    #[error("Failed to parse price '{value}': {reason}")]
    InvalidPrice {
        /// The price value that failed to parse
        value: String,
        /// Reason for parse failure
        reason: String,
    },

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("Configuration error: {0}")]
    Config(String),

    /// Validation error (e.g., missing required parameters)
    #[error("Validation error: {0}")]
    Validation(String),

    /// I/O error (for file operations)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create an API error with status and message
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }

    /// Create a GraphQL error
    pub fn graphql(message: impl Into<String>) -> Self {
        Self::GraphQL(message.into())
    }

    /// Create an invalid price error
    pub fn invalid_price(value: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidPrice {
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// Create a no prices error
    pub fn no_prices(sku: impl Into<String>) -> Self {
        Self::NoPrices { sku: sku.into() }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a no products error
    pub fn no_products() -> Self {
        Self::NoProducts
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }
}

/// Result type alias using the Infracost error type.
pub type Result<T> = std::result::Result<T, Error>;
