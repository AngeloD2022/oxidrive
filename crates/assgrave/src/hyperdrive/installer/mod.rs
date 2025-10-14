mod pim;
pub mod compression;

pub enum InstallConfiguration {
    WindowsInstall {
        create_file_associations: bool
    },
    MacInstall
}
