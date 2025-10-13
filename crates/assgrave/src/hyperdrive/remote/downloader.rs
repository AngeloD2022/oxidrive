use crate::hyperdrive::remote::condition::{ConditionEvaluator, parse_condition};
use crate::hyperdrive::remote::models::{Application, Package, PackageKind};
use crate::hyperdrive::remote::{ProductsClient, configure_headers};
use crate::strmap;
use futures_util::StreamExt;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, RANGE, USER_AGENT};
use std::collections::HashMap;
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
            locale: "en_US".to_string(),
        }
    }
}

impl DownloadConfiguration {
    pub fn with_locale<S: Into<String>>(locale: S) -> Self {
        Self {
            locale: locale.into(),
        }
    }

    pub fn locale(&self) -> &str {
        &self.locale
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

        headers.insert(USER_AGENT, HeaderValue::from_static("Creative Cloud"));

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
    ) -> anyhow::Result<()> {
        let pkg = Arc::new(pkg.clone()); // or keep &Package and clone fields you need
        let ranges = self.compute_ranges_for_pkg(&pkg);
        let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = Vec::with_capacity(ranges.len());
        let file_size = pkg.download_size;

        let file_name = PathBuf::from_str(pkg.path.as_str())
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let file_path = out_dir.join(&file_name);
        // parent should exist; caller ensures create_dir_all
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&file_path)
            .await?;
        file.set_len(file_size as u64).await?;
        let file = Arc::new(Mutex::new(file));

        const MAX_CONCURRENCY: usize = 2;

        let file_name = Arc::new(file_name);
        let url = Arc::new(format!("{}{}", CDN_SECURE, pkg.path));
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
        let client = Arc::new(self.client.clone());

        // Emit start once per file (not per chunk)
        progress.on_file_start(&file_name, file_size as usize);

        for (start, end) in ranges {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = client.clone();
            let url = url.clone();
            let file = file.clone();
            let progress = progress.clone();
            let file_name = file_name.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = permit;
                let mut attempts = 0;

                loop {
                    let resp = client
                        .get(&*url)
                        .header(RANGE, format!("bytes={}-{}", start, end))
                        .send()
                        .await;

                    if let Err(req_err) = resp {
                        println!("PROBLEM: {}, {} to {}", url, start, end);
                        // fixme: Decoding the response seems to be a reoccurring problem.
                        //  Apparently, some kind of better retrying is needed.
                        //  Only way to mitigate it currently is by reducing the parallelism.
                        if attempts == 5 {
                            return Err(req_err.into());
                        }
                        attempts += 1;
                        continue;
                    }

                    let resp = resp.unwrap();
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

                    break;
                }
                Ok(())
            }));
        }

        for t in tasks {
            t.await??;
        }

        progress.on_file_done(&file_name);
        Ok(())
    }

    fn filter_pkgs_for_config(&self, pkgs: &'a [Package]) -> impl Iterator<Item = &Package> {
        // todo: include more details here.
        let mut vars = strmap! {
            "installLanguage" => self.download_cfg.locale,
            "OSProcessorFamily" => "64-bit",
            "OSArchitecture" => "arm64"
        };
        let cev = ConditionEvaluator::new(vars, false);

        let filtered: Vec<_> = pkgs
            .iter()
            .filter(|pkg| {
                if let Some(pkg_type) = &pkg.package_type {
                    match pkg_type {
                        PackageKind::Core | PackageKind::Resources => return true,
                        _ => {}
                    }
                }
                if let Some(condition) = &pkg.condition {
                    println!("Parsing condition: {}", condition);
                    let parsed = parse_condition(condition).unwrap();
                    cev.evaluate(&parsed).unwrap()
                } else {
                    true
                }
            })
            .collect();

        filtered.into_iter()
    }

    pub async fn start_download<P: ProgressSink + 'static>(
        &self,
        progress: P,
    ) -> anyhow::Result<()> {
        self.prepare_directory();

        let dependencies = self
            .products_client
            .get_download_dependencies(&self.app_spec)
            .await?;

        let progress = Arc::new(progress);

        let main_application_dir = self.output_dir.join(&self.app_spec.sap_code);
        fs::create_dir_all(&main_application_dir)?; // ensure exists

        // main packages
        for pkg in self.filter_pkgs_for_config(&self.app_spec.packages.package) {
            self.exec_download_for_pkg(pkg, &main_application_dir, progress.clone())
                .await?;
        }

        // dependencies
        if let Some(deps) = dependencies {
            for dep in &deps {
                let application_dir = self.output_dir.join(&dep.sap_code);
                fs::create_dir_all(&application_dir)?; // ensure exists
                for pkg in self.filter_pkgs_for_config(&dep.packages.package) {
                    self.exec_download_for_pkg(pkg, &application_dir, progress.clone())
                        .await?;
                }
            }
        }

        Ok(())
    }
}

mod tests {
    use crate::hyperdrive::remote::downloader::{
        ApplicationDownloader, DownloadConfiguration, NoopProgress, ProgressSink,
    };
    use crate::hyperdrive::remote::{ProductPlatform, ProductsClient};
    use std::path::PathBuf;
    use std::str::FromStr;

    struct TestConsoleProgress {}
    impl ProgressSink for TestConsoleProgress {
        fn on_file_start(&self, file: &str, total_size: usize) {
            println!("Started: {} with {} bytes", file, total_size);
        }

        fn on_range_done(&self, file: &str, delta: usize) {
            println!("Downloaded {} bytes for {}", delta, file);
        }

        fn on_file_done(&self, file: &str) {
            println!("Finished downloading {}!", file);
        }
    }

    #[tokio::test]
    async fn test_application_downloader() {
        let pc = ProductsClient::new(ProductPlatform::MacOSUniversal)
            .await
            .unwrap();

        let path =
            PathBuf::from_str("/Users/angelodeluca/RustroverProjects/assgrave/dl_test").unwrap();

        let channel = pc.get_reduced_channel("CCM").unwrap();
        let product = channel.index.get_latest("AEFT").unwrap();

        let application = pc
            .get_application(product.build_guid.unwrap())
            .await
            .unwrap();

        let dl_cfg = DownloadConfiguration::default();

        let downloader = ApplicationDownloader::new(path, &application, &pc, dl_cfg)
            .await
            .unwrap();

        let progress = TestConsoleProgress {};

        downloader.start_download(progress).await.unwrap();
    }
}
