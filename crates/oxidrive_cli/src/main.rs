mod cli;
mod progress;
mod system;
mod wizard;
mod logging;

use crate::cli::InstallCommand;
use crate::progress::IndicatifDownloadProgress;
use crate::system::cache_dir_name_from_application;
use anyhow::{Result, anyhow};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use cli::{Cli, Command};
// use log::{error, info};
use oxidrive::core::condition::EvalValue;
use oxidrive::core::platform::ProductPlatform;
use oxidrive::core::utils::get_os_version;
use oxidrive::remote::downloader::{ApplicationDownloader, DownloadConfiguration};
use oxidrive::remote::products::ProductsClient;
use std::fs;
use tokio::runtime::Runtime;
use oxidrive::installer::install::{InstallConfig, NoopInstallProgress, ProductInstaller};
use oxidrive::installer::os_actions::{system_default_backend, DebugBackend};

async fn install_routine(cmd: &InstallCommand) -> anyhow::Result<()> {
    let product_sap = cmd.product.to_sap();
    log_info!(
        "INIT",
        "Selected product: {}, version: {}",
        product_sap,
        cmd.version.as_deref().unwrap_or("latest").to_string()
    );

    log_info!("INIT", "Detecting platform...");
    let platform = ProductPlatform::detect();
    if let Some(platform) = platform {
        log_info!("INIT", "Platform: {}", platform.to_key());
        log_info!(
            "INIT",
            "ApplicationPlatform: {}",
            platform.application_platform().to_key()
        );

        let mut client = ProductsClient::new(platform)
            .await
            .map_err(|e| anyhow!("Failed to initialize products API client: {}", e))?;

        log_info!("REMOTE", "Resolving selection...");
        let channel = client
            .get_reduced_channel("CCM")
            .ok_or(anyhow!("Cannot obtain CCM channel."))?;

        let reduced_info = if let Some(version) = &cmd.version {
            channel
                .index
                .get(&product_sap, &version, platform.application_platform())
        } else {
            channel
                .index
                .get_latest(&product_sap, platform.application_platform())
        }
        .ok_or(anyhow!(
            "Could not resolve selection: {}, version: {}",
            product_sap,
            cmd.version.as_deref().unwrap_or("latest")
        ))?;

        if let None = reduced_info.build_guid {
            return Err(anyhow!(
                "Selected product does not include a build GUID, which is required for downloading."
            ));
        }

        let product = client
            .get_application(reduced_info.build_guid.unwrap())
            .await
            .map_err(|e| anyhow!("Failed to retrieve application metadata: {}", e))?;

        log_info!("REMOTE", "Application: {}", product.name);

        log_info!("REMOTE", "Resolving dependencies...");
        let mut to_install = client
            .get_download_dependencies(&product)
            .await
            .map_err(|e| anyhow!("Unable to resolve product dependencies: {}", e))?
            .unwrap_or(Vec::new());

        log_info!(
            "REMOTE",
            "Resolved dependencies: {}",
            to_install
                .iter()
                .map(|a| format!("{} v{}", a.sap_code.to_string(), a.version.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let default_cache_dir_name = cache_dir_name_from_application(&product);

        let mut dl_cfg = DownloadConfiguration::default();
        dl_cfg.locale = cmd.language.clone();

        let base_dir = cmd
            .out_dir
            .clone()
            .unwrap_or(system::cache_dir().unwrap().join(default_cache_dir_name));

        log_info!("DOWNLOAD", "Target directory: {}", base_dir.display());

        fs::create_dir_all(&base_dir)
            .map_err(|e| anyhow!("Couldn't create download directory: {e}"))?;

        let download_progress = IndicatifDownloadProgress::new();
        let downloader = ApplicationDownloader::new(base_dir.clone(), &product, &client, dl_cfg)
            .await
            .map_err(|e| anyhow!("Failed to initialize downloader: {e}"))?;

        downloader
            .start_download(download_progress)
            .await
            .map_err(|e| anyhow!("Download failure: {e}"))?;

        to_install.push(product);

        let backend =
            if cmd.dry_run {
                Box::new(DebugBackend {})
            } else {
                system_default_backend()
            };

        let progress = NoopInstallProgress {};

        let install_cfg = InstallConfig {
            language: cmd.language.clone()
        };

        let installer = ProductInstaller::new_with_apps(
            product_sap,
            base_dir,
            install_cfg,
            to_install,
            backend
        );

        installer.prewarm()
            .map_err(|e| anyhow!("Installer prewarm error: {e}"))?;

        installer.run(progress)
            .map_err(|e| anyhow!("Install error: {e}"))?;

        Ok(())
    } else {
        Err(anyhow!("Incompatible installation platform."))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cli::parse_args();

    let Cli { command } = cli;
    match command {
        Command::Wizard => {
            todo!("Not implemented!")
        }
        Command::Install(command) => install_routine(&command).await?,
        Command::Package => {
            todo!()
        }
        Command::List => {
            todo!()
        }
    }

    Ok(())
}
