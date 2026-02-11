//! HTTP client for the Infracost API.
//!
//! Use [`Client::from_env`] to create a client from `INFRACOST_API_KEY`,
//! or [`Client::new`] with an explicit key.

use crate::cache::{DEFAULT_CACHE_TTL, PriceCache};
use crate::error::{Error, Result};
use crate::graphql::{GqlProductFilter, ProductQuery, ProductQueryVariables};
use crate::providers::aws::AwsProvider;
use crate::providers::azure::AzureProvider;
use crate::providers::gcp::GcpProvider;
use crate::query::ProductQueryBuilder;
use crate::types::{Product, ProductFilter};
use async_trait::async_trait;
use cynic::QueryBuilder;
use std::env;
use std::sync::Arc;
use std::time::Duration;

/// Default API endpoint for Infracost
pub const DEFAULT_ENDPOINT: &str = "https://pricing.api.infracost.io/graphql";

/// Default request timeout
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Trait for pricing clients (real and mock)
///
/// This trait allows for dependency injection and testing with mock clients.
/// Use `query_products()` for executing queries. The fluent builder pattern
/// via `products()` is available as an inherent method on `Client`.
#[async_trait]
pub trait PricingClient: Send + Sync {
    /// Execute a raw query with a filter
    async fn query_products(&self, filter: ProductFilter) -> Result<Vec<Product>>;

    /// Execute a raw query with a filter and optional API key override
    async fn query_products_with_key(
        &self,
        filter: ProductFilter,
        api_key: Option<&str>,
    ) -> Result<Vec<Product>>;
}

/// Internal client data shared between client instances
struct ClientInner {
    http: reqwest::Client,
    api_key: Option<String>,
    endpoint: String,
    error_on_fallback: bool,
    cache: Option<Arc<dyn PriceCache>>,
    cache_ttl: Duration,
}

/// Client for the Infracost Cloud Pricing API.
///
/// ```no_run
/// use infracost_rs::Client;
///
/// # async fn example() -> Result<(), infracost_rs::Error> {
/// let client = Client::from_env()?;        // from INFRACOST_API_KEY
/// let client = Client::new("ico-xxx");     // explicit key
/// let client = Client::anonymous();        // per-request key
///
/// let products = client
///     .products()
///     .vendor("gcp")
///     .service("Compute Engine")
///     .fetch()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("endpoint", &self.inner.endpoint)
            .field("has_api_key", &self.inner.api_key.is_some())
            .finish()
    }
}

