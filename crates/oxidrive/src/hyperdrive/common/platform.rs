use crate::strmap;
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub enum ProductPlatform {
    MacAarch64,
    MacIntel64,
    MacIntel32,
    MacUniversal,
    WindowsAarch64,
    WindowsIntel64,
    WindowsIntel32,
}

impl ProductPlatform {
    pub fn detect() -> Option<Self> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match os {
            "macos" => match arch {
                "aarch64" => Some(Self::MacAarch64),
                "x86_64" => Some(Self::MacIntel64),
                "x86" => Some(Self::MacIntel32),
                _ => None,
            },
            "windows" => match arch {
                "aarch64" => Some(Self::WindowsAarch64),
                "x86_64" => Some(Self::WindowsIntel64),
                "x86" => Some(Self::WindowsIntel32),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn to_cdn_key(&self) -> String {
        match self {
            ProductPlatform::MacAarch64 => "macuniversal,macarm64,osx10-64,osx10",
            ProductPlatform::MacIntel64 => "macuniversal,osx10-64",
            ProductPlatform::MacIntel32 => "osx10",
            ProductPlatform::MacUniversal => "macuniversal",
            ProductPlatform::WindowsAarch64 => "winarm64",
            ProductPlatform::WindowsIntel64 => "win64,win32",
            ProductPlatform::WindowsIntel32 => "win32",
        }
        .to_string()
    }

    pub fn allowed_platforms(&self) -> Vec<String> {
        match self {
            ProductPlatform::MacAarch64 => vec!["macuniversal".to_string(), "macarm64".to_string()],
            ProductPlatform::MacIntel64 => vec!["macuniversal".to_string(), "osx10-64".to_string()],
            ProductPlatform::MacIntel32 => vec!["osx10".to_string()],
            ProductPlatform::MacUniversal => vec![],
            ProductPlatform::WindowsAarch64 => vec!["winarm64".to_string()],
            ProductPlatform::WindowsIntel64 => vec!["win64".to_string(), "win32".to_string()],
            ProductPlatform::WindowsIntel32 => vec!["win32".to_string()],
        }
    }

    pub fn get_condition_vars(&self) -> Option<HashMap<String, String>> {
        Some(match self {
            ProductPlatform::MacAarch64 | ProductPlatform::WindowsAarch64 => {
                strmap! {
                    "OSProcessorFamily" => "64-bit",
                    "OSArchitecture" => "arm64",
                }
            }
            ProductPlatform::MacIntel64 | ProductPlatform::WindowsIntel64 => {
                strmap! {
                    "OSProcessorFamily" => "64-bit",
                    "OSArchitecture" => "x64",
                }
            }
            ProductPlatform::MacIntel32 | ProductPlatform::WindowsIntel32 => {
                strmap! {
                    "OSProcessorFamily" => "32-bit",
                }
            }
            ProductPlatform::MacUniversal => return None,
        })
    }

    pub fn is_mac(&self) -> bool {
        matches!(
            self,
            ProductPlatform::MacIntel64
                | ProductPlatform::MacIntel32
                | ProductPlatform::MacAarch64
                | ProductPlatform::MacUniversal
        )
    }
}
