# Adding a New Resource

## Files You Touch

```
resources/{vendor}/{resource}.yaml     <- pricing definition
src/providers/{vendor}/{resource}.rs   <- async builder (use resource_builder! macro)
src/providers/{vendor}/mod.rs          <- wire it in
src/blocking/{vendor}.rs               <- blocking wrapper (use blocking_builder! macro)
src/catalog/mod.rs                     <- register yaml
tests/{vendor}_{resource}_regional.rs  <- integration tests
```

## The Order

1. **Research** - use `irs` CLI to find query attributes that work across 7+ regions
2. **YAML** - define the resource pricing
3. **Catalog** - register the YAML in `src/catalog/mod.rs`
4. **Builder** - use `resource_builder!` macro (or hand-write for complex cases)
5. **Wire** - add to `mod.rs` + blocking wrapper via `blocking_builder!`
6. **Test** - unit tests in builder, integration tests in `tests/`

## 1. Research

```bash
cargo build --features cli --bin irs

# explore
irs query -v gcp -r us-central1 --service "Compute Engine" --limit 20

# narrow down
irs query -v gcp -r us-central1 -a 'resourceGroup=SSD' --limit 5 -f json

# validate across regions (CRITICAL)
for r in us-central1 europe-west1 asia-southeast1; do
  irs query -v gcp -r "$r" -a 'resourceGroup=SSD' --limit 1 -f json
done
```

Avoid AWS `usagetype` - has regional prefixes. Use `group`, `productFamily`, `volumeApiName` instead.

## 2. YAML

Simple (hourly):
```yaml
- name: static-ip
  default_region: us-central1
  cost_components:
    - name: uptime
      is_primary: true
      unit: hour
      default_price: 0.01
      query:
        service: "Compute Engine"
        product_family: Network
        attributes:
          - { key: resourceGroup, value: IpAddress }
      pricing_model:
        type: hourly_to_monthly
```

With params (storage * size):
```yaml
- name: snapshot
  parameters:
    - { name: size_gb, required_for_monthly: true }
  cost_components:
    - name: storage
      is_primary: true
      unit: GB-month
      default_price: 0.05
      query: { ... }
      pricing_model:
        type: linear
        quantity_param: size_gb
```

Multi-component (uptime + data):
```yaml
- name: nat-gateway
  parameters:
    - { name: data_processed_gb, required_for_monthly: true }
  cost_components:
    - name: uptime
      is_primary: true
      unit: hour
      default_price: 0.0014
      query: { ... }
      pricing_model:
        type: hourly_to_monthly
    - name: data_processing
      is_primary: false
      unit: GiB
      default_price: 0.045
      query: { ... }
      pricing_model:
        type: linear
        quantity_param: data_processed_gb
```

Pricing models: `hourly_to_monthly`, `linear`, `linear_with_baseline`, `tiered`, `fixed`

## 3. Register in Catalog

`src/catalog/mod.rs` - add `include_str!` + test assertion:
```rust
include_str!("../../resources/gcp/your-resource.yaml"),
```

## 4. Builder

Most builders use `resource_builder!` macro from `src/providers/macros.rs`. Three variants:

**Simple (no params):**
```rust
use crate::providers::macros::resource_builder;

resource_builder! {
    /// Builder for querying GCP static IP prices.
    pub struct StaticIpBuilder {
        catalog: gcp_catalog,
        resource: "static-ip",
        vendor: "gcp",
    }
}
```

**Required param (must set for fetch_monthly):**
```rust
resource_builder! {
    /// Builder for querying AWS Snapshot prices.
    pub struct SnapshotBuilder {
        catalog: aws_catalog,
        resource: "snapshot",
        vendor: "aws",
        required param: size_gb(u64) => "size_gb is required for fetch_monthly",
    }
}
```

**Optional param (defaults to 0):**
```rust
resource_builder! {
    /// Builder for querying AWS NAT Gateway prices.
    pub struct NatGatewayBuilder {
        catalog: aws_catalog,
        resource: "nat-gateway",
        vendor: "aws",
        optional param: data_processed_gb(u64),
    }
}
```

The macro generates: `new(client: Client)`, `region()`, `api_key()`, `override_default()`, `fetch()`, `fetch_price()`, `fetch_monthly()`.

Builders own a `Client` (no lifetimes). `Client` is `Arc<ClientInner>` so cloning is O(1).

**Hand-write** for complex cases (multi-param, tiered, conditional logic). See `disk.rs`, `ebs.rs`, `backend_service.rs`, `managed_disk.rs`.

Add unit tests below the macro/struct:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;

    #[tokio::test]
    async fn test_{resource}_returns_default_without_api_key() {
        let client = Client::anonymous();
        let result = client.{vendor}().{resource}()
            .region("{default_region}")
            .fetch().await.unwrap();
        assert!(result.is_from_default());
        assert_eq!(result.price, {DEFAULT_PRICE});
        assert_eq!(result.unit, "{UNIT}");
    }
}
```

## 5. Wire

`src/providers/{vendor}/mod.rs`:
```rust
mod your_resource;
pub use your_resource::YourResourceBuilder;

// add method to provider impl block - takes owned self, no lifetimes
pub fn your_resource(self) -> YourResourceBuilder {
    YourResourceBuilder::new(self.client)
}
```

`src/blocking/{vendor}.rs` - use `blocking_builder!` macro:
```rust
blocking_builder! {
    /// Blocking builder for querying {Vendor} {Resource} prices.
    pub struct Blocking{Vendor}{Resource}Builder wraps crate::providers::{vendor}::{Resource}Builder {
        // list extra setter methods here (beyond region/api_key/override_default):
        fn size_gb(u64);
    }
}
```

The macro wraps the async builder directly: `{ inner: AsyncBuilder, runtime: Arc<Runtime> }`. It generates `region()`, `api_key()`, `override_default()`, `fetch()`, `fetch_price()`, `fetch_monthly()` plus any listed extra setters.

Add the provider method:
```rust
pub fn your_resource(self) -> Blocking{Vendor}{Resource}Builder {
    Blocking{Vendor}{Resource}Builder {
        inner: self.client.{vendor}().your_resource(),
        runtime: self.runtime,
    }
}
```

## 6. Test

```bash
cargo test                        # unit tests
cargo clippy --all-features       # 0 warnings
INFRACOST_API_KEY=xxx cargo test --test your_test -- --ignored  # integration
```

## Checklist

- [ ] IRS query works across 7+ regions
- [ ] YAML parses (`cargo check`)
- [ ] Catalog test passes
- [ ] Unit tests pass without API key
- [ ] Integration tests pass with API key
- [ ] Blocking wrapper exists (via `blocking_builder!`)
- [ ] `cargo clippy --all-features` clean

AI-automated workflow: `/add-resource` skill (`.claude/skills/add-resource/SKILL.md`)
