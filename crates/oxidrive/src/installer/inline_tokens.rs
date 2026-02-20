//!
//! Adobe Inline Path Token Expansion
//!

// use log::warn;
use regex::{Captures, Regex, Replacer};

const TOKEN_PATTERN: &str = r"\[([^\]]+)\]";

pub struct TokenExpander {
    install_dir: Option<String>,
}

impl TokenExpander {
    pub fn new(install_dir: &str) -> Self {
        let mut inst = Self::without_vars();

        let install_dir = inst.expand(&install_dir).unwrap();
        let install_dir = install_dir.strip_suffix('/').unwrap_or(&install_dir);
        Self {
            install_dir: Some(install_dir.to_string()),
        }
    }

    fn without_vars() -> Self {
        Self { install_dir: None }
    }

    pub fn expand(&mut self, value: &str) -> Result<String, ()> {
        let regex = Regex::new(TOKEN_PATTERN).unwrap();
        Ok(regex.replace_all(value, self).to_string())
    }

    fn expand_token(&self, token: &str) -> Option<String> {
        match token {
            "INSTALLDIR" | "InstallDir" => self.install_dir.clone(),
            "StagingFolder" => Some("".to_string()),
            _ => {
                #[cfg(target_os = "macos")]
                let r = self.macos_token_expand(token);

                #[cfg(target_os = "windows")]
                let r = self.win_token_expand(token);

                r
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn macos_token_expand(&self, value: &str) -> Option<String> {
        // from: HDPIM.dylib @ FolderResolver::ExpandPathKey
        use objc2_foundation::NSFileManager;
        use objc2_foundation::{NSHomeDirectory, NSSearchPathDirectory, NSSearchPathDomainMask};

        if value == "UserHome" {
            let value = NSHomeDirectory();
            return Some(value.to_string());
        }

        let (domain, sp_directory) = match value {
            "AdobeProgramFiles" | "ProgramFiles" | "Utilities" => (
                NSSearchPathDomainMask::LocalDomainMask,
                NSSearchPathDirectory::ApplicationDirectory,
            ),
            "AdobeCommon" | "Common" | "SharedApplicationData" | "_OOBEHome" => (
                NSSearchPathDomainMask::LocalDomainMask,
                NSSearchPathDirectory::ApplicationSupportDirectory,
            ),
            "Library" | "FontsFolder" | "LibraryPreferences" | "ScriptingAdditions"
            | "InternetPlugins" | "ColorSyncProfiles" => (
                NSSearchPathDomainMask::LocalDomainMask,
                NSSearchPathDirectory::LibraryDirectory,
            ),
            "UserInternetPlugins" | "UserPreferences" => (
                NSSearchPathDomainMask::UserDomainMask,
                NSSearchPathDirectory::LibraryDirectory,
            ),
            "UserCommon" => (
                NSSearchPathDomainMask::UserDomainMask,
                NSSearchPathDirectory::ApplicationSupportDirectory,
            ),
            "SharedDocuments" => (
                NSSearchPathDomainMask::LocalDomainMask,
                NSSearchPathDirectory::UserDirectory,
            ),
            "UserDocuments" => (
                NSSearchPathDomainMask::UserDomainMask,
                NSSearchPathDirectory::DocumentDirectory,
            ),
            "UserDesktop" => (
                NSSearchPathDomainMask::UserDomainMask,
                NSSearchPathDirectory::DesktopDirectory,
            ),
            _ => return None,
        };

        let file_manager = NSFileManager::defaultManager();

        let result = file_manager
            .URLForDirectory_inDomain_appropriateForURL_create_error(
                sp_directory,
                domain,
                None,
                true,
            )
            .ok()?
            .path()?
            .to_string();

        let append = match value {
            "Utilities" => "/Utilities",
            "SharedDocuments" => "/Shared",
            "AdobeCommon" => "/Adobe",
            "FontsFolder" => "/Fonts",
            "LibraryPreferences" | "UserPreferences" => "/Preferences",
            "ScriptingAdditions" => "/ScriptingAdditions",
            "InternetPlugins" | "UserInternetPlugins" => "/Internet Plug-Ins",
            "ColorSyncProfiles" => "/ColorSync/Profiles",
            _ => "",
        };

        Some(format!("{}{}", result, append))
    }

    #[cfg(target_os = "windows")]
    fn win_token_expand(&self, value: &str) -> Option<String> {
        // note: HDPIM used the SHGetFolderPathW function, which is deprecated.
        //  We will be using the newer SHGetKnownFolderPath function.

        use windows::Win32::System::Com::{
            COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree,
        };
        use windows::Win32::UI::Shell::SHGetKnownFolderPath;
        use windows::Win32::UI::Shell::{
            FOLDERID_CommonPrograms, FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Favorites,
            FOLDERID_Fonts, FOLDERID_LocalAppData, FOLDERID_Pictures, FOLDERID_Profile,
            FOLDERID_ProgramData, FOLDERID_ProgramFiles, FOLDERID_ProgramFilesCommon,
            FOLDERID_ProgramFilesCommonX86, FOLDERID_ProgramFilesX86, FOLDERID_PublicDocuments,
            FOLDERID_RoamingAppData, FOLDERID_System, FOLDERID_Templates, KF_FLAG_DEFAULT,
        };

        let placeholder_flag = true;

        let folder_id = match value {
            "AdobeCommon" => {
                if placeholder_flag {
                    FOLDERID_ProgramFilesCommon
                } else {
                    FOLDERID_ProgramFilesCommonX86
                }
            }
            "AdobeProgramFiles" => {
                if placeholder_flag {
                    FOLDERID_ProgramFiles
                } else {
                    FOLDERID_ProgramFilesX86
                }
            }
            "FontsFolder" => FOLDERID_Fonts,
            "Common" => FOLDERID_ProgramFilesCommon,
            "CommonX86" => FOLDERID_ProgramFilesCommonX86,
            "ProgramFiles" => FOLDERID_ProgramFiles,
            "ProgramFilesX86" => FOLDERID_ProgramFilesX86,
            "SharedApplicationData" => FOLDERID_ProgramData,
            "SharedDocuments" => FOLDERID_PublicDocuments,
            "StartMenu" | "StartMenuSubFolder" => FOLDERID_CommonPrograms,
            "System32Folder" | "System" => FOLDERID_System,
            "UserHome" => FOLDERID_Profile,
            "UserDocuments" => FOLDERID_Documents,
            "UserRoamingAppData" => FOLDERID_RoamingAppData,
            "UserLocalAppData" => FOLDERID_LocalAppData,
            "UserFavorites" => FOLDERID_Favorites,
            "UserPictures" => FOLDERID_Pictures,
            "UserTemplates" => FOLDERID_Templates,
            "UserDesktop" => FOLDERID_Desktop,
            "SharedDesktop" => FOLDERID_Desktop,
            _ => {
                eprintln!("Unhandled path macro: {}", value);
                return None;
            }
        };

        let result = unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            match SHGetKnownFolderPath(&folder_id, KF_FLAG_DEFAULT, None) {
                Ok(path) => {
                    let path_str = path.to_string().ok()?;
                    CoTaskMemFree(Some(path.0 as _));
                    path_str
                }
                Err(_) => return None,
            }
        };

        let append = match value {
            "AdobeCommon" | "AdobeProgramFiles" => "\\Adobe",
            _ => "",
        };

        Some(format!("{}{}", result, append))
    }
}

impl Replacer for &mut TokenExpander {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        if let Some(token_match) = caps.get(1) {
            let token = token_match.as_str();

            if let Some(path) = self.expand_token(token) {
                dst.push_str(&path);
            } else {
                // Keep og token if expansion fails
                dst.push_str(&caps[0]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::installer::inline_tokens::{TOKEN_PATTERN, TokenExpander};
    use regex::Regex;

    #[test]
    fn test_regex_pattern() {
        let value = "[UserRoamingAppData]\\Adobe\\Adobe Photoshop 2025\\Logs\\debug.log";
        let regex = Regex::new(TOKEN_PATTERN).unwrap();
        let result = regex.captures_iter(value);

        for mat in result {}
    }

    #[test]
    fn test_expander_macos() {
        let value = "[AdobeCommon]/Adobe Photoshop 2025/AMT/Core key files/AddRemoveInfo/ps_cc_folder_plugin.icns";
        let mut exp = TokenExpander::new("smth");

        let r = exp.expand(&value).unwrap();

        println!("{}", r);
    }
}
