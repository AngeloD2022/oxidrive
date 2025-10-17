///
/// Adobe Inline Path Token Expansion
///

use regex::{Captures, Regex, Replacer};

const TOKEN_PATTERN: &str = r"(?m)\[(.*)\]";

struct TokenExpander {
    install_dir: String,
}

impl TokenExpander {
    pub fn new(install_dir: String) -> Self {
        Self { install_dir }
    }

    pub fn expand(&mut self, value: &str) -> Result<String, ()> {
        let regex = Regex::new(TOKEN_PATTERN).unwrap();
        Ok(regex.replace_all(value, self).to_string())
    }

    fn expand_token(&self, token: &str) -> Option<String> {
        match token {
            "INSTALLDIR" => Some(self.install_dir.clone()),
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
            "SharedDocuments" => (NSSearchPathDomainMask::LocalDomainMask, NSSearchPathDirectory::UserDirectory),
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
        todo!()
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
    use crate::hyperdrive::installer::inline_tokens::{TokenExpander, TOKEN_PATTERN};
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
        let mut exp = TokenExpander::new("smth".to_string());

        let r = exp.expand(&value).unwrap();

        println!("{}", r);
    }
}
