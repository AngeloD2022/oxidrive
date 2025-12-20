use directories::ProjectDirs;
use lazy_static::lazy_static;
use oxidrive::core::models::Application;
use std::path::PathBuf;

lazy_static! {
    static ref PDIRS: Option<ProjectDirs> = ProjectDirs::from("com", "angelod", "oxidrive");
}

pub fn appdata_dir() -> Option<PathBuf> {
    PDIRS.as_ref().map(|pd| pd.data_dir().to_path_buf())
}

pub fn config_dir() -> Option<PathBuf> {
    PDIRS.as_ref().map(|pd| pd.config_dir().to_path_buf())
}

pub fn cache_dir() -> Option<PathBuf> {
    PDIRS.as_ref().map(|pd| pd.cache_dir().to_path_buf())
}

pub fn cache_dir_name_from_application(app: &Application) -> String {
    format!("{}_{}_{}", app.sap_code, app.version, app.platform)
}
