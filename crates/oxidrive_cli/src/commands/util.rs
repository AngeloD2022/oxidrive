use anyhow::{Context, Result, anyhow};
use oxidrive::hyperdrive::{Channel, Product, ProductPlatform, ProductsClient};
use std::env;

pub async fn client_from_cli(
    platform_flag: Option<&str>,
) -> Result<(ProductsClient, ProductPlatform)> {
    let platform = resolve_platform(platform_flag)?;
    let client = ProductsClient::new(platform).await.with_context(|| {
        format!(
            "failed to initialize products client for {:?}",
            platform_key(&platform)
        )
    })?;

    Ok((client, platform))
}

pub fn resolve_platform(flag: Option<&str>) -> Result<ProductPlatform> {
    if let Some(flag) = flag {
        return parse_platform(flag).ok_or_else(|| anyhow!("unknown platform '{flag}'"));
    }

    if let Some(platform) = ProductPlatform::detect() {
        Ok(platform)
    } else {
        eprintln!(
            "No compatible platform detected; defaulting to Windows 64-bit (override with --platform)."
        );
        Ok(ProductPlatform::WindowsIntel64)
    }
}

fn parse_platform(raw: &str) -> Option<ProductPlatform> {
    let normalized = raw.to_ascii_lowercase();
    match normalized.as_str() {
        "auto" => ProductPlatform::detect(),
        "mac-arm64" | "macarm64" | "mac_m1" | "mac-aarch64" => Some(ProductPlatform::MacAarch64),
        "mac-intel64" | "mac64" | "mac-x86_64" => Some(ProductPlatform::MacIntel64),
        "mac-intel32" | "mac32" | "mac-x86" => Some(ProductPlatform::MacIntel32),
        "mac-universal" | "macos-universal" | "mac-universal2" | "mac" | "macos" => {
            Some(ProductPlatform::MacUniversal)
        }
        "win-arm64" | "winarm64" | "windows-arm64" => Some(ProductPlatform::WindowsAarch64),
        "win64" | "windows" | "windows64" | "windows-x86_64" | "win" => {
            Some(ProductPlatform::WindowsIntel64)
        }
        "win32" | "windows32" | "windows-x86" => Some(ProductPlatform::WindowsIntel32),
        other => {
            if let Some(platform) = ProductPlatform::detect() {
                if other == "auto" {
                    return Some(platform);
                }
            }
            None
        }
    }
}

pub fn platform_key(platform: &ProductPlatform) -> &'static str {
    match platform {
        ProductPlatform::MacAarch64 => "mac-arm64",
        ProductPlatform::MacIntel64 => "mac-intel64",
        ProductPlatform::MacIntel32 => "mac-intel32",
        ProductPlatform::MacUniversal => "mac-universal",
        ProductPlatform::WindowsAarch64 => "win-arm64",
        ProductPlatform::WindowsIntel64 => "win64",
        ProductPlatform::WindowsIntel32 => "win32",
    }
}

pub fn platform_from_components(os: Option<&str>, arch: Option<&str>) -> Option<ProductPlatform> {
    let os = os?.to_ascii_lowercase();
    let arch = arch?.to_ascii_lowercase();

    match (os.as_str(), arch.as_str()) {
        ("mac", "arm64") | ("macos", "arm64") | ("darwin", "arm64") => {
            Some(ProductPlatform::MacAarch64)
        }
        ("mac", "x86_64") | ("macos", "x86_64") | ("darwin", "x86_64") => {
            Some(ProductPlatform::MacIntel64)
        }
        ("mac", "x86") | ("macos", "x86") | ("darwin", "x86") => Some(ProductPlatform::MacIntel32),
        ("mac", "universal") | ("macos", "universal") => Some(ProductPlatform::MacUniversal),
        ("windows", "arm64") | ("win", "arm64") => Some(ProductPlatform::WindowsAarch64),
        ("windows", "x86_64") | ("windows", "amd64") | ("win", "x86_64") | ("win", "amd64") => {
            Some(ProductPlatform::WindowsIntel64)
        }
        ("windows", "x86") | ("win", "x86") | ("windows", "i686") | ("win", "i686") => {
            Some(ProductPlatform::WindowsIntel32)
        }
        _ => None,
    }
}

pub fn get_channel<'a>(client: &'a ProductsClient, channel: &str) -> Result<&'a Channel> {
    client
        .channel(channel)
        .ok_or_else(|| anyhow!("channel '{channel}' not found"))
}

pub fn get_product<'a>(
    client: &'a ProductsClient,
    channel: &str,
    sap_code: &str,
) -> Result<&'a Product> {
    client
        .product_in_channel(channel, sap_code)
        .ok_or_else(|| anyhow!("product '{sap_code}' not found in channel '{channel}'"))
}

pub fn format_bytes(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }

    if idx == 0 {
        format!("{bytes} {}", UNITS[idx])
    } else {
        format!("{value:.2} {}", UNITS[idx])
    }
}

pub fn truncate(value: &str, width: usize) -> String {
    if value.len() <= width {
        value.to_string()
    } else if width > 3 {
        format!("{}...", &value[..width - 3])
    } else {
        value[..width].to_string()
    }
}

pub fn detect_locale() -> Option<String> {
    let candidates = [
        env::var("LC_ALL").ok(),
        env::var("LC_MESSAGES").ok(),
        env::var("LANG").ok(),
        env::var("LANGUAGE").ok(),
    ];

    for candidate in candidates.into_iter().flatten() {
        if let Some(parsed) = sanitize_locale(&candidate) {
            return Some(parsed);
        }
    }

    None
}

fn sanitize_locale(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let locale = trimmed
        .split(['.', '@'])
        .next()
        .unwrap_or(trimmed)
        .split(':')
        .next()
        .unwrap_or(trimmed)
        .replace('-', "_");

    let mut parts = locale.split('_');
    let lang = parts.next()?.to_ascii_lowercase();
    let region = parts.next().map(|r| r.to_ascii_uppercase());

    region.map(|region| format!("{}_{}", lang, region))
}
