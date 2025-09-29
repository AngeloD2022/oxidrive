use crate::hyperdrive::remote::models::Channel;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SAPCode(String);

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
    by_key: HashMap<(SAPCode, Version), ProductReduced<'a>>,
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
    pub fn from_channel(channel: &'a Channel) -> Self {
        let channel_code = channel.name.to_owned();

        // For channel index:
        let mut by_key = HashMap::new();
        let mut latest_by_sap: HashMap<SAPCode, ProductReduced> = HashMap::new();

        for product in &channel.products.product {
            let key = (
                SAPCode(product.id.to_owned()),
                Version(product.version.to_owned()),
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
