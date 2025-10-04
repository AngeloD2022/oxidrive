use crate::hyperdrive::remote::index::ChannelReduced;
use models::*;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

mod index;
pub mod models;

#[derive(Copy, Clone)]
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
            ProductPlatform::MacOSUniversal => "osx10-64,osx10,macarm64,macuniversal",
            ProductPlatform::WindowsAarch64 => "winarm64",
            ProductPlatform::WindowsIntel64 => "win64",
            ProductPlatform::WindowsIntel32 => "win32",
        }
        .to_string()
    }
}

pub(crate) fn configure_headers(headers: &mut HeaderMap) {
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
        "https://prod-rel-ffc-ccm.oobesaas.adobe.com/adobe-ffc-external/core/v6/products/all?\
        _type=json&channel=ccm&channel=sti&platform={}&productType=Desktop",
        platform.to_cdn_key()
    );

    let mut headers = HeaderMap::new();
    configure_headers(&mut headers);

    let client = Client::builder().default_headers(headers).build().unwrap();

    let response = client.get(url).send().await?.error_for_status()?;
    let data = response.json::<Products>().await?;

    Ok(data)
}

pub struct ProductsClient {
    products: Products,
    client: Client,
    platform: ProductPlatform,
    // application_cache: HashMap<String, Application>,
}

impl ProductsClient {
    pub async fn new(platform: ProductPlatform) -> Result<Self, reqwest::Error> {
        let products = get_products(&platform).await?;

        let mut headers = HeaderMap::new();
        configure_headers(&mut headers);

        let client = Client::builder().default_headers(headers).build().unwrap();

        Ok(ProductsClient {
            products,
            client,
            platform,
        })
    }

    pub fn platform(&self) -> ProductPlatform {
        self.platform.clone()
    }

    pub async fn get_application(&self, build_guid: &str) -> Result<Application, reqwest::Error> {
        const URL: &str = "https://cdn-ffc.oobesaas.adobe.com/core/v3/applications";

        let r = self
            .client
            .get(URL)
            .header("x-adobe-build-guid", build_guid)
            .send()
            .await?;

        // let data = r.json::<Application>().await?;
        let raw = r.text().await?;

        // YES, I NEED TO FUCKING DO IT THIS WAY FOR SOME REASON.
        // I can't use the deserializer off of reqwest because it keeps producing an unexplainable
        // duplicate key error. FUCK.
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let resp: Application = serde_json::from_value(value).unwrap();

        Ok(resp)
    }

    pub fn get_reduced_channel(&self, name: &str) -> Option<ChannelReduced> {
        let channel = self
            .products
            .channels
            .channel
            .iter()
            .find(|ch| ch.name.to_ascii_lowercase() == name.to_ascii_lowercase())?;
        Some(ChannelReduced::from_channel(channel))
    }

    pub async fn get_download_dependencies(
        &self,
        application: &Application,
    ) -> Result<Option<Vec<Application>>, reqwest::Error> {
        // Resolve dependencies of application using Products index.
        if let Some(dependencies) = &application.dependencies {
            let mut result = Vec::new();

            // Build an index of the channel for easy navigation.
            let reduced = self.get_reduced_channel("STI").unwrap();

            for dep in &dependencies.dependency {
                if let Some(product) = reduced.index.get(&dep.sap_code, &dep.base_version) {
                    if let Some(build_guid) = product.build_guid {
                        let resolved_application = self.get_application(build_guid).await?;
                        result.push(resolved_application);
                    }
                }
            }

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
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
    async fn test_products_client() {
        let mut client = ProductsClient::new(ProductPlatform::MacOSUniversal).await;

        let mut client = client.unwrap();
        let ch = client.get_reduced_channel("CCM").unwrap();
        let latest = ch.index.get_latest("PHSP").unwrap();

        let guid = latest.build_guid.unwrap().to_owned();
        let application = client.get_application(&guid).await.unwrap();

        let ae_deps = client
            .get_download_dependencies(&application)
            .await
            .unwrap()
            .unwrap();
        println!("{}", ae_deps.len())
    }
}
