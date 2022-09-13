use std::{collections::HashMap, fmt};

use serde::Deserialize;
use smart_default::SmartDefault;

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// possible value reference: https://nodejs.org/dist/latest-v16.x/docs/api/process.html#processarch
pub enum NodeArch {
    /// x86_64, amd64
    X64,
    /// i686, x86
    IA32,
    /// aarch64, arm64
    Arm64,
    /// arm 32-bit
    Arm,
}

impl fmt::Display for NodeArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NodeArch::X64 => "x64",
                NodeArch::IA32 => "ia32",
                NodeArch::Arm64 => "arm64",
                NodeArch::Arm => "arm",
            }
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    pub name: String,
    pub product_name: Option<String>,
    pub description: Option<String>,
    pub version: String,
    pub build: Option<EBuilderConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrMultiple {
    One(String),
    Multiple(Vec<String>),
}

impl From<&StringOrMultiple> for Vec<String> {
    fn from(som: &StringOrMultiple) -> Self {
        match som {
            StringOrMultiple::One(s) => vec![s.clone()],
            StringOrMultiple::Multiple(v) => v.clone(),
        }
    }
}

impl From<StringOrMultiple> for Vec<String> {
    fn from(som: StringOrMultiple) -> Self {
        Vec::<String>::from(&som)
    }
}

impl Default for StringOrMultiple {
    fn default() -> Self {
        StringOrMultiple::Multiple(vec![])
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum AnyCopyDefs {
    One(CopyDef),
    Multiple(Vec<CopyDef>),
}

impl From<&AnyCopyDefs> for Vec<CopyDef> {
    fn from(acd: &AnyCopyDefs) -> Self {
        match acd {
            AnyCopyDefs::One(s) => vec![s.clone()],
            AnyCopyDefs::Multiple(v) => v.clone(),
        }
    }
}

impl From<AnyCopyDefs> for Vec<CopyDef> {
    fn from(acd: AnyCopyDefs) -> Self {
        Vec::<CopyDef>::from(&acd)
    }
}

impl From<&AnyCopyDefs> for Vec<FileSet> {
    fn from(acd: &AnyCopyDefs) -> Self {
        Vec::<CopyDef>::from(acd)
            .into_iter()
            .map(|cd| cd.into())
            .collect()
    }
}

impl From<AnyCopyDefs> for Vec<FileSet> {
    fn from(acd: AnyCopyDefs) -> Self {
        Vec::<FileSet>::from(&acd)
    }
}

impl Default for AnyCopyDefs {
    fn default() -> Self {
        AnyCopyDefs::Multiple(vec![])
    }
}

#[derive(Debug, Clone, Deserialize, SmartDefault)]
#[serde(rename_all = "camelCase")]
pub struct FileSet {
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub filter: Option<StringOrMultiple>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CopyDef {
    Set(FileSet),
    Simple(String),
}

impl From<CopyDef> for FileSet {
    fn from(cd: CopyDef) -> Self {
        match cd {
            CopyDef::Simple(s) => FileSet {
                from: s,
                to: None,
                ..Default::default()
            },
            CopyDef::Set(s) => s,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EBDirectories {
    pub output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EBProtocol {
    pub name: Option<String>,
    pub schemes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum EBProtocolOrPlural {
    Single(EBProtocol),
    Multiple(Vec<EBProtocol>),
}

impl From<&EBProtocolOrPlural> for Vec<EBProtocol> {
    fn from(maybe_plural: &EBProtocolOrPlural) -> Self {
        match maybe_plural {
            EBProtocolOrPlural::Single(s) => vec![s.clone()],
            EBProtocolOrPlural::Multiple(m) => m.clone(),
        }
    }
}

impl From<EBProtocolOrPlural> for Vec<EBProtocol> {
    fn from(maybe_plural: EBProtocolOrPlural) -> Self {
        Vec::<EBProtocol>::from(&maybe_plural)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EBFileAssoc {
    pub ext: StringOrMultiple,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum EBFileAssocOrPlural {
    Single(EBFileAssoc),
    Multiple(Vec<EBFileAssoc>),
}

impl From<&EBFileAssocOrPlural> for Vec<EBFileAssoc> {
    fn from(maybe_plural: &EBFileAssocOrPlural) -> Self {
        match maybe_plural {
            EBFileAssocOrPlural::Single(s) => vec![s.clone()],
            EBFileAssocOrPlural::Multiple(m) => m.clone(),
        }
    }
}

impl From<EBFileAssocOrPlural> for Vec<EBFileAssoc> {
    fn from(maybe_plural: EBFileAssocOrPlural) -> Self {
        Vec::<EBFileAssoc>::from(&maybe_plural)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LinuxOptions {
    pub executable_name: Option<String>,
    pub category: Option<String>,
    pub desktop: Option<HashMap<String, String>>,
    pub protocols: Option<EBProtocolOrPlural>,
    pub file_associations: Option<EBFileAssocOrPlural>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowsOptions {
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MacOptions {
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
/// might be a part of package.json or a separate yaml/toml/json/js file
/// https://www.electron.build/configuration/configuration
pub struct EBuilderConfig {
    pub product_name: Option<String>,
    pub copyright: Option<String>,

    pub files: Option<AnyCopyDefs>,
    pub asar_unpack: Option<StringOrMultiple>,
    pub extra_resources: Option<AnyCopyDefs>,

    pub directories: Option<EBDirectories>,

    pub linux: Option<LinuxOptions>,
    pub win: Option<WindowsOptions>,
    pub mac: Option<MacOptions>,
    pub executable_name: Option<String>,
    pub protocols: Option<EBProtocolOrPlural>,
    pub file_associations: Option<EBFileAssocOrPlural>,
}
