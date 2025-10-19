use anyhow::Result;
use clap::Args;

use super::download::{self, DownloadIntent, DownloadResult};

#[derive(Args, Debug)]
pub struct InstallArgs {
    #[command(flatten)]
    inner: download::DownloadArgs,
}

pub async fn execute(args: InstallArgs) -> Result<()> {
    let request = args.inner.into_request();

    match download::perform_download(&request, DownloadIntent::Install).await? {
        DownloadResult::Completed(summary) => {
            if summary.was_dry_run {
                println!("Dry run complete. No packages were staged.");
            } else {
                println!(
                    "Install payload prepared. Files saved to {}",
                    summary.output.display()
                );
            }
        }
        DownloadResult::Aborted => {}
    }

    Ok(())
}
