use crate::hyperdrive::common::models::Dependency;
use crate::hyperdrive::common::platform::ProductPlatform;
use std::path::PathBuf;

pub struct AppInstallManifest {
    sap_code: String,
    codex_version: String,
    esd_dir: String,
    dependencies: Dependency,
    platform: ProductPlatform,
}
