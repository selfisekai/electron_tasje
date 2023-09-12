use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use smart_default::SmartDefault;
use std::collections::HashMap;

use crate::environment::Platform;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileSet {
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default, deserialize_with = "might_be_single")]
    pub(crate) filter: Vec<String>,
}

impl FileSet {
    pub fn from(&self) -> Option<&str> {
        self.from
            .as_ref()
            .and_then(|f| f.strip_prefix("./"))
            .or(self.from.as_deref())
    }

    pub fn to(&self) -> Option<&str> {
        self.to
            .as_ref()
            .and_then(|to| to.strip_prefix("./"))
            .or(self.to.as_deref())
    }

    pub fn filters(&self) -> &[String] {
        &self.filter
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CopyDef {
    Simple(String),
    Set(FileSet),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EBDirectories {
    pub output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolAssociation {
    pub name: Option<String>,
    pub schemes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAssociation {
    #[serde(deserialize_with = "might_be_single")]
    ext: Vec<String>,
    pub mime_type: Option<String>,
}

impl FileAssociation {
    pub fn exts(&self) -> &[String] {
        &self.ext
    }
}

fn might_be_single<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    let v = MightBeSingle::deserialize(deserializer)?;
    Ok(v.into())
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, SmartDefault)]
#[serde(untagged)]
pub(crate) enum MightBeSingle<T> {
    Multiple(Vec<T>),
    One(T),
    #[default]
    None,
}

impl<T> From<MightBeSingle<T>> for Vec<T> {
    fn from(x: MightBeSingle<T>) -> Vec<T> {
        use MightBeSingle::*;

        match x {
            Multiple(v) => v,
            One(v) => vec![v],
            None => vec![],
        }
    }
}

impl<T> From<Vec<T>> for MightBeSingle<T> {
    fn from(value: Vec<T>) -> Self {
        Self::Multiple(value)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommonOverridableProperties {
    pub description: Option<String>,
    pub executable_name: Option<String>,
    pub product_name: Option<String>,
    pub desktop_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EBuilderBaseConfig {
    #[serde(flatten)]
    pub(crate) common: CommonOverridableProperties,

    #[serde(default, deserialize_with = "might_be_single")]
    files: Vec<CopyDef>,
    #[serde(default, deserialize_with = "might_be_single")]
    asar_unpack: Vec<String>,
    #[serde(default, deserialize_with = "might_be_single")]
    extra_files: Vec<CopyDef>,
    #[serde(default, deserialize_with = "might_be_single")]
    extra_resources: Vec<CopyDef>,

    #[serde(default)]
    directories: EBDirectories,
    icon: Option<String>,

    #[serde(default, deserialize_with = "might_be_single")]
    protocols: Vec<ProtocolAssociation>,
    #[serde(default, deserialize_with = "might_be_single")]
    file_associations: Vec<FileAssociation>,

    #[serde(default)]
    extra_metadata: Option<serde_json::Value>,

    // "linux-specific" section
    #[serde(default, deserialize_with = "might_be_single")]
    category: Vec<String>,
    desktop: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
/// an electron-builder config for an app. might be a part of package.json,
/// or in a separate yaml/toml/json/js file.
/// tries to follow https://www.electron.build/configuration/configuration
pub struct EBuilderConfig {
    #[serde(flatten)]
    pub(crate) base: EBuilderBaseConfig,

    #[serde(default)]
    linux: EBuilderBaseConfig,

    #[serde(default)]
    mac: EBuilderBaseConfig,

    #[serde(default)]
    win: EBuilderBaseConfig,
}

impl<'a> EBuilderConfig {
    #[inline]
    pub(crate) fn current_platform(&'a self, platform: Platform) -> &'a EBuilderBaseConfig {
        use Platform::*;
        match platform {
            Windows => &self.win,
            Linux => &self.linux,
            Darwin => &self.mac,
        }
    }

    pub fn files(&'a self, platform: Platform) -> &'a [CopyDef] {
        let platform_files = &self.current_platform(platform).files;
        if !platform_files.is_empty() {
            platform_files.as_slice()
        } else {
            self.base.files.as_slice()
        }
    }

    pub fn asar_unpack(&'a self, platform: Platform) -> &'a [String] {
        let platform_asar = &self.current_platform(platform).asar_unpack;
        if !platform_asar.is_empty() {
            platform_asar.as_slice()
        } else {
            self.base.asar_unpack.as_slice()
        }
    }

    pub fn extra_files(&'a self, platform: Platform) -> &'a [CopyDef] {
        let platform_extra = &self.current_platform(platform).extra_files;
        if !platform_extra.is_empty() {
            platform_extra.as_slice()
        } else {
            self.base.extra_files.as_slice()
        }
    }

    pub fn extra_resources(&'a self, platform: Platform) -> &'a [CopyDef] {
        let platform_extra = &self.current_platform(platform).extra_resources;
        if !platform_extra.is_empty() {
            platform_extra.as_slice()
        } else {
            self.base.extra_resources.as_slice()
        }
    }

    pub fn extra_metadata(&'a self, platform: Platform) -> Option<&'a serde_json::Value> {
        let platform_extra = &self.current_platform(platform).extra_metadata;
        platform_extra
            .as_ref()
            .or(self.base.extra_metadata.as_ref())
    }

    pub fn desktop_properties(&'a self, platform: Platform) -> Option<Vec<(String, String)>> {
        self.current_platform(platform)
            .desktop
            .as_ref()
            .or(self.base.desktop.as_ref())
            .map(|m| m.clone().into_iter().collect())
    }

    pub fn output_dir(&'a self, platform: Platform) -> Option<&'a str> {
        self.current_platform(platform)
            .directories
            .output
            .as_deref()
            .or(self.base.directories.output.as_deref())
    }

    pub fn protocol_associations(&'a self, platform: Platform) -> &[ProtocolAssociation] {
        let platform_protocols = &self.current_platform(platform).protocols;
        if !platform_protocols.is_empty() {
            platform_protocols.as_slice()
        } else {
            self.base.protocols.as_slice()
        }
    }

    pub fn file_associations(&'a self, platform: Platform) -> &'a [FileAssociation] {
        let platform_assocs = &self.current_platform(platform).file_associations;
        if !platform_assocs.is_empty() {
            platform_assocs.as_slice()
        } else {
            self.base.file_associations.as_slice()
        }
    }

    /// https://specifications.freedesktop.org/menu-spec/latest/apa.html#main-category-registry
    pub fn desktop_categories(&'a self, platform: Platform) -> &[String] {
        &self.current_platform(platform).category
    }

    pub(crate) fn icon_locations(&'a self) -> Vec<&'a str> {
        [
            self.linux.icon.as_deref(),
            self.mac
                .icon
                .as_deref()
                .or(Some("build/icon.icns")),
            self.win
                .icon
                .as_deref()
                .or(Some("build/icon.ico")),
            self.base.icon.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::EBuilderConfig;
    use crate::config::{CopyDef, FileSet};
    use crate::environment::Platform;
    use anyhow::Result;
    use serde_json::json;

    static LINUX: Platform = Platform::Linux;

    #[test]
    fn test_parse_empty() -> Result<()> {
        let bc: EBuilderConfig = serde_json::from_value(json!({
            "files": null,
            "asarUnpack": [],
        }))?;
        assert!(bc.files(LINUX).is_empty());
        assert!(bc.asar_unpack(LINUX).is_empty());
        assert!(bc.extra_resources(LINUX).is_empty());
        Ok(())
    }

    #[test]
    fn test_parse_single() -> Result<()> {
        let bc: EBuilderConfig = serde_json::from_value(json!({
            "files": "file.aoeu",
            "asarUnpack": "*.aoeu",
            "extraResources": {
                "from": "dir",
            },
        }))?;
        assert_eq!(bc.files(LINUX), [CopyDef::Simple("file.aoeu".to_owned())]);
        assert_eq!(bc.asar_unpack(LINUX), ["*.aoeu"]);
        assert_eq!(
            bc.extra_resources(LINUX),
            [CopyDef::Set(FileSet {
                from: Some("dir".to_owned()),
                to: None,
                filter: vec![],
            })]
        );
        Ok(())
    }

    #[test]
    fn test_parse_multiple() -> Result<()> {
        let bc: EBuilderConfig = serde_json::from_value(json!({
            "files": ["file.aoeu", "bestand.aoeu"],
            "asarUnpack": ["*.aoeu", "dir/"],
            "extraResources": [{
                "from": "source",
                "filter": "*",
            }, "dir1", "dir2", {
                "from": "hx",
                "to": "mz",
                "filter": ["**/*", "!foo/*.js"],
            }, {
                "filter": ["LICENSE.txt"],
            }],
        }))?;
        assert_eq!(
            bc.files(LINUX),
            [
                CopyDef::Simple("file.aoeu".to_owned()),
                CopyDef::Simple("bestand.aoeu".to_owned()),
            ],
        );
        assert_eq!(bc.asar_unpack(LINUX), ["*.aoeu", "dir/"]);
        assert_eq!(
            bc.extra_resources(LINUX),
            [
                CopyDef::Set(FileSet {
                    from: Some("source".to_owned()),
                    to: None,
                    filter: vec!["*".to_owned()],
                }),
                CopyDef::Simple("dir1".to_owned()),
                CopyDef::Simple("dir2".to_owned()),
                CopyDef::Set(FileSet {
                    from: Some("hx".to_owned()),
                    to: Some("mz".to_owned()),
                    filter: vec!["**/*".to_owned(), "!foo/*.js".to_owned(),],
                }),
                CopyDef::Set(FileSet {
                    from: None,
                    to: None,
                    filter: vec!["LICENSE.txt".to_owned()],
                }),
            ],
        );
        Ok(())
    }
}
