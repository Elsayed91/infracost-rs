# Project Structure

## Directory Layout

```
resources/                      pricing YAML definitions
  aws/                            ebs.yaml, snapshot.yaml, elastic-ip.yaml, nat-gateway.yaml, alb.yaml
  gcp/                            disk.yaml, snapshot.yaml, static-ip.yaml, nat-gateway.yaml, forwarding-rule.yaml
  azure/                          managed-disk/*.yaml, snapshot.yaml, public-ip.yaml

src/
  lib.rs                        crate root, re-exports Client + ProductFilter
  client.rs                     HTTP client (async), API key management
  types.rs                      ProductFilter builder, Product/Price structs
  error.rs                      Error types

  catalog/
    types.rs                    YAML schema: ResourceDef, CostComponentDef, PricingModelDef
    engine.rs                   PricingEngine - builds filters, queries API, applies post-filters, calculates cost
    mod.rs                      static catalogs (AWS_CATALOG, GCP_CATALOG, AZURE_CATALOG)

  providers/
    mod.rs                      PriceResult, PriceSource
    aws/mod.rs                  AwsProvider + re-exports
    aws/{resource}.rs           per-resource async builders
    aws/from_json.rs            parse AWS CLI JSON into builders
    gcp/mod.rs                  GcpProvider + re-exports
    gcp/{resource}.rs           per-resource async builders
    gcp/from_json.rs            parse gcloud JSON into builders
    azure/mod.rs                AzureProvider + re-exports
    json_utils.rs               shared helpers: parse_u64, zone_to_region, etc.

  blocking/
    mod.rs                      blocking::Client
    aws.rs                      sync mirrors of async AWS builders
    gcp.rs                      sync mirrors of async GCP builders
    azure.rs                    sync mirrors of async Azure builders

tests/                          integration tests (require API key, #[ignore])
examples/                       runnable usage examples per vendor/resource
```

## How Pricing Works

```
YAML definition
    |
    v
PricingEngine::fetch() / fetch_monthly()
    |
    |--> build_filter() from YAML query attributes
    |--> client.query_products_with_key() -> API call
    |--> apply_post_filter() on results (description/usagetype matching)
    |--> apply price_filter (Azure: Consumption vs Reservation)
    |--> apply price_transform (divide_by, multiply_by)
    |--> calculate_component_cost() using pricing_model
    |
    v
PriceResult { price, unit, source }
```

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `ResourceDef` | `catalog/types.rs` | One resource definition from YAML |
| `CostComponentDef` | `catalog/types.rs` | One cost dimension (storage, IOPS, uptime) |
| `PricingModelDef` | `catalog/types.rs` | How to calculate: linear, tiered, hourly_to_monthly, etc. |
| `PricingEngine` | `catalog/engine.rs` | Does all the work: query, filter, calculate |
| `PriceResult` | `providers/mod.rs` | What you get back: price + unit + source |
| `ProductFilter` | `types.rs` | Raw API query builder (used by engine internally) |
