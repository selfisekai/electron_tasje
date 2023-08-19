use serde::Deserialize;
use smart_default::SmartDefault;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileSet {
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub filter: MightBeSingle<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum CopyDef {
    Simple(String),
    Set(FileSet),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, SmartDefault)]
#[serde(untagged)]
pub enum MightBeSingle<T> {
    Multiple(Vec<T>),
    One(T),
    #[default]
    None,
}

impl<T> MightBeSingle<T> {
    fn is_empty(&self) -> bool {
        match self {
            MightBeSingle::None => true,
            MightBeSingle::One(_) => false,
            MightBeSingle::Multiple(multiple) => multiple.is_empty(),
        }
    }

    fn or<'a>(&'a self, other: &'a MightBeSingle<T>) -> &'a MightBeSingle<T> {
        if self.is_empty() {
            other
        } else {
            self
        }
    }

    fn as_vec<'a>(&'a self) -> Vec<&'a T> {
        match self {
            MightBeSingle::None => Vec::new(),
            MightBeSingle::One(one) => vec![one],
            MightBeSingle::Multiple(multiple) => multiple.iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EBuilderBaseConfig {
    product_name: Option<String>,
    copyright: Option<String>,

    #[serde(default)]
    files: MightBeSingle<CopyDef>,
    #[serde(default)]
    asar_unpack: MightBeSingle<String>,
    #[serde(default)]
    extra_resources: MightBeSingle<CopyDef>,

    // directories: Option<EBDirectories>,
    executable_name: Option<String>,
    // protocols: Option<EBProtocolOrPlural>,
    // file_associations: Option<EBFileAssocOrPlural>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
/// an electron-builder config for an app. might be a part of package.json,
/// or in a separate yaml/toml/json/js file.
/// tries to follow https://www.electron.build/configuration/configuration
pub struct EBuilderConfig {
    #[serde(flatten)]
    base: EBuilderBaseConfig,

    #[serde(default)]
    linux: EBuilderBaseConfig,
}

impl EBuilderConfig {
    #[cfg(target_os = "linux")]
    /// this assumes no cross-compilation is ever done
    fn current_platform(&self) -> &EBuilderBaseConfig {
        &self.linux
    }
}

impl<'a> EBuilderConfig {
    pub fn files(&'a self) -> Vec<&'a CopyDef> {
        self.current_platform().files.or(&self.base.files).as_vec()
    }

    pub fn asar_unpack(&'a self) -> Vec<&'a str> {
        self.current_platform()
            .asar_unpack
            .or(&self.base.asar_unpack)
            .as_vec()
            .into_iter()
            .map(String::as_str)
            .collect()
    }

    pub fn extra_resources(&'a self) -> Vec<&'a CopyDef> {
        self.current_platform()
            .extra_resources
            .or(&self.base.extra_resources)
            .as_vec()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use crate::config::{CopyDef, FileSet, MightBeSingle};

    use super::EBuilderConfig;

    #[test]
    fn test_parse_empty() -> Result<()> {
        let bc: EBuilderConfig = serde_json::from_value(json!({
            "files": null,
        }))?;
        assert!(bc.files().is_empty());
        assert!(bc.asar_unpack().is_empty());
        assert!(bc.extra_resources().is_empty());
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
        assert_eq!(bc.files(), vec![&CopyDef::Simple("file.aoeu".to_owned())]);
        assert_eq!(bc.asar_unpack(), vec!["*.aoeu"]);
        assert_eq!(
            bc.extra_resources(),
            vec![&CopyDef::Set(FileSet {
                from: "dir".to_owned(),
                to: None,
                filter: MightBeSingle::None,
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
            }],
        }))?;
        assert_eq!(
            bc.files(),
            vec![
                &CopyDef::Simple("file.aoeu".to_owned()),
                &CopyDef::Simple("bestand.aoeu".to_owned()),
            ],
        );
        assert_eq!(bc.asar_unpack(), vec!["*.aoeu", "dir/"]);
        assert_eq!(
            bc.extra_resources(),
            vec![
                &CopyDef::Set(FileSet {
                    from: "source".to_owned(),
                    to: None,
                    filter: MightBeSingle::One("*".to_owned()),
                }),
                &CopyDef::Simple("dir1".to_owned()),
                &CopyDef::Simple("dir2".to_owned()),
                &CopyDef::Set(FileSet {
                    from: "hx".to_owned(),
                    to: Some("mz".to_owned()),
                    filter: MightBeSingle::Multiple(vec![
                        "**/*".to_owned(),
                        "!foo/*.js".to_owned()
                    ]),
                })
            ],
        );
        Ok(())
    }
}
