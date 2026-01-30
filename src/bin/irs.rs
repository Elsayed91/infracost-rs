//! CLI tool for querying cloud pricing from the command line.
//!
//! # Usage
//!
//! ```bash
//! # Query GCP disk pricing
//! irs query --vendor gcp --service "Compute Engine" --region us-central1
//!
//! # List available services
//! irs services --vendor gcp
//!
//! # List regions for a service
//! irs regions --vendor gcp --service "Compute Engine"
//! ```

use clap::{Parser, Subcommand, ValueEnum};
use infracost::{Client, ProductFilter};

#[derive(Parser)]
#[command(name = "irs")]
#[command(about = "Infracost-rs: Query cloud pricing from the command line")]
#[command(version)]
struct Cli {
    /// API key (or set INFRACOST_API_KEY environment variable)
    #[arg(long, global = true, env = "INFRACOST_API_KEY")]
    api_key: Option<String>,

    /// Output format
    #[arg(long, short, global = true, default_value = "table")]
    format: OutputFormat,

    /// Minimal output (just prices)
    #[arg(long, short, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query products with filters
    Query {
        /// Vendor name (gcp, aws, azure)
        #[arg(long, short)]
        vendor: String,

        /// Service name
        #[arg(long, short)]
        service: Option<String>,

        /// Product family
        #[arg(long)]
        product_family: Option<String>,

        /// Region
        #[arg(long, short)]
        region: Option<String>,

        /// SKU
        #[arg(long)]
        sku: Option<String>,

        /// Attribute filters (key=value or key~=regex)
        #[arg(long, short)]
        attribute: Vec<String>,

        /// Filter by purchase option (on_demand, preemptible, spot, reserved)
        #[arg(long, short = 'p')]
        purchase_option: Option<String>,

        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// List available services for a vendor
    Services {
        /// Vendor name (gcp, aws, azure)
        #[arg(long, short)]
        vendor: String,
    },

    /// List available regions for a vendor/service
    Regions {
        /// Vendor name (gcp, aws, azure)
        #[arg(long, short)]
        vendor: String,

        /// Service name
        #[arg(long, short)]
        service: Option<String>,
    },

    /// Validate a filter and show matched products
    Validate {
        /// Vendor name (gcp, aws, azure)
        #[arg(long, short)]
        vendor: String,

        /// Service name
        #[arg(long, short)]
        service: Option<String>,

        /// Product family
        #[arg(long)]
        product_family: Option<String>,

        /// Region
        #[arg(long, short)]
        region: Option<String>,

        /// SKU
        #[arg(long)]
        sku: Option<String>,

        /// Attribute filters (key=value or key~=regex)
        #[arg(long, short)]
        attribute: Vec<String>,
    },
}

#[derive(ValueEnum, Clone, Copy, Default)]
enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Create client
    let client = match &cli.api_key {
        Some(key) => Client::new(key),
        None => Client::from_env().map_err(|_| {
            anyhow::anyhow!(
                "API key not provided. Use --api-key or set INFRACOST_API_KEY environment variable"
            )
        })?,
    };

