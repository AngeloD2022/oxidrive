use crate::core::condition::EvalValue;
use std::collections::HashMap;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum ProductPlatform {
    MacAarch64,
    MacIntel64,
    MacIntel32,
    MacUniversal,
    WindowsAarch64,
    WindowsIntel64,
    WindowsIntel32,
}

macro_rules! hashmap {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = HashMap::new();
        $(map.insert($key, $val);)*
        map
    }};
}

macro_rules! s {
    ($e:expr) => {
        $e.to_string()
    };
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

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "macuniversal" => Some(Self::MacUniversal),
            "macarm64" => Some(Self::MacAarch64),
            "osx10-64" => Some(Self::MacIntel64),
            "osx10" => Some(Self::MacIntel32),
            "win64" => Some(Self::WindowsIntel64),
            "win32" => Some(Self::WindowsIntel32),
            "winarm64" => Some(Self::WindowsAarch64),
            _ => None,
        }
    }

    pub fn to_key(&self) -> &'static str {
        match self {
            Self::MacUniversal => "macuniversal",
            Self::MacAarch64 => "macarm64",
            Self::MacIntel64 => "osx10-64",
            Self::MacIntel32 => "osx10",
            Self::WindowsIntel64 => "win64",
            Self::WindowsIntel32 => "win32",
            Self::WindowsAarch64 => "winarm64",
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
            ProductPlatform::MacAarch64 => vec!["macuniversal", "macarm64"],
            ProductPlatform::MacIntel64 => vec!["macuniversal", "osx10-64"],
            ProductPlatform::MacIntel32 => vec!["osx10"],
            ProductPlatform::MacUniversal => vec![],
            ProductPlatform::WindowsAarch64 => vec!["winarm64", "win64", "win32"],
            ProductPlatform::WindowsIntel64 => vec!["win64", "win32"],
            ProductPlatform::WindowsIntel32 => vec!["win32"],
        }
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    pub fn get_condition_vars(&self) -> Option<HashMap<String, EvalValue>> {
        Some(match self {
            // ProductPlatform::MacAarch64  => {
            //     hashmap! {
            //         s!("OSProcessorFamily") => EvalValue::String(s!("64-bit")),
            //         s!("OSArchitecture") => EvalValue::StringSet(vec![s!("arm64")]),
            //     }
            // }
            ProductPlatform::WindowsAarch64 | ProductPlatform::MacAarch64 => {
                hashmap! {
                    s!("OSProcessorFamily") => EvalValue::String(s!("64-bit")),
                    s!("OSArchitecture") => EvalValue::String(s!("arm64")),
                }
            }
            ProductPlatform::MacIntel64 | ProductPlatform::WindowsIntel64 => {
                hashmap! {
                    s!("OSProcessorFamily") => EvalValue::String(s!("64-bit")),
                    s!("OSArchitecture") => EvalValue::String(s!("x64")),
                }
            }
            ProductPlatform::MacIntel32 | ProductPlatform::WindowsIntel32 => {
                hashmap! {
                    s!("OSProcessorFamily") => EvalValue::String(s!("32-bit")),
                }
            }
            ProductPlatform::MacUniversal => return None,
        })
    }

    pub fn application_platform(&self) -> Self {
        match self {
            ProductPlatform::MacAarch64 => ProductPlatform::MacUniversal,
            ProductPlatform::WindowsAarch64 => ProductPlatform::WindowsIntel64,
            _ => *self,
        }
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
