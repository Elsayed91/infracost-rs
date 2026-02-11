//! Declarative macros for generating resource builders.

/// Generate a resource builder with standard methods.
///
/// Supports zero, one required, or one optional parameter beyond the common fields.
///
/// # Examples
///
/// ```ignore
/// // Simple builder (no extra params)
/// resource_builder! {
///     /// Builder for querying AWS Elastic IP prices.
///     pub struct ElasticIpBuilder {
///         catalog: aws_catalog,
///         resource: "elastic-ip",
///         vendor: "aws",
///     }
/// }
///
/// // Builder with required parameter
/// resource_builder! {
///     /// Builder for querying AWS Snapshot prices.
///     pub struct SnapshotBuilder {
///         catalog: aws_catalog,
///         resource: "snapshot",
///         vendor: "aws",
///         required param: size_gb(u64) => "size_gb is required for fetch_monthly",
///     }
/// }
///
/// // Builder with optional parameter (defaults to 0)
/// resource_builder! {
///     /// Builder for querying AWS NAT Gateway prices.
///     pub struct NatGatewayBuilder {
///         catalog: aws_catalog,
///         resource: "nat-gateway",
///         vendor: "aws",
///         optional param: data_processed_gb(u64),
///     }
/// }
/// ```
macro_rules! resource_builder {
    // Case 1: Simple builder (no extra params)
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            catalog: $catalog:ident,
            resource: $resource:expr,
            vendor: $vendor:expr,
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            client: crate::Client,
            region: Option<String>,
            api_key: Option<String>,
            override_default: Option<f64>,
        }

        impl $name {
            pub(crate) fn new(client: crate::Client) -> Self {
                Self {
                    client,
                    region: None,
                    api_key: None,
                    override_default: None,
                }
            }

            pub fn region(mut self, region: impl Into<String>) -> Self {
                self.region = Some(region.into());
                self
            }

            pub fn api_key(mut self, key: impl Into<String>) -> Self {
                self.api_key = Some(key.into());
                self
            }

            pub fn override_default(mut self, price: f64) -> Self {
                self.override_default = Some(price);
                self
            }

            pub async fn fetch_price(self) -> crate::Result<f64> {
                self.fetch().await.map(|r| r.price)
            }

            pub async fn fetch(self) -> crate::Result<crate::providers::PriceResult> {
                let resource = crate::catalog::$catalog().find($resource)?;
                let region = self.region.as_deref().unwrap_or(&resource.default_region);
                crate::catalog::engine::PricingEngine::fetch(
                    &self.client,
                    resource,
                    $vendor,
                    region,
                    self.api_key.as_deref(),
                    self.override_default,
                )
                .await
            }

            pub async fn fetch_monthly(self) -> crate::Result<crate::providers::PriceResult> {
                let resource = crate::catalog::$catalog().find($resource)?;
                let region = self.region.as_deref().unwrap_or(&resource.default_region);

                let unit_result = crate::catalog::engine::PricingEngine::fetch(
                    &self.client,
                    resource,
                    $vendor,
                    region,
                    self.api_key.as_deref(),
                    self.override_default,
                )
                .await?;

                if unit_result.unit == "hour" {
                    Ok(crate::providers::PriceResult {
                        price: unit_result.price * 730.0,
                        unit: "month".to_string(),
                        source: unit_result.source,
                    })
                } else {
                    Ok(unit_result)
                }
            }
        }
    };

    // Case 2: Builder with required parameter
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            catalog: $catalog:ident,
            resource: $resource:expr,
            vendor: $vendor:expr,
            required param: $param_name:ident($param_type:ty) => $error_msg:expr,
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            client: crate::Client,
            region: Option<String>,
            api_key: Option<String>,
            override_default: Option<f64>,
            $param_name: Option<$param_type>,
        }

        impl $name {
            pub(crate) fn new(client: crate::Client) -> Self {
                Self {
                    client,
                    region: None,
                    api_key: None,
                    override_default: None,
                    $param_name: None,
                }
            }

            pub fn region(mut self, region: impl Into<String>) -> Self {
                self.region = Some(region.into());
                self
            }

            pub fn api_key(mut self, key: impl Into<String>) -> Self {
                self.api_key = Some(key.into());
                self
            }

            pub fn override_default(mut self, price: f64) -> Self {
                self.override_default = Some(price);
                self
            }

            pub fn $param_name(mut self, value: $param_type) -> Self {
                self.$param_name = Some(value);
                self
            }

            pub async fn fetch_price(self) -> crate::Result<f64> {
                self.fetch().await.map(|r| r.price)
            }

            pub async fn fetch(self) -> crate::Result<crate::providers::PriceResult> {
                let resource = crate::catalog::$catalog().find($resource)?;
                let region = self.region.as_deref().unwrap_or(&resource.default_region);
                crate::catalog::engine::PricingEngine::fetch(
                    &self.client,
                    resource,
                    $vendor,
                    region,
                    self.api_key.as_deref(),
                    self.override_default,
                )
                .await
            }

            pub async fn fetch_monthly(self) -> crate::Result<crate::providers::PriceResult> {
                let param_value = self
                    .$param_name
                    .ok_or_else(|| crate::Error::validation($error_msg))?;
                let resource = crate::catalog::$catalog().find($resource)?;
                let region = self.region.as_deref().unwrap_or(&resource.default_region);

                let unit_result = crate::catalog::engine::PricingEngine::fetch(
                    &self.client,
                    resource,
                    $vendor,
                    region,
                    self.api_key.as_deref(),
                    self.override_default,
                )
                .await?;

                let monthly_price = unit_result.price * param_value as f64;

                Ok(crate::providers::PriceResult {
                    price: monthly_price,
                    unit: "month".to_string(),
                    source: unit_result.source,
                })
            }
        }
    };

    // Case 3: Builder with optional parameter (defaults to 0)
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            catalog: $catalog:ident,
            resource: $resource:expr,
            vendor: $vendor:expr,
            optional param: $param_name:ident($param_type:ty),
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            client: crate::Client,
            region: Option<String>,
            api_key: Option<String>,
            override_default: Option<f64>,
            $param_name: Option<$param_type>,
        }

        impl $name {
            pub(crate) fn new(client: crate::Client) -> Self {
                Self {
                    client,
                    region: None,
                    api_key: None,
                    override_default: None,
                    $param_name: None,
                }
            }

            pub fn region(mut self, region: impl Into<String>) -> Self {
                self.region = Some(region.into());
                self
            }

            pub fn api_key(mut self, key: impl Into<String>) -> Self {
                self.api_key = Some(key.into());
                self
            }

            pub fn override_default(mut self, price: f64) -> Self {
                self.override_default = Some(price);
                self
            }

            pub fn $param_name(mut self, value: $param_type) -> Self {
                self.$param_name = Some(value);
                self
            }

            pub async fn fetch_price(self) -> crate::Result<f64> {
                self.fetch().await.map(|r| r.price)
            }

            pub async fn fetch(self) -> crate::Result<crate::providers::PriceResult> {
                let resource = crate::catalog::$catalog().find($resource)?;
                let region = self.region.as_deref().unwrap_or(&resource.default_region);
                crate::catalog::engine::PricingEngine::fetch(
                    &self.client,
                    resource,
                    $vendor,
                    region,
                    self.api_key.as_deref(),
                    self.override_default,
                )
                .await
            }

            pub async fn fetch_monthly(self) -> crate::Result<crate::providers::PriceResult> {
                let resource = crate::catalog::$catalog().find($resource)?;
                let region = self.region.as_deref().unwrap_or(&resource.default_region);
                let mut params = std::collections::HashMap::new();
                let param_value = self.$param_name.unwrap_or(0);
                params.insert(stringify!($param_name).to_string(), param_value);
                crate::catalog::engine::PricingEngine::fetch_monthly(
                    &self.client,
                    resource,
                    $vendor,
                    region,
                    self.api_key.as_deref(),
                    &params,
                )
                .await
            }
        }
    };
}

pub(crate) use resource_builder;
