use serde::Deserialize;
use smart_default::SmartDefault;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
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
/// might be a part of package.json or a separate yaml/toml/json/js file
/// https://www.electron.build/configuration/configuration
pub struct EBuilderConfig {
    pub product_name: Option<String>,
    pub copyright: Option<String>,

    pub files: Option<AnyCopyDefs>,
    pub asar_unpack: Option<StringOrMultiple>,
    pub extra_resources: Option<AnyCopyDefs>,

    pub directories: Option<EBDirectories>,
}
