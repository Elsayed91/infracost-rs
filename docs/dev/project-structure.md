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
    macros.rs                   resource_builder! macro - generates builders for simple/required/optional param cases
    aws/mod.rs                  AwsProvider + re-exports
    aws/{resource}.rs           per-resource async builders (macro-based or hand-written)
    gcp/mod.rs                  GcpProvider + re-exports
    gcp/{resource}.rs           per-resource async builders (macro-based or hand-written)
    azure/mod.rs                AzureProvider + re-exports

  blocking/
    mod.rs                      blocking::Client + blocking_builder! macro
    aws.rs                      blocking wrappers (blocking_builder! invocations + smoke test)
    gcp.rs                      blocking wrappers (blocking_builder! invocations + smoke test)
    azure.rs                    blocking wrappers (blocking_builder! invocations + smoke test)

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

## Builder Architecture

Two macros power most builders:

**`resource_builder!`** (`src/providers/macros.rs`) - generates async builders:
- 3 variants: simple (no params), required param, optional param
- Generates struct with `client: Client` (owned, no lifetimes), `region`, `api_key`, `override_default`
- Generates `new()`, `region()`, `api_key()`, `override_default()`, `fetch()`, `fetch_price()`, `fetch_monthly()`
- 10 of 14 builders use this macro

**`blocking_builder!`** (`src/blocking/mod.rs`) - generates blocking wrappers:
- Wraps async builder directly: `{ inner: AsyncBuilder, runtime: Arc<Runtime> }`
- Delegates all methods to the wrapped async builder
- Extra setters listed in the macro invocation
- All blocking builders use this macro (no hand-written blocking code)

**4 hand-written async builders** for complex cases:
- `aws/ebs.rs` - 3 params, tiered IOPS pricing, baseline-aware logic
- `gcp/disk.rs` - 4 params, conditional regional pricing, pd-extreme IOPS
- `gcp/backend_service.rs` - dual-query logic (data + forwarding rules)
- `azure/managed_disk.rs` - constructor requires type+size params, fixed SKU pricing

## Testing Strategy

- **Async builder unit tests** - detailed assertions (default prices, monthly calculations, edge cases)
- **Blocking smoke tests** - one test per vendor verifying all blocking wrappers compile and execute (no duplicated assertions)
- **Integration tests** - `tests/` directory, require API key, test across 7+ regions

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `ResourceDef` | `catalog/types.rs` | One resource definition from YAML |
| `CostComponentDef` | `catalog/types.rs` | One cost dimension (storage, IOPS, uptime) |
| `PricingModelDef` | `catalog/types.rs` | How to calculate: linear, tiered, hourly_to_monthly, etc. |
| `PricingEngine` | `catalog/engine.rs` | Does all the work: query, filter, calculate |
| `PriceResult` | `providers/mod.rs` | What you get back: price + unit + source |
| `ProductFilter` | `types.rs` | Raw API query builder (used by engine internally) |
| `Client` | `client.rs` | `Arc<ClientInner>` - cheap to clone, owned by builders |
