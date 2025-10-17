pub mod compression;
mod inline_tokens;
mod install;
mod pim;

pub enum InstallConfiguration {
    WindowsInstall { create_file_associations: bool },
    MacInstall,
}
