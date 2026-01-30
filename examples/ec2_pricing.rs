//! Example: Query AWS EC2 pricing
//!
//! Run with: cargo run --example ec2_pricing

use infracost::Client;

#[tokio::main]
async fn main() -> Result<(), infracost::Error> {
    let client = Client::from_env()?;

    // Query t3.micro on-demand Linux pricing in us-east-1
    let products = client
        .products()
        .vendor("aws")
        .service("AmazonEC2")
        .region("us-east-1")
        .product_family("Compute Instance")
        .attribute("instanceType", "t3.micro")
        .attribute("operatingSystem", "Linux")
        .attribute("tenancy", "Shared")
        .attribute("capacitystatus", "Used")
        .attribute("preInstalledSw", "NA")
        .fetch()
        .await?;

    println!(
        "Found {} products for t3.micro Linux in us-east-1:\n",
        products.len()
    );

    for product in &products {
        println!("SKU: {}", product.sku);
        println!("  vCPU: {}", product.attribute("vcpu").unwrap_or("N/A"));
        println!("  Memory: {}", product.attribute("memory").unwrap_or("N/A"));

        // Filter for on-demand pricing using the price filter
        if let Ok(on_demand_price) = product
            .prices()
            .unit("Hrs")
            .description("On Demand")
            .first()
        {
            println!("  On-Demand Price: ${}/hour", on_demand_price.usd);

            let hourly = on_demand_price.usd_f64()?;
            let monthly = hourly * 24.0 * 30.0;
            println!("  Monthly Estimate: ${:.2}", monthly);
        }
        println!();
    }

    Ok(())
}
