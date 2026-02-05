# Quality Review Guide for Pricing Accuracy

## 1. Overview

### Purpose
Quality reviews ensure that the default pricing values in `infracost-rs` accurately reflect real cloud provider pricing. This prevents cost estimation errors that could lead to budget surprises for users.

### When to Perform Reviews
- **After adding new resources**: Verify all defaults before merging
- **Periodically**: Cloud providers update pricing; quarterly reviews recommended
- **Before releases**: Full verification of all resource pricing
- **When users report discrepancies**: Investigate and correct as needed

---

## 2. Tools Required

### 2.1 IRS CLI Tool
The `irs` binary queries the Infracost Cloud Pricing API directly.

```bash
# Build the CLI tool
cargo build --features cli --bin irs
```

### 2.2 API Key

An Infracost API key is required. **If you don't have one:**
- Ask the user/maintainer for the test API key
- Or obtain one from https://www.infracost.io/

```bash
export INFRACOST_API_KEY="<your-api-key>"
```

### 2.3 Web Browser
For verifying against official cloud pricing documentation:
- AWS: https://aws.amazon.com/pricing/
- Azure: https://azure.microsoft.com/en-us/pricing/
- GCP: https://cloud.google.com/pricing

---

## 3. Verification Process

### 3.1 Query Infracost API

#### AWS Examples

```bash
# EBS gp3 Storage (per GB-month)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'usagetype=EBS:VolumeUsage.gp3' \
  --limit 5 -f json

# NAT Gateway (per hour)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'usagetype=NatGateway-Hours' \
  --limit 5 -f json

# NAT Gateway Data Processing (per GB)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'usagetype=NatGateway-Bytes' \
  --limit 5 -f json

# ALB (per hour)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'usagetype=LoadBalancerUsage' \
  -a 'productFamily=Load Balancer-Application' \
  --limit 5 -f json

# Elastic IP (per hour, in use)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'usagetype=ElasticIP:IdleAddress' \
  --limit 5 -f json

# EBS Snapshot (per GB-month)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v aws -r us-east-1 \
  -a 'usagetype=EBS:SnapshotUsage' \
  --limit 5 -f json
```

#### Azure Examples

```bash
# Managed Disk - Premium SSD P10 (per month)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v azure -r eastus \
  -a 'productName=Premium SSD Managed Disks' \
  -a 'skuName=P10 LRS' \
  -a 'meterName=P10 LRS Disk' \
  --limit 5 -f json

# Standard SSD E10
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v azure -r eastus \
  -a 'productName=Standard SSD Managed Disks' \
  -a 'skuName=E10 LRS' \
  --limit 5 -f json

# Public IP - Standard Static (per hour)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v azure -r eastus \
  -a 'productName=IP Addresses' \
  -a 'skuName=Standard' \
  -a 'meterName=Standard Static Public IP' \
  --limit 5 -f json

# Snapshot - Standard HDD (per GB-month)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v azure -r eastus \
  -a 'productName=Managed Disks Snapshots' \
  -a 'skuName=Standard HDD' \
  --limit 5 -f json
```

#### GCP Examples

```bash
# Persistent Disk SSD (per GB-month)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v gcp -r us-central1 \
  -a 'description=SSD backed PD Capacity' \
  --limit 5 -f json

# NAT Gateway (per hour per VM)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v gcp -r us-central1 \
  -a 'description=NAT Gateway: Uptime charge' \
  --limit 5 -f json

# Forwarding Rule (per hour)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v gcp -r us-central1 \
  -a 'description=Forwarding Rule Minimum Service Charge' \
  --limit 5 -f json

# Static IP - In Use (per hour)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v gcp -r us-central1 \
  -a 'description=Static Ip Charge' \
  --limit 5 -f json

# Disk Snapshot (per GB-month)
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v gcp -r us-central1 \
  -a 'description=Storage PD Snapshot' \
  --limit 5 -f json
```

### 3.2 Web Search Official Pricing

Use these search patterns to find official pricing documentation:

```
# AWS
site:aws.amazon.com pricing EBS
site:aws.amazon.com pricing NAT Gateway
site:aws.amazon.com pricing elastic IP

# Azure
site:azure.microsoft.com pricing managed disks
site:azure.microsoft.com pricing IP addresses
site:azure.microsoft.com pricing bandwidth

# GCP
site:cloud.google.com pricing compute disks
site:cloud.google.com pricing cloud nat
site:cloud.google.com pricing network
```

**Key pricing pages:**
- AWS EBS: https://aws.amazon.com/ebs/pricing/
- AWS VPC (NAT, IP): https://aws.amazon.com/vpc/pricing/
- Azure Disks: https://azure.microsoft.com/en-us/pricing/details/managed-disks/
- Azure IP: https://azure.microsoft.com/en-us/pricing/details/ip-addresses/
- GCP Disks: https://cloud.google.com/compute/disks-image-pricing
- GCP Network: https://cloud.google.com/vpc/network-pricing

### 3.3 Compare Values

