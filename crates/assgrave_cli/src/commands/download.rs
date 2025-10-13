use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{Context, Result, anyhow};
use clap::Args;
use console::Style;
use inquire::Confirm;

use assgrave::hyperdrive::{
    Application, ApplicationDownloader, ChannelReduced, DownloadConfiguration, Package,
    ProductPlatform, ProductReduced, ProgressSink,
};

use super::util;

#[derive(Args, Debug)]
pub struct DownloadArgs {
    pub sap_code: String,

    #[arg(long, default_value = "CCM")]
    pub channel: String,

    #[arg(long)]
    pub platform: Option<String>,

    #[arg(long)]
    pub version: Option<String>,

    #[arg(long, default_value = "en_US")] // TODO: detect system locale rather than defaulting
    pub locale: String,

    #[arg(long, default_value = "./downloads")] // TODO: use proper user download dir based on OS
    pub output: PathBuf,

    #[arg(long)]
    pub yes: bool,

    #[arg(long)]
    pub dry_run: bool, // debugging goodie for linux folks or dry runs (duh)
}

pub async fn execute(args: DownloadArgs) -> Result<()> {
    let sap_code = args.sap_code.to_ascii_uppercase();
    let (client, platform) = util::client_from_cli(args.platform.as_deref()).await?;

    let reduced = client
        .get_reduced_channel(&args.channel)
        .ok_or_else(|| anyhow!("channel '{}' has no index", args.channel))?;

    let selection = select_build(&reduced, &sap_code, args.version.as_deref())?;
    let build_guid = selection
        .build_guid
        .ok_or_else(|| anyhow!("product '{}' is missing build GUID", sap_code))?;

    let application = client
        .get_application(build_guid)
        .await
        .with_context(|| format!("failed to fetch build metadata for {build_guid}"))?;

    let product = util::get_product(&client, &args.channel, &sap_code)?;
    let header = Style::new().bold();
    println!(
        "{}",
        header.apply_to(format!(
            "Preparing download: {} {} ({})",
            product.display_name,
            selection.version,
            util::platform_key(&platform)
        ))
    );
    println!("Channel: {}", args.channel);
    println!("Output directory: {}", args.output.display());

    let packages = &application.packages.package;
    let total_download: i64 = packages.iter().map(|pkg| pkg.download_size).sum();
    println!(
        "Total size: {} across {} packages",
        util::format_bytes(total_download),
        packages.len()
    );

    if !args.yes && !args.dry_run {
        let prompt = format!(
            "Download {} {} to {}?",
            product.display_name,
            selection.version,
            args.output.display()
        );

        let proceed = Confirm::new(&prompt)
            .with_default(false)
            .prompt()
            .unwrap_or(false);

        if !proceed {
            println!("Aborted.");
            return Ok(());
        }
    }

    if args.dry_run {
        println!("Dry run enabled — no files will be downloaded.");
        let deps = client
            .get_download_dependencies(&application)
            .await
            .with_context(|| "failed to resolve dependencies")?;
        print_dry_run_summary(&application, deps.as_deref());
        return Ok(());
    }

    let downloader = ApplicationDownloader::new(
        args.output.clone(),
        &application,
        &client,
        DownloadConfiguration::with_locale(&args.locale),
    )
    .await
    .map_err(|err| anyhow!("failed to initialize downloader: {err}"))?;

    let progress = CliProgress::new(&platform);
    downloader
        .start_download(progress)
        .await
        .with_context(|| "download failed")?;

    println!("Download complete.");

    Ok(())
}

fn select_build<'a>(
    channel: &'a ChannelReduced<'a>,
    sap_code: &str,
    version: Option<&str>,
) -> Result<ProductReduced<'a>> {
    if let Some(version) = version {
        channel
            .index
            .get(sap_code, version)
            .ok_or_else(|| anyhow!("version '{version}' not found for '{sap_code}'"))
    } else {
        channel
            .index
            .get_latest(sap_code)
            .ok_or_else(|| anyhow!("no builds found for '{sap_code}'"))
    }
}

struct CliProgress {
    platform: ProductPlatform,
    inner: Mutex<HashMap<String, FileState>>,
}

struct FileState {
    total: usize,
    downloaded: usize,
    last_step: u8,
}

impl CliProgress {
    fn new(platform: &ProductPlatform) -> Self {
        Self {
            platform: *platform,
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl ProgressSink for CliProgress {
    fn on_file_start(&self, file: &str, total_size: usize) {
        let mut state = self.inner.lock().unwrap();
        state.insert(
            file.to_string(),
            FileState {
                total: total_size,
                downloaded: 0,
                last_step: 0,
            },
        );
        println!(
            "→ {} ({} on {})",
            file,
            util::format_bytes(total_size as i64),
            util::platform_key(&self.platform)
        );
    }

    fn on_range_done(&self, file: &str, delta: usize) {
        let mut state = self.inner.lock().unwrap();
        if let Some(entry) = state.get_mut(file) {
            entry.downloaded = (entry.downloaded + delta).min(entry.total);
            let percent = ((entry.downloaded as f64 / entry.total as f64) * 100.0).min(100.0);
            let step = (percent / 10.0).floor() as u8;
            if step > entry.last_step && step < 10 {
                entry.last_step = step;
                println!("   {file}: {}%", step * 10);
            }
        }
    }

    fn on_file_done(&self, file: &str) {
        let mut state = self.inner.lock().unwrap();
        if let Some(entry) = state.remove(file) {
            println!(
                "✓ {} ({} downloaded)",
                file,
                util::format_bytes(entry.total as i64)
            );
        } else {
            println!("✓ {file}");
        }
    }

    fn on_error(&self, file: &str) {
        println!("✗ {file} (error during download)");
    }
}

fn print_dry_run_summary(application: &Application, dependencies: Option<&[Application]>) {
    println!("\nPrimary packages:");
    print_package_rows(&application.packages.package);

    let primary_total: i64 = application
        .packages
        .package
        .iter()
        .map(|pkg| pkg.download_size)
        .sum();

    let dep_total = if let Some(deps) = dependencies {
        println!("\nDependencies ({}):", deps.len());
        let mut total = 0i64;
        for dep in deps {
            let dep_size: i64 = dep
                .packages
                .package
                .iter()
                .map(|pkg| pkg.download_size)
                .sum();
            total += dep_size;
            println!(
                "  {} {} — {}",
                dep.sap_code,
                dep.product_version,
                util::format_bytes(dep_size)
            );
        }
        total
    } else {
        println!("\nDependencies: (none resolved)");
        0
    };

    println!(
        "\nEstimated download volume: {} ({} including dependencies)",
        util::format_bytes(primary_total),
        util::format_bytes(primary_total + dep_total)
    );
}

fn print_package_rows(packages: &[Package]) {
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
