use crate::hyperdrive::remote::condition::{ConditionEvaluator, parse_condition};
use crate::hyperdrive::remote::models::{Application, Package, PackageKind};
use crate::hyperdrive::remote::{ProductsClient, configure_headers};
use crate::strmap;
use futures_util::StreamExt;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue, RANGE, USER_AGENT};
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
#[async_trait::async_trait]
pub trait ProgressSink: Send + Sync {
    async fn on_file_start(&self, file: &str, total_size: usize) {}
    async fn on_range_done(&self, file: &str, delta: usize) {}
    async fn on_file_done(&self, file: &str) {}
    async fn on_error(&self, file: &str) {}
}
pub struct NoopProgress;
impl ProgressSink for NoopProgress {}

pub struct DownloadConfiguration {
    locale: String,
    os_version: String,
}

impl Default for DownloadConfiguration {
    fn default() -> Self {
        Self {
            locale: "en_US".to_string(),
            os_version: utils::get_os_version(),
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

        const MAX_CONCURRENCY: usize = 5;

        let file_name = Arc::new(file_name);
        let url = Arc::new(format!("{}{}", CDN_SECURE, pkg.path));
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
        let client = Arc::new(self.client.clone());

        // Emit start once per file (not per chunk)
        progress.on_file_start(&file_name, file_size as usize).await;

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
                const MAX_ATTEMPTS: usize = 5;

                loop {
                    let resp = client
                        .get(&*url)
                        .header(RANGE, format!("bytes={}-{}", start, end))
                        .send()
                        .await;

                    match resp {
                        Err(req_err) => {
                            println!("Request Failed: {}, {} to {}", url, start, end);
                            if attempts >= MAX_ATTEMPTS {
                                return Err(req_err.into());
                            }
                            attempts += 1;
                            tokio::time::sleep(Duration::from_millis(1000 * attempts as u64)).await;
                            continue;
                        }
                        Ok(resp) => {
                            let mut stream = resp.bytes_stream();
                            let mut buffer = Vec::with_capacity((end - start + 1) as usize);

                            let stream_res: anyhow::Result<()> = async {
                                while let Some(chunk) = stream.next().await {
                                    let x = chunk?;
                                    buffer.extend_from_slice(&x);
                                    progress.on_range_done(&file_name, x.len()).await;
                                }
                                Ok(())
                            }
                            .await;

                            if let Err(e) = stream_res {
                                println!("Streaming failed: {}, {} to {}", url, start, end);
                                if attempts >= MAX_ATTEMPTS {
                                    return Err(e);
                                }
                                attempts += 1;
                                tokio::time::sleep(Duration::from_millis(1000 * attempts as u64))
                                    .await;
                                continue;
                            }

                            let mut f = file.lock().await;
                            f.seek(std::io::SeekFrom::Start(start)).await?;
                            f.write_all(&buffer).await?;
                            f.flush().await?;

                            break;
                        }
                    }
                }
                Ok(())
            }));
        }

        for t in tasks {
            t.await??;
        }

        progress.on_file_done(&file_name).await;
        Ok(())
    }

    fn filter_pkgs_for_config(&self, pkgs: &'a [Package]) -> impl Iterator<Item = &Package> {
        // todo: include more details here.
        let mut vars = strmap! {
            "installLanguage" => self.download_cfg.locale,
            "OSProcessorFamily" => "64-bit",
            "OSArchitecture" => "arm64",
            "OSVersion" => "26.0.1",
        };
        let cev = ConditionEvaluator::new(vars, false);

        let filtered: Vec<_> = pkgs
            .iter()
            .filter(|pkg| {
                if let Some(condition) = &pkg.condition {
                    if condition.len() == 0 {
                        return true;
                    }

                    println!("Evaluating expression: {}", condition);
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
        ApplicationDownloader, DownloadConfiguration, ProgressSink,
    };
    use crate::hyperdrive::remote::products::{ProductPlatform, ProductsClient};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::str::FromStr;
    use tokio::sync::Mutex;

    struct TestConsoleProgress {
        files: Mutex<HashMap<String, usize>>,
        progress: Mutex<HashMap<String, usize>>,
    }

    impl TestConsoleProgress {
        fn new() -> Self {
            Self {
                files: Mutex::new(HashMap::new()),
                progress: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl ProgressSink for TestConsoleProgress {
        async fn on_file_start(&self, file: &str, total_size: usize) {
            println!("\nStarted: {} with {} bytes", file, total_size);
            let mut files = self.files.lock().await;
            let mut progress = self.progress.lock().await;
            files.insert(file.to_string(), total_size);
            progress.insert(file.to_string(), 0);
        }

        async fn on_range_done(&self, file: &str, delta: usize) {
            let file = file.to_string();
            let mut files = self.files.lock().await;
            let mut progress = self.progress.lock().await;
            let n = progress[&file] + delta;
            let total = files[&file];
            *progress.entry(file.to_string()).or_insert(0) = n;

            let percent = (n as f32 / total as f32 * 10_000_f32).round() / 100_f32;

            print!("\r{} - {}% downloaded", file, percent);
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }

        async fn on_file_done(&self, file: &str) {
            println!("\nFinished downloading {}!", file);
        }
    }

    #[tokio::test]
    async fn test_application_downloader() {
        let pc = ProductsClient::new(ProductPlatform::MacAarch64)
            .await
            .unwrap();

        let path =
            PathBuf::from_str("/Users/angelodeluca/RustroverProjects/assgrave/dl_test").unwrap();

        let channel = pc.get_reduced_channel("CCM").unwrap();
        let product = channel.index.get_latest("PHSP").unwrap();

        let application = pc
            .get_application(product.build_guid.unwrap())
            .await
            .unwrap();

        let dl_cfg = DownloadConfiguration::default();

        let downloader = ApplicationDownloader::new(path, &application, &pc, dl_cfg)
            .await
            .unwrap();

        let progress = TestConsoleProgress::new();

        downloader.start_download(progress).await.unwrap();
    }
}
