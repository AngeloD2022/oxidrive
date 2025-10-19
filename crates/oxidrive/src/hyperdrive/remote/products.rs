use crate::hyperdrive::common::models::{Application, Products};
use crate::hyperdrive::common::platform::ProductPlatform;
use crate::hyperdrive::remote::index::ChannelReduced;
use log::{error, info, warn};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub(crate) fn configure_headers(headers: &mut HeaderMap) {
    let extra = vec![
        ("x-api-key", "CC_HD_ESD_1_0"),
        ("x-adobe-app-id", "accc-apps-panel-desktop"),
    ];

    for (name, val) in extra {
        headers.append(HeaderName::from_static(name), HeaderValue::from_static(val));
    }
}

pub async fn get_products(platform: &str) -> Result<Products, reqwest::Error> {
    let url = format!(
        "https://prod-rel-ffc-ccm.oobesaas.adobe.com/adobe-ffc-external/core/v6/products/all?\
        _type=json&channel=ccm&channel=sti&platform={}&productType=Desktop",
        platform
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
}

impl ProductsClient {
    pub async fn new(platform: ProductPlatform) -> Result<Self, reqwest::Error> {
        let products = get_products(&platform.to_cdn_key()).await?;

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
        Some(ChannelReduced::from_channel(
            channel,
            &self.platform.allowed_platforms(),
        ))
    }

    pub async fn get_download_dependencies(
        &self,
        application: &Application,
    ) -> Result<Option<Vec<Application>>, reqwest::Error> {
        info!(
            "Resolving dependencies for {} {}...",
            &application.sap_code, &application.base_version
        );

        // Resolve dependencies of application using Products index.
        if let Some(dependencies) = &application.dependencies {
            let mut result = Vec::new();

            // Build an index of the channel for easy navigation.
            let reduced_sti = self.get_reduced_channel("STI").unwrap();
            let reduced_ccm = self.get_reduced_channel("CCM").unwrap();

            for dep in &dependencies.dependency {
                let product = if let Some(product) =
                    reduced_ccm.index.get(&dep.sap_code, &dep.base_version)
                {
                    Some(product)
                } else if let Some(product) =
                    reduced_sti.index.get(&dep.sap_code, &dep.base_version)
                {
                    Some(product)
                } else {
                    error!(
                        "Could not resolve dependency: {} {}",
                        &dep.sap_code, &dep.base_version
                    );
                    None
                };

                if let Some(product) = product {
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
    use crate::hyperdrive::common::platform::ProductPlatform;
    use crate::hyperdrive::remote::products::{ProductsClient, get_products};

    #[tokio::test]
    async fn test_get_products() {
        let platform = ProductPlatform::MacAarch64;
        let response = get_products(&platform.to_cdn_key()).await.unwrap();
    }

    #[tokio::test]
    async fn test_products_client() {
        let mut client = ProductsClient::new(ProductPlatform::MacAarch64).await;

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
