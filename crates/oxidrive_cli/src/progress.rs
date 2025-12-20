use async_trait::async_trait;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use oxidrive::remote::downloader::ProgressSink;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct IndicatifDownloadProgress {
    multi: MultiProgress,
    bars: Mutex<HashMap<String, ProgressBar>>,
}

impl IndicatifDownloadProgress {
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ProgressSink for IndicatifDownloadProgress {
    async fn on_file_start(&self, file: &str, total_size: usize) {
        let pb = self.multi.add(ProgressBar::new(total_size as u64));

        pb.set_style(ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}"
        ).unwrap()
            .progress_chars("#>-"));

        pb.set_message(file.to_string());

        let mut bars = self.bars.lock().await;
        bars.insert(file.to_string(), pb);
    }

    async fn on_range_done(&self, file: &str, delta: usize) {
        let bars = self.bars.lock().await;
        if let Some(pb) = bars.get(file) {
            pb.inc(delta as u64);
        }
    }

    async fn on_file_done(&self, file: &str) {
        let mut bars = self.bars.lock().await;
        if let Some(pb) = bars.remove(file) {
            pb.finish_with_message(format!("✅ {}", file));
        }
    }

    async fn on_error(&self, file: &str) {
        let mut bars = self.bars.lock().await;
        if let Some(pb) = bars.remove(file) {
            pb.abandon_with_message(format!("❌ {}", file));
        }
    }
}
