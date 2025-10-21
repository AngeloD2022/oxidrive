use anyhow::{Context, Result};
use clap::Args;
use console::Style;
use textwrap::fill;

type ProductsClient = oxidrive::hyperdrive::ProductsClient;
type SupportedLanguages = oxidrive::hyperdrive::SupportedLanguages;

use super::util;

#[derive(Args, Debug)]
pub struct InfoArgs {
    pub sap_code: String,

    #[arg(long, default_value = "CCM")]
    pub channel: String,

    #[arg(long)]
    pub platform: Option<String>,

    #[arg(long)]
    pub version: Option<String>,

    #[arg(long)]
    pub dependencies: bool,

    #[arg(long)]
    pub packages: bool,
}

pub async fn execute(args: InfoArgs) -> Result<()> {
    let sap_code = args.sap_code.to_ascii_uppercase();
    let (client, platform) = util::client_from_cli(args.platform.as_deref()).await?;
    let reduced = client
        .get_reduced_channel(&args.channel)
        .ok_or_else(|| anyhow::anyhow!("channel '{}' has no index", args.channel))?;

    let selected = if let Some(version) = args.version.as_deref() {
        reduced
            .index
            .get(&sap_code, version)
            .ok_or_else(|| anyhow::anyhow!("version '{version}' not found for '{sap_code}'"))?
    } else {
        reduced
            .index
            .get_latest(&sap_code)
            .ok_or_else(|| anyhow::anyhow!("no builds found for '{sap_code}'"))?
    };

    let build_guid = selected
        .build_guid
        .ok_or_else(|| anyhow::anyhow!("product '{sap_code}' is missing build GUID"))?;

    let application = client
        .get_application(build_guid)
        .await
        .with_context(|| format!("failed to load application for build {build_guid}"))?;

    let product = util::get_product(&client, &args.channel, &sap_code)?;
    let header = Style::new().bold();
    println!(
        "{}",
        header.apply_to(format!(
            "{} ({}) — {}",
            product.display_name,
            sap_code,
            util::platform_key(&platform)
        ))
    );
    println!("Channel: {}", args.channel);
    println!("Latest channel version: {}", product.version);
    println!(
        "Selected build: {} (base {})",
        selected.version, application.base_version
    );
    println!("Build GUID: {}", build_guid);
    println!("Language set: {}", application.language_set);

    let preferred_locale = util::detect_locale();

    if let Some(desc) = &application.product_description {
        if let Some(tagline) = select_localized_text(&desc.tagline, preferred_locale.as_deref()) {
            println!("Tagline: {}", tagline);
        }
        if let Some(details) = desc
            .detailed_description
            .as_ref()
            .and_then(|langs| select_localized_text(langs, preferred_locale.as_deref()))
        {
            println!("Description: {}", fill(details, 80));
        }
    }

    let packages = &application.packages.package;
    let total_download: i64 = packages.iter().map(|pkg| pkg.download_size).sum();
    let total_extract: i64 = packages.iter().map(|pkg| pkg.extract_size).sum();
    println!(
        "Packages: {} (download {} / extract {})",
        packages.len(),
        util::format_bytes(total_download),
        util::format_bytes(total_extract)
    );

    if args.packages {
        print_package_table(packages);
    }

    if args.dependencies {
        print_dependencies(&client, &application).await?;
    }

    Ok(())
}

fn select_localized_text<'a>(
    supported: &'a SupportedLanguages,
    preferred_locale: Option<&str>,
) -> Option<&'a str> {
    if supported.language.is_empty() {
        return None;
    }
    let normalized_preferred = preferred_locale.map(normalize_locale);

    if let Some(ref pref) = normalized_preferred {
        if let Some(value) = supported.language.iter().find_map(|lang| {
            let value = lang.value.trim();
            if value.is_empty() {
                return None;
            }

            if normalize_locale(&lang.locale) == *pref {
                Some(value)
            } else {
                None
            }
        }) {
            return Some(value);
        }

        let pref_lang = pref.split('_').next().unwrap_or(pref.as_str());
        if let Some(value) = supported.language.iter().find_map(|lang| {
            let value = lang.value.trim();
            if value.is_empty() {
                return None;
            }

            let locale_norm = normalize_locale(&lang.locale);
            let lang_code = locale_norm.split('_').next().unwrap_or(locale_norm.as_str());
            if lang_code == pref_lang {
                Some(value)
            } else {
                None
            }
        }) {
            return Some(value);
        }
    }

    if let Some(value) = supported.language.iter().find_map(|lang| {
        let value = lang.value.trim();
        if value.is_empty() {
            return None;
        }

        let locale_norm = normalize_locale(&lang.locale);
        let lang_code = locale_norm.split('_').next().unwrap_or(locale_norm.as_str());
        if lang_code == "en" {
            Some(value)
        } else {
            None
        }
    }) {
        return Some(value);
    }

    supported.language.iter().find_map(|lang| {
        let value = lang.value.trim();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

fn normalize_locale(raw: &str) -> String {
    raw.trim().replace('-', "_").to_ascii_lowercase()
}

fn print_package_table(packages: &[oxidrive::hyperdrive::Package]) {
    let header = Style::new().underlined();
    println!("\n{}", header.apply_to("Packages"));
    println!("{:<40} {:<12} {:<12}", "Name", "Download", "Extract");
    println!("{:-<40} {:-<12} {:-<12}", "", "", "");

    for pkg in packages {
        println!(
            "{:<40} {:<12} {:<12}",
            util::truncate(&pkg.package_name, 40),
            util::format_bytes(pkg.download_size),
            util::format_bytes(pkg.extract_size)
        );
    }
}

async fn print_dependencies(
    client: &ProductsClient,
    application: &oxidrive::hyperdrive::Application,
) -> Result<()> {
    println!("\nDependencies:");
    let deps = client
        .get_download_dependencies(application)
        .await
        .with_context(|| "failed to resolve dependencies")?;

    match deps {
        None => println!("  (none)"),
        Some(list) if list.is_empty() => println!("  (none)"),
        Some(list) => {
            for dep in list {
                println!(
                    "  {} {} (GUID {})",
                    dep.sap_code, dep.product_version, dep.build_guid
                );
            }
        }
    }

    Ok(())
}
