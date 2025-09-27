use std::path::PathBuf;
use std::time::Duration;
use reqwest::Client;
use reqwest::header::HeaderMap;
use crate::hyperdrive::remote::configure_headers;
use crate::hyperdrive::remote::models::Application;

const CDN_SECURE: &str = "https://ccmdls.adobe.com";

pub trait ProgressSink: Send {
    fn on_file_start(file: &str, total_size: usize) {}
    fn on_range_done(file: &str, delta: usize) {}
    fn on_file_done(file: &str) {}
    fn on_error(file: &str) {}
}

pub struct NoopProgress;
impl ProgressSink for NoopProgress {}

pub struct ProductDownloader<'a> {
    output_dir: PathBuf,
    client: reqwest::Client,
    app_spec: &'a Application,
}

impl<'a> ProductDownloader<'a> {

    pub fn new(output_dir: PathBuf, application: &'a Application) -> Result<Self, Box<dyn std::error::Error>> {
        let mut headers = HeaderMap::new();
        configure_headers(&mut headers);

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(8)
            .default_headers(headers)
            .build()?;

        Ok(Self {output_dir, client, app_spec: application})
    }

    pub async fn start_download<P: ProgressSink>(progress: &mut P) {

        // - use app_spec.packages.package[i].download_size to determine chunk width.
        // - compute ranges
        // - use a tokio semaphore to not go overboard with concurrency

        todo!();
    }
}




