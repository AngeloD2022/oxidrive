use anyhow::Result;
use clap::Args;
use console::Style;
use serde_json::json;

use super::util;

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long, default_value = "CCM")]
    pub channel: String,

    #[arg(long)]
    pub filter: Option<String>,

    #[arg(long)]
    pub platform: Option<String>,

    #[arg(long)]
    pub limit: Option<usize>,

    #[arg(long, help = "Emit product metadata as JSON rather than a table")]
    pub json: bool,
}

pub async fn execute(args: ListArgs) -> Result<()> {
    let filter = args.filter.as_ref().map(|f| f.to_ascii_lowercase());
    let (client, platform) = util::client_from_cli(args.platform.as_deref()).await?;
    let channel = util::get_channel(&client, &args.channel)?;

    let mut products: Vec<_> = channel.products.product.iter().collect();
    products.sort_by(|a, b| a.display_name.cmp(&b.display_name));

    let mut visible = Vec::new();
    for product in products {
        if let Some(filter) = &filter {
            let display = product.display_name.to_ascii_lowercase();
            let sap = product.id.to_ascii_lowercase();
            if !display.contains(filter) && !sap.contains(filter) {
                continue;
            }
        }

        visible.push(product);
        if let Some(limit) = args.limit {
            if visible.len() >= limit {
                break;
            }
        }
    }

    if args.json {
        let payload: Vec<_> = visible
            .iter()
            .map(|product| {
                json!({
                    "sap": product.id,
                    "display_name": product.display_name,
                    "version": product.version,
                    "family": product.family,
                    "channel": args.channel,
                    "platform": util::platform_key(&platform),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let header = Style::new().bold();
    println!(
        "{}",
        header.apply_to(format!(
            "Products for channel '{}' on platform {}",
            args.channel,
            util::platform_key(&platform)
        ))
    );
    println!(
        "{:<12} {:<40} {:<12} {:<16}",
        "SAP", "Name", "Version", "Family"
    );
    println!("{:-<12} {:-<40} {:-<12} {:-<16}", "", "", "", "");

    for product in visible {
        println!(
            "{:<12} {:<40} {:<12} {:<16}",
            product.id,
            util::truncate(&product.display_name, 40),
            product.version,
            util::truncate(&product.family, 16)
        );
    }

    Ok(())
}
