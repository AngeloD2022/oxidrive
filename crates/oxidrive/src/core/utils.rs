use crate::core::models::Package;
use std::path::PathBuf;
use std::str::FromStr;
use sysinfo::System;

pub fn file_name_from_package(pkg: &Package) -> String {
    PathBuf::from_str(pkg.path.as_str())
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

pub fn get_os_version() -> String {
    let _sys = System::new_all();
    let os_version = System::os_version().unwrap_or_else(|| "0".to_owned());
    os_version
}
