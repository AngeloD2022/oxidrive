use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::Args;
use console::Style;
use inquire::Confirm;
use tokio::sync::Mutex;

use oxidrive::hyperdrive::{
    Application, ApplicationDownloader, ChannelReduced, ConditionEvaluator, DownloadConfiguration,
    Package, ProductPlatform, ProductReduced, ProductsClient, ProgressSink, parse_condition,
};

use super::util;

#[derive(Clone, Debug)]
pub struct DownloadRequest {
    pub sap_code: String,
    pub channel: String,
    pub platform: Option<String>,
    pub version: Option<String>,
    pub locale: String,
    pub output: PathBuf,
    pub confirm: bool,
    pub dry_run: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum DownloadIntent {
    Download,
    Install,
}

impl DownloadIntent {
    fn preparation_label(&self) -> &'static str {
        match self {
            Self::Download => "Preparing download",
            Self::Install => "Preparing install",
        }
    }

    fn verb(&self) -> &'static str {
        match self {
            Self::Download => "download",
            Self::Install => "install",
        }
    }
}

impl fmt::Display for DownloadIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Download => write!(f, "Download packages"),
            Self::Install => write!(f, "Install (stage packages)"),
        }
    }
}

pub struct DownloadSummary {
    pub product_name: String,
    pub sap_code: String,
    pub channel: String,
    pub version: String,
    pub output: PathBuf,
    pub locale: String,
    pub platform: ProductPlatform,
    pub was_dry_run: bool,
}

pub enum DownloadResult {
    Completed(DownloadSummary),
    Aborted,
}

const DEFAULT_DOWNLOAD_DIR: &str = "./downloads";

#[derive(Args, Debug)]
#[command(disable_version_flag = true)]
pub struct DownloadArgs {
    pub sap_code: String,

    #[arg(
        long,
        default_value = "CCM",
        help = "Channel code to use when resolving builds"
    )]
    pub channel: String,

    #[arg(long, help = "Explicit platform flag (e.g. win64, mac-arm64)")]
    pub platform: Option<String>,

    #[arg(
        long,
        help = "Build platform using OS + architecture (e.g. --os windows --arch arm)"
    )]
    pub os: Option<String>,

    #[arg(long)]
    pub arch: Option<String>,

    #[arg(
        long,
        help = "Explicit product version to download (defaults to latest available)"
    )]
    pub version: Option<String>,

    #[arg(long, help = "Locale to request when resolving packages")]
    pub locale: Option<String>,

    #[arg(
        short = 'o',
        long,
        value_name = "DIR",
        default_value = DEFAULT_DOWNLOAD_DIR,
        help = "Directory where packages should be saved"
    )]
    pub output: PathBuf,

    #[arg(
        short = 'y',
        long,
        help = "Approve the download without prompting for confirmation"
    )]
    pub yes: bool,

    #[arg(long, help = "Produce a dry run report without downloading packages")]
    pub dry_run: bool,
}

impl DownloadArgs {
    pub fn into_request(self) -> DownloadRequest {
        let platform_flag: Option<String> = self.platform.or_else(|| {
            util::platform_from_components(self.os.as_deref(), self.arch.as_deref())
                .map(|platform| util::platform_key(&platform).to_string())
        });

        let locale = match self.locale.or_else(util::detect_locale) {
            Some(locale) => locale,
            None => {
                eprintln!("Unable to detect the system locale; falling back to en_US");
                "en_US".to_string()
            }
        };

        DownloadRequest {
            sap_code: self.sap_code,
            channel: self.channel,
            platform: platform_flag,
            version: self.version,
            locale,
            output: self.output,
            confirm: !self.yes,
            dry_run: self.dry_run,
        }
    }
}

pub async fn execute(args: DownloadArgs) -> Result<()> {
    let request = args.into_request();

    match perform_download(&request, DownloadIntent::Download).await? {
        DownloadResult::Completed(summary) => {
            if summary.was_dry_run {
                println!("Dry run complete. No packages were downloaded.");
            } else {
                println!(
                    "Download complete. Files saved to {}",
                    summary.output.display()
                );
            }
        }
        DownloadResult::Aborted => {}
    }

    Ok(())
}

