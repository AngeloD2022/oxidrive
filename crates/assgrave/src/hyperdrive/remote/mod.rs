use models::*;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

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


fn configure_headers(headers: &mut HeaderMap) {
    let extra = vec![
        ("x-api-key", "CC_HD_ESD_1_0"),
        ("x-adobe-app-id", "accc-apps-panel-desktop"),
    ];

    for (name, val) in extra {
        headers.append(HeaderName::from_static(name), HeaderValue::from_static(val));
    }
}

pub async fn get_products(platform: &ProductPlatform) -> Result<Products, reqwest::Error> {
    let url = format!(
        "https://prod-rel-ffc-ccm.oobesaas.adobe.com/adobe-ffc-external/core/v6/products/all?_type=json&channel=ccm&channel=sti&platform={}&productType=Desktop",
        platform.to_cdn_key()
    );

    let mut headers = HeaderMap::new();
    configure_headers(&mut headers);

    let client = Client::builder().default_headers(headers).build().unwrap();

    let response = client.get(url).send().await?.error_for_status()?;
    let data = response.json::<Products>().await?;

    Ok(data)
}

pub async fn get_application(build_guid: &str) -> Result<Application, reqwest::Error> {
    const url: &str = "https://cdn-ffc.oobesaas.adobe.com/core/v3/applications";

    let mut headers = HeaderMap::new();
    configure_headers(&mut headers);

    headers.append(HeaderName::from_static("x-adobe-build-guid"), build_guid.parse().unwrap());

    let client = Client::builder().default_headers(headers).build().unwrap();

    let response = client.get(url).send().await?.error_for_status()?;
    let data = response.json::<Application>().await?;

    Ok(data)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_products() {
        let platform = ProductPlatform::MacAarch64;
        let response = get_products(&platform).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_application() {
        let guid = "5b1886a2-0714-4b2c-be37-bfd622522da6";
        let response  = get_application(guid).await.unwrap();
    }

}