Create a comparison table for each resource:

| Component | Our Default | API Price | Official Price | Status |
|-----------|-------------|-----------|----------------|--------|
| AWS EBS gp3 ($/GB-mo) | 0.08 | 0.08 | $0.08 | OK |
| AWS NAT Gateway ($/hr) | 0.045 | 0.045 | $0.045 | OK |
| Azure P10 Disk ($/mo) | 19.71 | 19.71 | $19.71 | OK |
| GCP SSD PD ($/GB-mo) | 0.17 | 0.17 | $0.170 | OK |

**Status values:**
- **OK**: All three sources match
- **MISMATCH**: Values differ (investigate)
- **API_ONLY**: Cannot find official doc confirmation
- **REVIEW**: Minor difference, may be rounding

### 3.4 Verify Filter Uniqueness

Each filter combination should return exactly ONE product:

```bash
# Good - returns 1 product
INFRACOST_API_KEY="ico-..." cargo run --features cli --bin irs -- query \
  -v azure -r eastus \
  -a 'productName=Premium SSD Managed Disks' \
  -a 'skuName=P10 LRS' \
  -a 'meterName=P10 LRS Disk' \
  --limit 5 -f json
# Result: 1 product

# Bad - returns multiple products (ambiguous)
INFRACOST_API_KEY="ico-..." cargo run --features cli --bin irs -- query \
  -v azure -r eastus \
  -a 'productName=Premium SSD Managed Disks' \
  --limit 5 -f json
# Result: Multiple products (P1, P2, P10, etc.)
```

**If multiple products are returned:**
1. Add more specific attributes (skuName, meterName, usagetype)
2. Update the filter in the source code
3. Document which product variant we're targeting

---

## 4. Issue Severity Classification

### Critical (>5% price difference)
- Immediate fix required before release
- Could significantly impact user cost estimates
- Examples:
  - Using wrong unit (per hour vs per month)
  - Missing a major cost component
  - Order of magnitude error

### Warning (1-5% price difference)
- Should be fixed, but not blocking
- Track in issue for next release
- Examples:
  - Slightly outdated pricing
  - Minor regional variation
  - Rounding in intermediate calculations

### Info (<1% difference)
- Document but may not need fixing
- Examples:
  - Rounding to fewer decimal places
  - Tiered pricing not implemented (using first tier)
  - Minor regional variations

---

## 5. What We Don't Handle (By Design)

### Account-wide Free Tiers
These are account-level benefits, not per-resource pricing:

| Provider | Free Tier | Our Approach |
|----------|-----------|--------------|
| GCP | First static IP free when attached | Charge for all IPs |
| AWS | Free tier (12 months) | Ignore, use standard pricing |
| Azure | Free tier services | Ignore, use pay-as-you-go |

**Rationale**: Users estimating costs should see worst-case pricing. Free tiers vary by account age, region, and prior usage.

### Reserved/Committed Pricing
We use on-demand/consumption pricing only:

| Pricing Type | Our Support |
|--------------|-------------|
| On-demand (AWS) | YES |
| Pay-as-you-go (Azure) | YES |
| Consumption (GCP) | YES |
| Reserved Instances | NO |
| Savings Plans | NO |
| Committed Use Discounts | NO |

**Rationale**: Committed pricing requires knowledge of existing commitments, which varies by organization.

### Regional Variations
Default prices are based on reference regions:

| Provider | Reference Region |
|----------|------------------|
| AWS | us-east-1 |
| Azure | eastus |
| GCP | us-central1 |

**Rationale**: These are the most commonly used regions and typically have the lowest/reference pricing. Users can adjust for their target region.

---

## 6. Common Issues and Solutions

### Filter Returns 0 Products

**Symptom:**
```json
{
  "products": [],
  "total_count": 0
}
```

**Solutions:**
1. Check spelling of description/attribute values
2. Remove filters one by one to find the problematic one
3. Query with fewer filters to see what's available
4. Check if the service name changed (Azure especially)

```bash
# Debug: See what products exist for a service
INFRACOST_API_KEY="ico-..." cargo run --features cli --bin irs -- query \
  -v azure -r eastus \
  -a 'serviceName=Storage' \
  --limit 20 -f json
```

### Filter Returns Multiple Products

**Symptom:**
```json
{
  "products": [...multiple items...],
  "total_count": 5
}
```

**Solutions:**
1. Add more specific attributes:
   - AWS: Add `usagetype`, `productFamily`
   - Azure: Add `skuName`, `meterName`
   - GCP: Add more specific `description`
2. Check if we need to handle multiple tiers
3. Document which variant we're using

### Price Mismatch

**Symptom:** Our default differs from API or official docs.

**Solutions:**
1. Verify using all three sources (code, API, official docs)
2. Check if pricing recently changed
3. Check regional differences
4. Update the default value in source code
5. Add a code comment with verification date

```rust
// Verified 2024-01-15: $0.08/GB-mo per AWS pricing page
const DEFAULT_GP3_PRICE: f64 = 0.08;
```