pub async fn perform_download(
    request: &DownloadRequest,
    intent: DownloadIntent,
) -> Result<DownloadResult> {
    let (client, platform) = util::client_from_cli(request.platform.as_deref()).await?;
    perform_download_with_client(&client, platform, request, intent).await
}

pub async fn perform_download_with_client(
    client: &ProductsClient,
    platform: ProductPlatform,
    request: &DownloadRequest,
    intent: DownloadIntent,
) -> Result<DownloadResult> {
    let sap_code = request.sap_code.to_ascii_uppercase();
    let reduced = client
        .get_reduced_channel(&request.channel)
        .ok_or_else(|| anyhow!("channel '{}' has no index", request.channel))?;

    let selection = select_build(&reduced, &sap_code, request.version.as_deref())?;
    let build_guid = selection
        .build_guid
        .ok_or_else(|| anyhow!("product '{sap_code}' is missing build GUID"))?;

    let application = client
        .get_application(build_guid)
        .await
        .with_context(|| format!("failed to fetch build metadata for {build_guid}"))?;

    let product = util::get_product(client, &request.channel, &sap_code)?;
    let header = Style::new().bold();
    println!(
        "{}",
        header.apply_to(format!(
            "{}: {} {} ({})",
            intent.preparation_label(),
            product.display_name,
            selection.version,
            util::platform_key(&platform)
        ))
    );
    println!("Channel: {}", request.channel);
    println!("Output directory: {}", request.output.display());

    let filtered_packages =
        filter_packages_for_locale(client, &request.locale, &application.packages.package);
    let total_download: i64 = filtered_packages.iter().map(|pkg| pkg.download_size).sum();
    println!(
        "Total size: {} across {} packages",
        util::format_bytes(total_download),
        filtered_packages.len()
    );
    if filtered_packages.is_empty() {
        println!(
            "No packages matched locale {}; try overriding with --locale if this looks wrong.",
            request.locale
        );
    }

    if request.confirm && !request.dry_run {
        let prompt = format!(
            "{} {} {} to {}?",
            intent.verb().to_ascii_uppercase(),
            product.display_name,
            selection.version,
            request.output.display()
        );

        let proceed = Confirm::new(&prompt)
            .with_default(false)
            .prompt()
            .unwrap_or(false);

        if !proceed {
            println!("Aborted.");
            return Ok(DownloadResult::Aborted);
        }
    }

    if request.dry_run {
        println!("Dry run enabled — no files will be downloaded.");
        let deps = client
            .get_download_dependencies(&application)
            .await
            .with_context(|| "failed to resolve dependencies")?;
        let dependency_summaries = deps.as_ref().map(|items| {
            items
                .iter()
                .map(|dep| DependencyDryRun {
                    application: dep,
                    packages: filter_packages_for_locale(
                        client,
                        &request.locale,
                        &dep.packages.package,
                    ),
                })
                .collect::<Vec<_>>()
        });
        print_dry_run_summary(&filtered_packages, dependency_summaries.as_deref());

        let summary = DownloadSummary {
            product_name: product.display_name.clone(),
            sap_code,
            channel: request.channel.clone(),
            version: selection.version.to_string(),
            output: request.output.clone(),
            locale: request.locale.clone(),
            platform,
            was_dry_run: true,
        };

        return Ok(DownloadResult::Completed(summary));
    }

    let mut download_cfg = DownloadConfiguration::default();
    download_cfg.set_locale(request.locale.clone());

    let downloader =
        ApplicationDownloader::new(request.output.clone(), &application, client, download_cfg)
            .await
            .map_err(|err| anyhow!("failed to initialize downloader: {err}"))?;

    let progress = CliProgress::new(platform);
    downloader
        .start_download(progress)
        .await
        .with_context(|| "download failed")?;

    let summary = DownloadSummary {
        product_name: product.display_name.clone(),
        sap_code,
        channel: request.channel.clone(),
        version: selection.version.to_string(),
        output: request.output.clone(),
        locale: request.locale.clone(),
        platform,
        was_dry_run: false,
    };

    Ok(DownloadResult::Completed(summary))
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
    fn new(platform: ProductPlatform) -> Self {
        Self {
            platform,
            inner: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl ProgressSink for CliProgress {
    async fn on_file_start(&self, file: &str, total_size: usize) {
        let mut state = self.inner.lock().await;
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

    async fn on_range_done(&self, file: &str, delta: usize) {
        let mut state = self.inner.lock().await;
        if let Some(entry) = state.get_mut(file) {
            entry.downloaded = (entry.downloaded + delta).min(entry.total);
            let percent = (entry.downloaded as f64 / entry.total as f64) * 100.0;
            let step = (percent / 10.0).floor() as u8;
            if step > entry.last_step && step < 10 {
                entry.last_step = step;
                println!("   {}: {}%", file, step * 10);
            }
        }
    }

    async fn on_file_done(&self, file: &str) {
        let mut state = self.inner.lock().await;
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

    async fn on_error(&self, file: &str) {
        println!("✗ {file} (error during download)");
    }
}

fn filter_packages_for_locale<'a>(
    client: &ProductsClient,
    locale: &str,
    packages: &'a [Package],
) -> Vec<&'a Package> {
    let mut vars = client.platform().get_condition_vars().unwrap_or_default();

    let cfg = DownloadConfiguration::default();
    vars.insert("OSVersion".to_string(), cfg.os_version().to_string());
    vars.insert("installLanguage".to_string(), locale.to_string());

    let evaluator = ConditionEvaluator::new(vars, false);

    packages
        .iter()
        .filter(|pkg| {
            match pkg
                .condition
                .as_ref()
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
            {
                Some(condition) => match parse_condition(condition) {
                    Ok(expr) => evaluator.evaluate(&expr).unwrap_or_else(|err| {
                        eprintln!("Failed to evaluate condition '{}': {}", condition, err);
                        true
                    }),
                    Err(err) => {
                        eprintln!("Failed to parse condition '{}': {}", condition, err);
                        true
                    }
                },
                None => true,
            }
        })
        .collect()
}

struct DependencyDryRun<'a> {
    application: &'a Application,
    packages: Vec<&'a Package>,
}

fn print_dry_run_summary<'a>(
    primary_packages: &[&'a Package],
    dependencies: Option<&[DependencyDryRun<'a>]>,
) {
    println!("\nPrimary packages:");
    print_package_rows(primary_packages);

    let primary_total: i64 = primary_packages.iter().map(|pkg| pkg.download_size).sum();

    let dep_total = if let Some(deps) = dependencies {
        if deps.is_empty() {
            println!("\nDependencies: (none resolved)");
            0
        } else {
            println!("\nDependencies ({}):", deps.len());
            let mut total = 0i64;
            for dep in deps {
                let dep_size: i64 = dep.packages.iter().map(|pkg| pkg.download_size).sum();
                total += dep_size;
                if dep.packages.is_empty() {
                    println!(
                        "  {} {} — 0 B (no locale-matching packages)",
                        dep.application.sap_code, dep.application.product_version
                    );
                } else {
                    println!(
                        "  {} {} — {}",
                        dep.application.sap_code,
                        dep.application.product_version,
                        util::format_bytes(dep_size)
                    );
                }
            }
            total
        }
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

fn print_package_rows(packages: &[&Package]) {
    println!("{:<40} {:<12} {:<12}", "Name", "Download", "Extract");
    println!("{:-<40} {:-<12} {:-<12}", "", "", "");
    if packages.is_empty() {
        println!("(no packages matched the requested locale)");
        return;
    }

    for pkg in packages {
        println!(
            "{:<40} {:<12} {:<12}",
            util::truncate(&pkg.package_name, 40),
            util::format_bytes(pkg.download_size),
            util::format_bytes(pkg.extract_size)
        );
    }
}
