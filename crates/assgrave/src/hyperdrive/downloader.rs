use crate::hyperdrive::remote::models::{Application, Package};
use crate::hyperdrive::remote::{ProductsClient, configure_headers};
use reqwest::Client;
use reqwest::header::HeaderMap;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::time::Duration;

const CDN_SECURE: &str = "https://ccmdls.adobe.com";

pub trait ProgressSink: Send {
    fn on_file_start(file: &str, total_size: usize) {}
    fn on_range_done(file: &str, delta: usize) {}
    fn on_file_done(file: &str) {}
    fn on_error(file: &str) {}
}
pub struct NoopProgress;
impl ProgressSink for NoopProgress {}

pub struct ApplicationDownloader<'a> {
    output_dir: PathBuf,
    client: Client,
    app_spec: &'a Application,
    products_client: &'a ProductsClient,
}

impl<'a> ApplicationDownloader<'a> {
    pub async fn new(
        output_dir: PathBuf,
        application: &'a Application,
        products_client: &'a ProductsClient,
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
            let end = (start + CHUNK_SIZE).min(size) - 1;  // end is inclusive
            ranges.push((start, end));
            start = end + 1;
        }

        ranges
    }

    pub async fn start_download<P: ProgressSink>(
        &self,
        progress: &mut P,
    ) -> Result<(), reqwest::Error> {
        // - compute ranges
        // - use a tokio semaphore to not go overboard with concurrency

        let dependencies = self
            .products_client
            .get_download_dependencies(&self.app_spec)
            .await?;

        todo!();
    }
}
