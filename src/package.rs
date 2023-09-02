use crate::config::{CommonOverridableProperties, EBuilderConfig};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    #[serde(flatten)]
    pub common: CommonOverridableProperties,
    pub build: Option<EBuilderConfig>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub value: Value,
    pub manifest: PackageManifest,
}

impl TryFrom<Value> for Package {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let manifest = serde_json::from_value(value.clone())?;
        Ok(Package { value, manifest })
    }
}
