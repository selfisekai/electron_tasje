use std::fs;
use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::config::EBuilderConfig;
use crate::package::Package;

#[derive(Debug, Clone)]
pub struct App {
    package: Package,
    config: EBuilderConfig,
}

impl App {
    pub fn new(package: Package, config: EBuilderConfig) -> App {
        App { package, config }
    }

    pub fn new_from_package(package: Package) -> Result<App> {
        let config = serde_json::from_value(
            package
                .value
                .get("build")
                .ok_or_else(|| anyhow!("no build config in package"))?
                .clone(),
        )?;
        Ok(App { package, config })
    }

    pub fn new_from_package_file<P: AsRef<Path>>(package_file: P) -> Result<App> {
        App::new_from_package(Package::try_from(serde_json::from_str::<Value>(
            &fs::read_to_string(package_file.as_ref())?,
        )?)?)
    }

    pub fn new_from_files<P: AsRef<Path>>(package_file: P, config_file: P) -> Result<App> {
        let package = Package::try_from(serde_json::from_str::<Value>(&fs::read_to_string(
            package_file.as_ref(),
        )?)?)?;
        let config = serde_json::from_str(&fs::read_to_string(config_file.as_ref())?)?;
        Ok(App { package, config })
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

impl App {
    pub fn description<'a>(&'a self) -> Option<&'a str> {
        common_property!(self, description).map(String::as_str)
    }

    pub fn executable_name<'a>(&'a self) -> &'a str {
        // TODO: ensure it's filesafe
        common_property!(self, executable_name)
            .unwrap_or(&self.package.manifest.name)
            .as_str()
    }

    pub fn product_name<'a>(&'a self) -> &'a str {
        common_property!(self, product_name)
            .unwrap_or(&self.package.manifest.name)
            .as_str()
    }

    pub fn desktop_name<'a>(&'a self) -> String {
        // TODO: ensure it's filesafe
        common_property!(self, desktop_name)
            .map(String::clone)
            .unwrap_or_else(|| format!("{}.desktop", self.package.manifest.name))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::App;

    #[test]
    fn test_parse() -> Result<()> {
        let app = App::new_from_package_file("src/test_assets/package.json")?;

        println!("{:#?}", app);

        assert_eq!(app.description(), Some("Packs Electron apps"));
        assert_eq!(app.executable_name(), "tasje");
        assert_eq!(app.product_name(), "Tasje");
        assert_eq!(app.desktop_name(), "electron_tasje.desktop");

        Ok(())
    }
}
