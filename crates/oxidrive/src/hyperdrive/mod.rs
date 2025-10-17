pub mod compression;
mod installer;
mod remote;

pub use remote::downloader::{ApplicationDownloader, DownloadConfiguration, ProgressSink};
pub use remote::models::{Application, Channel, Package, Product};
pub use remote::{ChannelReduced, ProductPlatform, ProductReduced, ProductsClient};
