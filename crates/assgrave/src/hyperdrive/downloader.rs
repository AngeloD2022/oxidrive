use crate::hyperdrive::remote::models::{Application, Package};
use crate::hyperdrive::remote::{configure_headers, ProductsClient};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, RANGE};
use reqwest::Client;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;

const CDN_SECURE: &str = "https://ccmdls.adobe.com";

/// For the sake of exposing progress updates without coupling that logic to this crate.
pub trait ProgressSink: Send + Sync {
    fn on_file_start(&self, file: &str, total_size: usize) {}
    fn on_range_done(&self, file: &str, delta: usize) {}
    fn on_file_done(&self, file: &str) {}
    fn on_error(&self, file: &str) {}
}
pub struct NoopProgress;
impl ProgressSink for NoopProgress {}


pub struct DownloadConfiguration {
    locale: String,
}

impl Default for DownloadConfiguration {
    fn default() -> Self {
        Self {
            locale: "en-US".to_string(),
        }
    }
}


pub struct ApplicationDownloader<'a> {
    output_dir: PathBuf,
    client: Client,
    app_spec: &'a Application,
    products_client: &'a ProductsClient,
    download_cfg: DownloadConfiguration,
}

impl<'a> ApplicationDownloader<'a> {
    pub async fn new(
        output_dir: PathBuf,
        application: &'a Application,
        products_client: &'a ProductsClient,
        download_cfg: DownloadConfiguration,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut headers = HeaderMap::new();
        configure_headers(&mut headers);

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(8)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            output_dir,
            client,
            app_spec: application,
            products_client,
            download_cfg,
        })
    }

    pub fn prepare_directory(&self) {
        if !self.output_dir.exists() {
            fs::create_dir_all(&self.output_dir).unwrap();
        }
    }

    fn compute_ranges_for_pkg(&self, pkg: &Package) -> Vec<(u64, u64)> {
        let size = pkg.download_size as u64;
        const CHUNK_SIZE: u64 = 10_000_000;

        let mut ranges = Vec::with_capacity(((size + CHUNK_SIZE - 1) / CHUNK_SIZE) as usize);
        let mut start = 0;

        while start < size {
            let end = (start + CHUNK_SIZE).min(size) - 1; // end is inclusive
            ranges.push((start, end));
            start = end + 1;
        }

        ranges
    }

    async fn exec_download_for_pkg<P: ProgressSink + 'static>(
        &self,
        pkg: &Package,
        out_dir: &PathBuf,
        progress: Arc<P>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pkg = Arc::new(pkg);
        let ranges = self.compute_ranges_for_pkg(&pkg);
        let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = Vec::with_capacity(ranges.len());
        let file_size = pkg.download_size;

        // Product
        //  - SAPCode (application)
        //    - package files

        let file_name = pkg.full_package_name.as_ref().unwrap().clone();
        let file_path = out_dir.join(PathBuf::from_str(&file_name)?);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(file_path)
            .await?;
        file.set_len(file_size as u64).await?;
        let file = Arc::new(Mutex::new(file));

        const MAX_CONCURRENCY: usize = 8;

        let file_name = Arc::new(file_name);
        let url = Arc::new(format!("{}{}", CDN_SECURE, pkg.path));
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
        let client = Arc::new(self.client.clone());
        let progress = Arc::new(progress);

        for (start, end) in ranges {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = client.clone();
            let url = url.clone();
            let file = file.clone();
            let progress = progress.clone();
            let file_name = file_name.clone();
            // let pkg = pkg.clone();

            progress.on_file_start(&file_name, file_size as usize);
            tasks.push(tokio::spawn(
                async move {
                    let _p = permit;

                    let resp = client
                        .get(&*url)
                        .header(RANGE, format!("bytes={}-{}", start, end))
                        .send()
                        .await?
                        .error_for_status()?;

                    let mut stream = resp.bytes_stream();

                    let mut buf = Vec::with_capacity((end - start + 1) as usize);
                    while let Some(chunk) = stream.next().await {
                        let x = chunk?;
                        buf.extend_from_slice(&x);
                        progress.on_range_done(&file_name, x.len());
                    }

                    let mut f = file.lock().await;
                    f.seek(std::io::SeekFrom::Start(start)).await?;
                    f.write_all(&buf).await?;
                    f.flush().await?;

                    Ok(())
                },
            ));
        }

        for t in tasks {
            t.await??;
        }

        progress.on_file_done(&file_name);
        Ok(())
    }

    pub async fn start_download<P: ProgressSink + 'static>(
        &self,
        progress: P,
    ) -> Result<(), reqwest::Error> {
        // - compute ranges
        // - use a tokio semaphore to not go overboard with concurrency

        let dependencies = self
            .products_client
            .get_download_dependencies(&self.app_spec)
            .await?;

        let progress = Arc::new(progress);

        let main_application_dir = self.output_dir.join(&self.app_spec.sap_code);
        fs::create_dir(main_application_dir.clone())
            .expect("Failed to create main application directory.");

        // todo: filter application packages based on download configuration.
        // todo: error handling.

        for pkg in &self.app_spec.packages.package {
            self.exec_download_for_pkg(pkg, &main_application_dir, progress.clone()).await;
        }

        if let Some(deps) = dependencies {
            for dep in &deps {
                let application_dir = &self.output_dir.join(&dep.sap_code);
                for pkg in &dep.packages.package {
                    self.exec_download_for_pkg(pkg, &application_dir, progress.clone()).await;
                }
            }
        }

        Ok(())
    }
}
