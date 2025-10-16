use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
    #[serde(rename = "Type")]
    type_: String,
    package_name: String,
    package_scheme: String,
    condition: Option<String>,
    assets: Assets,
    commands: Vec<Command>,
}

#[derive(Debug, Deserialize)]
pub struct Assets {
    #[serde(rename = "Asset")]
    asset: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    #[serde(rename = "@source")]
    source: String,

    #[serde(rename = "@target")]
    target: String,

    #[serde(rename = "@recursive")]
    recursive: Option<String>,

    #[serde(rename = "@isRecursiveDelete")]
    is_recursive_delete: Option<String>,

    #[serde(rename = "@isUserPreferences")]
    is_user_preferences: Option<String>,
}

//
// <Registry>
//     <Path>HKEY_CLASSES_ROOT\adbps</Path>
//     <Name>Default</Name>
//     <Type>REG_SZ</Type>
//     <Value>URL:adbps</Value>
// </Registry>
//
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RegistryCommand {
    path: String,
    name: Option<String>,
    #[serde(rename = "Type")]
    type_: Option<String>,
    value: Option<String>,
    #[serde(rename = "@isRecursiveDelete")]
    is_recursive_delete: Option<bool>,
    #[serde(rename = "@isUserPreferences")]
    is_user_preferences: Option<bool>,
}

//<FolderIcon>
//   <FolderPath>[INSTALLDIR]</FolderPath>
//   <IconPath>[AdobeCommon]/Adobe Photoshop 2025/AMT/Core key files/AddRemoveInfo/ps_cc_folder_hidpi.icns</IconPath>
//</FolderIcon>
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FolderIconCommand {
    folder_path: String,
    icon_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PermissionCommand {
    path: String,
    user: Option<String>,
    permission_value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RegisterApplicationCommand {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InstallUserPrefCommand {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OwnerCommand {
    path: String,
    user: String,
    group: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RunProgramCommand {
    #[serde(flatten)]
    install_command: InstallCommand,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InstallCommand {
    #[serde(rename = "@runInUserMode")]
    run_in_user_mode: Option<bool>,

    path: String,
    arguments: Option<Arguments>,
    success_exit_codes: SuccessExitCodes,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Arguments {
    argument: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SuccessExitCodes {
    exit_code: Vec<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ShortcutCommand {
    target: String,
    directory: String,
    name: ShortcutName,
}

#[derive(Debug, Deserialize)]
pub struct ShortcutName {
    #[serde(rename = "Language", default)]
    languages: Vec<LocalizedName>,
}

#[derive(Debug, Deserialize)]
pub struct LocalizedName {
    #[serde(rename = "@locale")]
    locale: String,

    #[serde(rename = "$value")]
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Command {
    Registry(RegistryCommand), // windows only
    FolderIcon(FolderIconCommand),
    Permission(PermissionCommand),
    RegisterApplication(RegisterApplicationCommand), // possibly mac only...?
    InstallUserPref(InstallUserPrefCommand),
    Owner(OwnerCommand),
    RunProgram(RunProgramCommand),
    Shortcut(ShortcutCommand),
}
