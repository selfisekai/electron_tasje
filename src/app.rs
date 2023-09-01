use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::config::EBuilderConfig;
use crate::package::Package;
use crate::utils::filesafe_package_name;

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

    pub fn new_from_package_file<P: AsRef<Path>>(package_file: P) -> Result<App> {
        let package_file = package_file.as_ref();
        let package = Package::try_from(serde_json::from_str::<Value>(&fs::read_to_string(
            package_file,
        )?)?)?;
        let config = serde_json::from_value(
            package
                .value
                .get("build")
                .ok_or_else(|| anyhow!("no build config in package"))?
                .clone(),
        )?;
        Ok(App {
            package,
            config,
            root: package_file.parent().unwrap().to_path_buf(),
        })
    }

    pub fn new_from_files<P1, P2>(package_file: P1, config_file: P2) -> Result<App>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let package_file = package_file.as_ref();
        let package = Package::try_from(serde_json::from_str::<Value>(&fs::read_to_string(
            package_file,
        )?)?)?;
        // TODO: handle all the other formats
        let config = serde_json::from_str(&fs::read_to_string(config_file.as_ref())?)?;
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
    use anyhow::Result;

    use super::App;

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