### Tiered Pricing

**Symptom:** API returns multiple prices for the same product (graduated tiers).

**Approach:**
1. For ongoing/steady-state costs: Use the first non-zero tier price
2. Document that tiered pricing exists but isn't implemented
3. Consider implementing tier support if the difference is significant

```rust
// Note: S3 has tiered pricing. We use first 50TB tier.
// First 50 TB: $0.023/GB
// Next 450 TB: $0.022/GB
// Over 500 TB: $0.021/GB
const DEFAULT_S3_STORAGE_PRICE: f64 = 0.023;
```

---

## 7. Reporting Template

Use this template to document quality review findings:

```markdown
# Quality Review Report

**Date:** YYYY-MM-DD
**Reviewer:** Name
**Scope:** [All resources | AWS only | Specific resources]

## Summary
- Total resources reviewed: X
- Issues found: Y
- Critical: N
- Warnings: M
- Info: K

## Findings

### Critical Issues

#### [Resource Name]
- **Current value:** $X.XX
- **API value:** $Y.YY
- **Official docs:** $Z.ZZ
- **Difference:** X%
- **Recommended fix:** Update default to $Y.YY
- **File:** `src/providers/aws/resources/xxx.rs:123`

### Warnings

#### [Resource Name]
- **Issue:** [Description]
- **Impact:** [Low/Medium]
- **Recommendation:** [Action]

### Info

- [Minor finding 1]
- [Minor finding 2]

## Verification Commands Used

```bash
# Command 1
...

# Command 2
...
```

## Next Steps
1. [ ] Fix critical issues
2. [ ] Create issues for warnings
3. [ ] Schedule next review
```

---

## 8. Resources Checklist

### AWS Resources

| Resource | File | Key Defaults | Last Verified |
|----------|------|--------------|---------------|
| EBS Volume | `src/providers/aws/resources/ebs_volume.rs` | gp3: $0.08/GB-mo | |
| | | gp2: $0.10/GB-mo | |
| | | io1: $0.125/GB-mo | |
| | | st1: $0.045/GB-mo | |
| | | sc1: $0.015/GB-mo | |
| NAT Gateway | `src/providers/aws/resources/nat_gateway.rs` | $0.045/hr | |
| | | $0.045/GB data | |
| ALB | `src/providers/aws/resources/alb.rs` | $0.0225/hr | |
| | | $0.008/LCU-hr | |
| Elastic IP | `src/providers/aws/resources/elastic_ip.rs` | $0.005/hr (in use) | |
| EBS Snapshot | `src/providers/aws/resources/ebs_snapshot.rs` | $0.05/GB-mo | |

### Azure Resources

| Resource | File | Key Defaults | Last Verified |
|----------|------|--------------|---------------|
| Managed Disk | `src/providers/azure/resources/managed_disk.rs` | P10: $19.71/mo | |
| | | E10: $9.60/mo | |
| | | S10: $5.89/mo | |
| Public IP | `src/providers/azure/resources/public_ip.rs` | Standard: $0.005/hr | |
| Snapshot | `src/providers/azure/resources/snapshot.rs` | $0.05/GB-mo | |

### GCP Resources

| Resource | File | Key Defaults | Last Verified |
|----------|------|--------------|---------------|
| Disk | `src/providers/gcp/resources/disk.rs` | pd-ssd: $0.17/GB-mo | |
| | | pd-standard: $0.04/GB-mo | |
| | | pd-balanced: $0.10/GB-mo | |
| NAT Gateway | `src/providers/gcp/resources/nat_gateway.rs` | $0.044/hr/VM | |
| | | $0.045/GB data | |
| Forwarding Rule | `src/providers/gcp/resources/forwarding_rule.rs` | $0.025/hr | |
| Static IP | `src/providers/gcp/resources/static_ip.rs` | $0.004/hr (attached) | |
| | | $0.01/hr (unused) | |
| Snapshot | `src/providers/gcp/resources/snapshot.rs` | $0.026/GB-mo | |

---

## Quick Reference

### API Query Template

```bash
INFRACOST_API_KEY="$INFRACOST_API_KEY" \
cargo run --features cli --bin irs -- query \
  -v <aws|azure|gcp> \
  -r <region> \
  -a '<attribute>=<value>' \
  --limit 5 \
  -f json
```

### Common Attributes by Provider

**AWS:**
- `usagetype` - Most specific identifier
- `productFamily` - Service category
- `operation` - Specific operation type

**Azure:**
- `productName` - Service name
- `skuName` - Size/tier identifier
- `meterName` - Billing meter name

**GCP:**
- `description` - Human-readable description (primary filter)
- `resourceGroup` - Resource category

### Price Verification Checklist

- [ ] Query Infracost API
- [ ] Find official pricing page
- [ ] Compare all three values (code, API, official)
- [ ] Verify filter returns exactly 1 product
- [ ] Check unit consistency (hour vs month vs GB)
- [ ] Document findings in review report