impl Client {
    /// Create a new client with an API key.
    ///
    /// The API key is stored in the client and used for all requests
    /// unless overridden per-request.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                http: reqwest::Client::builder()
                    .timeout(DEFAULT_TIMEOUT)
                    .build()
                    .expect("Failed to build HTTP client"),
                api_key: Some(api_key.into()),
                endpoint: DEFAULT_ENDPOINT.to_string(),
                error_on_fallback: false,
                cache: None,
                cache_ttl: DEFAULT_CACHE_TTL,
            }),
        }
    }

    /// Create a new client from the `INFRACOST_API_KEY` environment variable.
    ///
    /// Returns an error if the environment variable is not set.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("INFRACOST_API_KEY").map_err(|_| Error::MissingApiKey)?;
        Ok(Self::new(api_key))
    }

    /// Create an anonymous client without an API key.
    ///
    /// You must provide an API key per-request when using this client.
    pub fn anonymous() -> Self {
        Self {
            inner: Arc::new(ClientInner {
                http: reqwest::Client::builder()
                    .timeout(DEFAULT_TIMEOUT)
                    .build()
                    .expect("Failed to build HTTP client"),
                api_key: None,
                endpoint: DEFAULT_ENDPOINT.to_string(),
                error_on_fallback: false,
                cache: None,
                cache_ttl: DEFAULT_CACHE_TTL,
            }),
        }
    }

    /// Create a new client builder for advanced configuration.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Get the API endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.inner.endpoint
    }

    /// Check if this client has a default API key
    pub fn has_api_key(&self) -> bool {
        self.inner.api_key.is_some()
    }

    /// Check if this client will error on fallback to defaults.
    pub fn error_on_fallback(&self) -> bool {
        self.inner.error_on_fallback
    }

    /// Start building a product query.
    ///
    /// This is a convenience method that provides direct access without
    /// requiring the `PricingClient` trait to be in scope.
    pub fn products(&self) -> ProductQueryBuilder {
        ProductQueryBuilder::new(self.clone())
    }

    /// Access GCP resource pricing with built-in defaults.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::gcp::DiskType;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .gcp()
    ///     .disk(DiskType::PdSsd)
    ///     .region("us-central1")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn gcp(&self) -> GcpProvider {
        GcpProvider::new(self.clone())
    }

    /// Access AWS resource pricing with built-in defaults.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::aws::EbsType;
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .aws()
    ///     .ebs(EbsType::Gp3)
    ///     .region("us-east-1")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn aws(&self) -> AwsProvider {
        AwsProvider::new(self.clone())
    }

    /// Access Azure resource pricing with built-in defaults.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use infracost_rs::Client;
    /// use infracost_rs::providers::azure::{ManagedDiskType, ManagedDiskSize};
    ///
    /// # async fn example() -> Result<(), infracost_rs::Error> {
    /// let client = Client::anonymous();
    /// let price = client
    ///     .azure()
    ///     .managed_disk(ManagedDiskType::PremiumSsd, ManagedDiskSize::P10)
    ///     .region("eastus")
    ///     .fetch_price()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn azure(&self) -> AzureProvider {
        AzureProvider::new(self.clone())
    }

    /// Execute a raw query with a filter.
    ///
    /// This is a convenience method that provides direct access without
    /// requiring the `PricingClient` trait to be in scope.
    pub async fn query_products(&self, filter: ProductFilter) -> Result<Vec<Product>> {
        self.execute_query(filter, None).await
    }

    /// Execute a raw query with a filter and optional API key override.
    ///
    /// This is a convenience method that provides direct access without
    /// requiring the `PricingClient` trait to be in scope.
    pub async fn query_products_with_key(
        &self,
        filter: ProductFilter,
        api_key: Option<&str>,
    ) -> Result<Vec<Product>> {
        self.execute_query(filter, api_key).await
    }

    /// Execute a GraphQL query against the Infracost API
    pub(crate) async fn execute_query(
        &self,
        filter: ProductFilter,
        api_key_override: Option<&str>,
    ) -> Result<Vec<Product>> {
        let api_key = api_key_override
            .or(self.inner.api_key.as_deref())
            .ok_or(Error::MissingApiKey)?;

        // Check cache first (only for authenticated requests)
        let cache_key = filter.cache_key();
        if let Some(cache) = &self.inner.cache {
            if let Some(cached) = cache.get(&cache_key).await {
                tracing::debug!(key = %cache_key, "cache hit");
                return Ok(cached);
            }
            tracing::debug!(key = %cache_key, "cache miss");
        }

        let gql_filter: GqlProductFilter = filter.into();
        let operation = ProductQuery::build(ProductQueryVariables {
            filter: Some(gql_filter),
        });

        // Serialize to JSON, removing null fields (Infracost API quirk)
        let mut operation_json =
            serde_json::to_value(&operation).map_err(|e| Error::config(e.to_string()))?;
        remove_nulls(&mut operation_json);

        tracing::debug!("Sending GraphQL query to Infracost API");
        if tracing::enabled!(tracing::Level::TRACE)
            && let Ok(json_str) = serde_json::to_string_pretty(&operation_json)
        {
            tracing::trace!("Query: {}", json_str);
        }

        let response = self
            .inner
            .http
            .post(&self.inner.endpoint)
            .header("X-Api-Key", api_key)
            .header(
                "User-Agent",
                concat!("infracost-rs/", env!("CARGO_PKG_VERSION")),
            )
            .header("Content-Type", "application/json")
            .json(&operation_json)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::api(
                status.as_u16(),
                if error_text.is_empty() {
                    status.to_string()
                } else {
                    error_text
                },
            ));
        }

        let response_text = response.text().await?;
        tracing::trace!(
            "Response: {}",
            &response_text[..response_text.len().min(1000)]
        );

        let gql_response: cynic::GraphQlResponse<ProductQuery> =
            serde_json::from_str(&response_text)?;

        if let Some(errors) = gql_response.errors {
            let error_msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(Error::graphql(error_msgs.join("; ")));
        }

        let data = gql_response
            .data
            .ok_or_else(|| Error::graphql("No data in response"))?;

        let products: Vec<Product> = data
            .products
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(Product::from)
            .collect();

        tracing::debug!("Query returned {} products", products.len());

        // Cache the result
        if let Some(cache) = &self.inner.cache {
            cache.set(&cache_key, &products, self.inner.cache_ttl).await;
        }

        Ok(products)
    }
}

