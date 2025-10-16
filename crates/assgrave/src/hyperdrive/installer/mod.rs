pub mod compression;
mod pim;
mod install;

pub enum InstallConfiguration {
    WindowsInstall { create_file_associations: bool },
    MacInstall,
}
