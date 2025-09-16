use models::*;

mod models;

pub enum ProductPlatform {
    MacAarch64,
    MacIntel64,
    MacIntel32,
    MacOSUniversal,
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
                "aarch64" => Some(Self::MacOSUniversal),
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
            ProductPlatform::MacAarch64 => "macarm64",
            ProductPlatform::MacIntel64 => "osx10-64",
            ProductPlatform::MacIntel32 => "osx10",
            ProductPlatform::MacOSUniversal => "macuniversal",
            ProductPlatform::WindowsAarch64 => "winarm64",
            ProductPlatform::WindowsIntel64 => "win64",
            ProductPlatform::WindowsIntel32 => "win32",
        }
        .to_string()
    }
}

pub fn get_products(platform: &ProductPlatform) -> Result<products::ProductsRoot, ()> {
    todo!();
}

pub fn get_application() -> Result<applications::ApplicationRoot, ()> {
    todo!();
}