#[async_trait]
impl PricingClient for Client {
    async fn query_products(&self, filter: ProductFilter) -> Result<Vec<Product>> {
        Client::query_products(self, filter).await
    }

    async fn query_products_with_key(
        &self,
        filter: ProductFilter,
        api_key: Option<&str>,
    ) -> Result<Vec<Product>> {
        Client::query_products_with_key(self, filter, api_key).await
    }
}

/// Builder for constructing a Client with custom configuration.
#[derive(Clone)]
pub struct ClientBuilder {
    api_key: Option<String>,
    endpoint: Option<String>,
    timeout: Duration,
    error_on_fallback: bool,
    cache: Option<Arc<dyn PriceCache>>,
    cache_ttl: Duration,
}

impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("endpoint", &self.endpoint)
            .field("timeout", &self.timeout)
            .field("error_on_fallback", &self.error_on_fallback)
            .field("has_cache", &self.cache.is_some())
            .field("cache_ttl", &self.cache_ttl)
            .finish()
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            timeout: DEFAULT_TIMEOUT,
            error_on_fallback: false,
            cache: None,
            cache_ttl: DEFAULT_CACHE_TTL,
        }
    }
}

impl ClientBuilder {
    /// Set the API key for the client.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set a custom API endpoint.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Error instead of returning default prices when API is unavailable.
    ///
    /// By default, the library gracefully falls back to built-in default prices
    /// when the API fails or no API key is provided. Enable this for strict mode
    /// where you want errors instead of potentially stale defaults.
    ///
    /// Note: You can also check `PriceResult.source` to see if a price came from
    /// the API or from defaults, which gives you more flexibility.
    pub fn error_on_fallback(mut self, enabled: bool) -> Self {
        self.error_on_fallback = enabled;
        self
    }

    /// Enable caching with a custom cache implementation.
    pub fn with_cache<C: PriceCache + 'static>(mut self, cache: C) -> Self {
        self.cache = Some(Arc::new(cache));
        self
    }

    /// Set cache TTL (default: 24 hours).
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<Client> {
        let http = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| Error::config(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                api_key: self.api_key,
                endpoint: self
                    .endpoint
                    .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
                error_on_fallback: self.error_on_fallback,
                cache: self.cache,
                cache_ttl: self.cache_ttl,
            }),
        })
    }
}

/// Remove null values from a JSON value recursively.
///
/// This is needed because the Infracost API returns empty results
/// when null fields are present in the request.
fn remove_nulls(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|_, v| !v.is_null());
            for v in map.values_mut() {
                remove_nulls(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                remove_nulls(v);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = Client::new("test-key");
        assert!(client.has_api_key());
        assert_eq!(client.endpoint(), DEFAULT_ENDPOINT);
    }

    #[test]
    fn test_client_anonymous() {
        let client = Client::anonymous();
        assert!(!client.has_api_key());
    }

    #[test]
    fn test_client_builder() {
        let client = Client::builder()
            .api_key("test-key")
            .endpoint("https://custom.endpoint/graphql")
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        assert!(client.has_api_key());
        assert_eq!(client.endpoint(), "https://custom.endpoint/graphql");
    }

    #[test]
    fn test_client_error_on_fallback_default() {
        let client = Client::new("test-key");
        assert!(!client.error_on_fallback());

        let client = Client::anonymous();
        assert!(!client.error_on_fallback());
    }

    #[test]
    fn test_client_builder_error_on_fallback() {
        let client = Client::builder()
            .api_key("test-key")
            .error_on_fallback(true)
            .build()
            .unwrap();

        assert!(client.error_on_fallback());
    }

    #[test]
    fn test_client_builder_error_on_fallback_false() {
        let client = Client::builder()
            .api_key("test-key")
            .error_on_fallback(false)
            .build()
            .unwrap();

        assert!(!client.error_on_fallback());
    }

    #[test]
    fn test_remove_nulls() {
        let mut value = serde_json::json!({
            "a": 1,
            "b": null,
            "c": {
                "d": 2,
                "e": null
            },
            "f": [1, null, 3]
        });
        remove_nulls(&mut value);

        // b and e should be removed, but null in array stays
        assert!(value.get("b").is_none());
        assert!(value["c"].get("e").is_none());
        assert_eq!(value["c"]["d"], 2);
    }

    #[test]
    fn test_client_builder_with_cache_ttl() {
        use std::time::Duration;

        let client = Client::builder()
            .api_key("test-key")
            .cache_ttl(Duration::from_secs(3600))
            .build()
            .unwrap();

        assert!(client.has_api_key());
    }
}
