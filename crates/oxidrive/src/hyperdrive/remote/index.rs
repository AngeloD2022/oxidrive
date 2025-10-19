use crate::hyperdrive::common::models::{Channel, Product};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SAPCode(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version(String);

#[derive(Debug, Clone, Copy)]
pub struct ProductReduced<'a> {
    pub sap_code: &'a str,
    pub version: &'a str,
    pub build_guid: Option<&'a str>,
}

#[derive(Debug, Default)]
pub struct ChannelIndex<'a> {
    pub(crate) by_key: HashMap<(SAPCode, Version), ProductReduced<'a>>,
    latest_by_sap: HashMap<SAPCode, ProductReduced<'a>>,
}

impl<'a> ChannelIndex<'a> {
    pub fn get(&self, code: &str, version: &str) -> Option<ProductReduced<'a>> {
        self.by_key
            .get(&(SAPCode(code.to_string()), Version(version.to_string())))
            .copied()
    }

    pub fn get_latest(&self, code: &str) -> Option<ProductReduced<'a>> {
        self.latest_by_sap
            .get(&(SAPCode(code.to_string())))
            .copied()
    }

    pub fn versions(&self, code: &str) -> Vec<&'a str> {
        let mut versions: Vec<&'a str> = self
            .by_key
            .values()
            .filter(|product| product.sap_code.eq_ignore_ascii_case(code))
            .map(|product| product.version)
            .collect();

        versions.sort_by(|a, b| {
            if ver_newer(a, b) {
                Ordering::Less
            } else if ver_newer(b, a) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        versions.dedup();
        versions
    }
}

fn ver_newer(a: &str, b: &str) -> bool {
    let aspl = a.split(".");
    let bspl: Vec<_> = b.split(".").collect();

    for (i, ap) in aspl.enumerate() {
        if i == bspl.len() {
            return false;
        }
        let a = u32::from_str(ap).unwrap();
        let b = u32::from_str(bspl[i]).unwrap();
        if a == b {
            continue;
        }
        return a > b;
    }

    false
}

pub struct ChannelReduced<'a> {
    code: String,
    pub index: ChannelIndex<'a>,
}

impl<'a> ChannelReduced<'a> {
    // fn get_version_for_product(channel: &Channel, product: &Product) -> String {
    //     let lang_set = &product.platforms.platform[0].language_set[0];
    //     let base_version = lang_set.base_version.unwrap_or();
    //
    //     if product.id == "APRO" {
    //         let needle = channel.
    //     }
    // }

    pub fn from_channel(channel: &'a Channel, allowed_platforms: &[String]) -> Self {
        let channel_code = channel.name.to_owned();

        // For channel index:
        let mut by_key = HashMap::new();
        let mut latest_by_sap: HashMap<SAPCode, ProductReduced> = HashMap::new();

        for product in &channel.products.product {
            for platform in &product.platforms.platform {
                if !allowed_platforms.contains(&platform.id) {
                    continue;
                }

                let lang_set = &platform.language_set[0];
                let prod_ver = product.version.to_owned();
                let key = (
                    SAPCode(product.id.to_owned()),
                    Version(
                        lang_set
                            .base_version
                            .as_ref()
                            .unwrap_or(&prod_ver)
                            .to_owned(),
                    ),
                );

                // construct reduced product.
                let sap = product.id.as_str();
                let version = product.version.as_str();
                let build_guid = product.platforms.platform[0].language_set[0]
                    .build_guid
                    .as_deref();

                let reduced = ProductReduced {
                    sap_code: sap,
                    version,
                    build_guid: build_guid,
                };

                // apply to hash maps...

                by_key.insert(key.clone(), reduced);

                // if the key exists, update it if our version is newer.
                // if the key does not exist, add it.
                latest_by_sap
                    .entry(SAPCode(sap.to_owned()))
                    .and_modify(|current| {
                        if ver_newer(reduced.version, current.version) {
                            *current = reduced;
                        }
                    })
                    .or_insert(reduced);
            }
        }

        let index = ChannelIndex {
            by_key,
            latest_by_sap,
        };

        Self {
            code: channel_code,
            index,
        }
    }
}
