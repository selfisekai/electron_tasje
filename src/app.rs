use crate::config::EBuilderConfig;
use crate::package::Package;
use crate::utils::filesafe_package_name;
use anyhow::Result;
use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppParseError {
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
    #[error(transparent)]
    Json5Error(#[from] json5::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("package.json holds no ebuilder config under `build` key. reading electron-builder.yml as fallback failed too: {0}")]
    ConfigFallbackError(std::io::Error),
    #[error("no file extension in provided config path")]
    NoConfigFileExtension,
    #[error("unknown file extension in config path: {0:?}")]
    UnknownConfigFileExtension(String),
    #[error("node process for executing config exited unsuccessfully")]
    NodeProcessError { status_code: Option<i32> },
}

#[derive(Debug, Clone)]
pub struct App {
    package: Package,
    config: EBuilderConfig,
    pub root: PathBuf,
}

impl App {
    pub fn new(package: Package, config: EBuilderConfig, root: PathBuf) -> App {
        App {
            package,
            config,
            root,
        }
    }

    /// also looks for electron-builder.yml if there is no "build" in package.json
    pub fn new_from_package_file<P: AsRef<Path>>(package_file: P) -> Result<App, AppParseError> {
        let package_file = package_file.as_ref();
        let package = Package::try_from(serde_json::from_str::<Value>(&fs::read_to_string(
            package_file,
        )?)?)?;
        let root = package_file.parent().unwrap();
        let config = package
            .value
            .get("build")
            .filter(|b| b.is_object())
            .map(|b| -> Result<EBuilderConfig, AppParseError> {
                Ok(serde_json::from_value(b.clone())?)
            })
            .unwrap_or_else(|| -> Result<EBuilderConfig, AppParseError> {
                Ok(serde_yaml::from_reader(
                    fs::File::open(root.join("electron-builder.yml"))
                        .map_err(AppParseError::ConfigFallbackError)?,
                )?)
            })?;
        Ok(App {
            package,
            config,
            root: root.to_path_buf(),
        })
    }

    pub fn new_from_files<P1, P2>(package_file: P1, config_file: P2) -> Result<App, AppParseError>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let package_file = package_file.as_ref();
        let package = Package::try_from(serde_json::from_str::<Value>(&fs::read_to_string(
            package_file,
        )?)?)?;
        let config = match config_file
            .as_ref()
            .extension()
            .map(OsStr::to_str)
            .flatten()
            .ok_or(AppParseError::NoConfigFileExtension)?
        {
            "json" => serde_json::from_str(&fs::read_to_string(config_file.as_ref())?)?,
            "yaml" | "yml" => serde_yaml::from_str(&fs::read_to_string(config_file.as_ref())?)?,
            "toml" => toml::from_str(&fs::read_to_string(config_file.as_ref())?)?,
            "json5" => json5::from_str(&fs::read_to_string(config_file.as_ref())?)?,
            // runs node.js to import the file and serialize it to json, then parses the json output
            "js" => serde_json::from_slice(
                &Command::new(std::env::var("NODE").unwrap_or_else(|_| "node".to_string()))
                    .arg("-p")
                    .arg(format!(
                        "JSON.stringify(require({}))",
                        serde_json::to_string(&config_file.as_ref().canonicalize()?)?
                    ))
                    // to allow using electron binaries
                    .env("ELECTRON_RUN_AS_NODE", "1")
                    .output()
                    .map(|out| {
                        if out.status.code().is_some_and(|c| c == 0) {
                            Ok(out)
                        } else {
                            Err(AppParseError::NodeProcessError {
                                status_code: out.status.code(),
                            })
                        }
                    })??
                    .stdout,
            )?,
            unknown => {
                return Err(AppParseError::UnknownConfigFileExtension(
                    unknown.to_string(),
                ))
            }
        };
        Ok(App {
            package,
            config,
            root: package_file.parent().unwrap().to_path_buf(),
        })
    }

    pub fn config<'a>(&'a self) -> &'a EBuilderConfig {
        &self.config
    }
}

macro_rules! common_property {
    ($self:ident, $prop:ident) => {
        $self
            .config
            .current_platform()
            .common
            .$prop
            .as_ref()
            .or($self.config.base.common.$prop.as_ref())
            .or($self.package.manifest.common.$prop.as_ref())
    };
}

impl<'a> App {
    pub fn description(&'a self) -> Option<&'a str> {
        common_property!(self, description).map(String::as_str)
    }

    pub fn executable_name(&'a self) -> Result<String> {
        filesafe_package_name(
            common_property!(self, executable_name).unwrap_or(&self.package.manifest.name),
        )
    }

    pub fn product_name(&'a self) -> &'a str {
        common_property!(self, product_name)
            .unwrap_or(&self.package.manifest.name)
            .as_str()
    }

    pub fn desktop_name(&'a self) -> Result<String> {
        common_property!(self, desktop_name)
            .map(String::clone)
            .map(Result::Ok)
            .unwrap_or_else(|| {
                Ok(format!(
                    "{}.desktop",
                    filesafe_package_name(&self.package.manifest.name)?
                ))
            })
    }

    pub(crate) fn icon_locations(&'a self) -> Vec<PathBuf> {
        self.config
            .icon_locations()
            .into_iter()
            .map(|p| self.root.join(p))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::App;
    use anyhow::Result;

    #[test]
    fn test_parse() -> Result<()> {
        let app = App::new_from_package_file("test_assets/package.json")?;

        println!("{:#?}", app);

        assert_eq!(app.description(), Some("Packs Electron apps"));
        assert_eq!(app.executable_name()?, "tasje");
        assert_eq!(app.product_name(), "Tasje");
        assert_eq!(app.desktop_name()?, "electron_tasje.desktop");

        Ok(())
    }
}