    match cli.command {
        Commands::Query {
            vendor,
            service,
            product_family,
            region,
            sku,
            attribute,
            purchase_option,
            limit,
        } => {
            let filter = build_filter(vendor, service, product_family, region, sku, attribute)?;
            let products = client.query_products(filter).await?;
            let products: Vec<_> = products.into_iter().take(limit).collect();

            if cli.quiet {
                // Just output prices
                for product in &products {
                    let price = match &purchase_option {
                        Some(po) => product.prices().purchase_option(po).iter().next(),
                        None => product.prices.first(),
                    };
                    if let Some(p) = price {
                        println!("{}", p.usd);
                    }
                }
            } else {
                match cli.format {
                    OutputFormat::Table => print_products_table(&products, purchase_option.as_deref()),
                    OutputFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&products)?);
                    }
                    OutputFormat::Csv => print_products_csv(&products, purchase_option.as_deref()),
                }
            }
        }

        Commands::Services { vendor } => {
            // Query all products for the vendor to get unique services
            let products = client
                .products()
                .vendor(&vendor)
                .fetch()
                .await?;

            let mut services: Vec<_> = products
                .iter()
                .map(|p| p.service.as_str())
                .collect();
            services.sort();
            services.dedup();

            match cli.format {
                OutputFormat::Table | OutputFormat::Csv => {
                    for service in services {
                        println!("{}", service);
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&services)?);
                }
            }
        }

        Commands::Regions { vendor, service } => {
            let mut builder = client.products().vendor(&vendor);
            if let Some(ref s) = service {
                builder = builder.service(s);
            }
            let products = builder.fetch().await?;

            let mut regions: Vec<_> = products
                .iter()
                .filter_map(|p| p.region.as_deref())
                .collect();
            regions.sort();
            regions.dedup();

            match cli.format {
                OutputFormat::Table | OutputFormat::Csv => {
                    for region in regions {
                        println!("{}", region);
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&regions)?);
                }
            }
        }

        Commands::Validate {
            vendor,
            service,
            product_family,
            region,
            sku,
            attribute,
        } => {
            let filter = build_filter(vendor, service, product_family, region, sku, attribute)?;
            let products = client.query_products(filter).await?;

            if products.is_empty() {
                eprintln!("No products match the filter");
                std::process::exit(1);
            }

            println!("Filter matches {} products\n", products.len());
            println!("Samples:");

            for product in products.iter().take(5) {
                let region = product.region.as_deref().unwrap_or("-");
                let price = product
                    .prices
                    .first()
                    .map(|p| format!("${}/{}", p.usd, p.unit))
                    .unwrap_or_else(|| "-".to_string());
                println!("  {:<15} {:<40} {}", region, product.sku, price);
            }
        }
    }

    Ok(())
}

fn build_filter(
    vendor: String,
    service: Option<String>,
    product_family: Option<String>,
    region: Option<String>,
    sku: Option<String>,
    attributes: Vec<String>,
) -> anyhow::Result<ProductFilter> {
    let mut builder = ProductFilter::builder().vendor(vendor);

    if let Some(s) = service {
        builder = builder.service(s);
    }
    if let Some(pf) = product_family {
        builder = builder.product_family(pf);
    }
    if let Some(r) = region {
        builder = builder.region(r);
    }
    if let Some(s) = sku {
        builder = builder.sku(s);
    }

    for attr in attributes {
        if let Some((key, value)) = attr.split_once("~=") {
            // Regex match
            builder = builder.attribute_regex(key.trim(), value.trim());
        } else if let Some((key, value)) = attr.split_once('=') {
            // Exact match
            builder = builder.attribute(key.trim(), value.trim());
        } else {
            anyhow::bail!("Invalid attribute filter format: {}. Use key=value or key~=regex", attr);
        }
    }

    Ok(builder.build())
}

fn print_products_table(products: &[infracost::Product], purchase_option: Option<&str>) {
    if products.is_empty() {
        println!("No products found");
        return;
    }

    // Header
    println!(
        "{:<60} {:<12} {:<15}",
        "DESCRIPTION", "PRICE", "UNIT"
    );
    println!("{}", "-".repeat(87));

    for product in products {
        let desc = product
            .attribute("description")
            .unwrap_or(&product.sku);
        // Truncate long descriptions
        let desc_display: String = if desc.len() > 57 {
            format!("{}...", &desc[..57])
        } else {
            desc.to_string()
        };

        let price_entry = match purchase_option {
            Some(po) => product.prices().purchase_option(po).iter().next(),
            None => product.prices.first(),
        };

        let (price, unit) = price_entry
            .map(|p| (format!("${}", p.usd), p.unit.as_str()))
            .unwrap_or(("-".to_string(), "-"));

        println!("{:<60} {:<12} {:<15}", desc_display, price, unit);
    }
}

fn print_products_csv(products: &[infracost::Product], purchase_option: Option<&str>) {
    println!("vendor,service,region,sku,description,product_family,price,unit");
    for product in products {
        let price_entry = match purchase_option {
            Some(po) => product.prices().purchase_option(po).iter().next(),
            None => product.prices.first(),
        };

        let (price, unit) = price_entry
            .map(|p| (p.usd.as_str(), p.unit.as_str()))
            .unwrap_or(("", ""));

        let desc = product.attribute("description").unwrap_or("");
        // Escape quotes in description for CSV
        let desc_escaped = desc.replace('"', "\"\"");

        println!(
            "{},{},{},{},\"{}\",{},{},{}",
            product.vendor_name,
            product.service,
            product.region.as_deref().unwrap_or(""),
            product.sku,
            desc_escaped,
            product.product_family.as_deref().unwrap_or(""),
            price,
            unit
        );
    }
}
