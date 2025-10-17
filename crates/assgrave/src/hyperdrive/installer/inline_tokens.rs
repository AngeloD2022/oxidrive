use regex::{Captures, Regex, Replacer};

const TOKEN_PATTERN: &str = r"(?m)\[(.*)\]";

struct TokenExpander {
    install_dir: String,
}

impl TokenExpander {
    pub fn new(install_dir: String) -> Self {
        Self { install_dir }
    }

    #[cfg(target_os = "windows")]
    fn win_token_expand(&self, value: &str) -> Option<String> {
        todo!()
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
            .absoluteString()?
            .to_string();

        let append = match value {
            "FontsFolder" => "/Fonts",
            "LibraryPreferences" | "UserPreferences" => "/Preferences",
            "ScriptingAdditions" => "/ScriptingAdditions",
            "InternetPlugins" | "UserInternetPlugins" => "/Internet Plug-Ins",
            "ColorSyncProfiles" => "/ColorSync/Profiles",
            _ => "",
        };

        Some(format!("{}{}", result, append))
    }
}

impl Replacer for TokenExpander {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::hyperdrive::installer::inline_tokens::TOKEN_PATTERN;
    use regex::Regex;

    #[test]
    fn test_regex_pattern() {
        let value = "[UserRoamingAppData]\\Adobe\\Adobe Photoshop 2025\\Logs\\debug.log";
        let regex = Regex::new(TOKEN_PATTERN).unwrap();
        let result = regex.captures_iter(value);

        for mat in result {}
    }
}
