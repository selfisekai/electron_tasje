use crate::app::App;
use crate::config::CopyDef;
use crate::desktop::DesktopGenerator;
use crate::environment::{Environment, Platform, HOST_ENVIRONMENT};
use crate::icons::IconGenerator;
use crate::walker::Walker;
use anyhow::Result;
use asar::AsarWriter;
use once_cell::sync::Lazy;
use std::fs::{self, read, File};
use std::path::{Path, PathBuf};

static ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/"));

#[derive(Clone, Debug)]
pub struct PackingProcessBuilder {
    app: App,
    base_output_dir: Option<PathBuf>,
    icons_output_dir: Option<PathBuf>,
    resources_output_dir: Option<PathBuf>,
    target_environment: Option<Environment>,
    additional_files: Vec<CopyDef>,
    additional_asar_unpack: Vec<String>,
    additional_extra_resources: Vec<CopyDef>,
}

impl PackingProcessBuilder {
    pub fn new(app: App) -> Self {
        PackingProcessBuilder {
            app,
            base_output_dir: None,
            icons_output_dir: None,
            resources_output_dir: None,
            target_environment: None,
            additional_files: Vec::new(),
            additional_asar_unpack: Vec::new(),
            additional_extra_resources: Vec::new(),
        }
    }

    pub fn base_output_dir<P>(mut self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.base_output_dir = Some(self.app.root.join(path.as_ref()));
        self
    }

    pub fn target_environment(mut self, env: Environment) -> Self {
        self.target_environment = Some(env);
        self
    }

    pub fn additional_files(mut self, add: Vec<CopyDef>) -> Self {
        self.additional_files.extend(add);
        self
    }

    pub fn additional_asar_unpack(mut self, add: Vec<String>) -> Self {
        self.additional_asar_unpack.extend(add);
        self
    }

    pub fn additional_extra_resources(mut self, add: Vec<CopyDef>) -> Self {
        self.additional_extra_resources.extend(add);
        self
    }

    pub fn build<'a>(self) -> PackingProcess {
        let base_output_dir = self.app.root.clone().join(
            self.base_output_dir
                .clone()
                .or_else(|| self.app.config().output_dir().map(|o| o.into()))
                .unwrap_or_else(|| "tasje_out".into()),
        );
        let icons_output_dir = base_output_dir.join(
            self.icons_output_dir
                .unwrap_or_else(|| "icons".into()),
        );
        let resources_output_dir = base_output_dir.join(
            self.resources_output_dir
                .unwrap_or_else(|| "resources".into()),
        );
        PackingProcess {
            app: self.app,
            base_output_dir,
            icons_output_dir,
            resources_output_dir,
            environment: self
                .target_environment
                .unwrap_or(HOST_ENVIRONMENT),
            additional_files: self.additional_files,
            additional_asar_unpack: self.additional_asar_unpack,
            additional_extra_resources: self.additional_extra_resources,
        }
    }
}

pub struct PackingProcess {
    pub app: App,
    base_output_dir: PathBuf,
    icons_output_dir: PathBuf,
    resources_output_dir: PathBuf,
    environment: Environment,
    additional_files: Vec<CopyDef>,
    additional_asar_unpack: Vec<String>,
    additional_extra_resources: Vec<CopyDef>,
}

impl PackingProcess {
    pub fn proceed(self) -> Result<()> {
        fs::create_dir_all(&self.resources_output_dir)?;
        fs::create_dir_all(&self.icons_output_dir)?;

        self.pack_asar()?;
        self.pack_extra(self.app.config().extra_files(), &self.base_output_dir)?;
        self.pack_extra(
            self.app.config().extra_resources(),
            &self.resources_output_dir,
        )?;

        self.generate_desktop_file()?;
        self.generate_icons()?;

        Ok(())
    }

    fn pack_asar(&self) -> Result<()> {
        let mut asar = AsarWriter::new();
        let asar_file = File::create(self.resources_output_dir.join("app.asar"))?;
        let unpack_dir = self
            .resources_output_dir
            .join("app.asar.unpacked");
        let mut files = self.app.config().files();
        files.extend(self.additional_files.as_slice());
        let unpack_list = Some(
            self.app
                .config()
                .asar_unpack()
                .into_iter()
                .chain(
                    self.additional_asar_unpack
                        .iter()
                        .map(String::as_str),
                )
                .collect::<Vec<_>>(),
        )
        .filter(|l| !l.is_empty());
        for (source, dest, unpack) in
            Walker::new(self.app.root.clone(), self.environment, files, unpack_list)?
        {
            asar.write_file(ROOT.join(&dest), read(&source)?, true)?;
            if unpack {
                let unpack_dest = unpack_dir.join(dest);
                fs::create_dir_all(unpack_dest.parent().unwrap())?;
                fs::copy(&source, &unpack_dest)?;
            }
        }
        asar.finalize(asar_file)?;

        Ok(())
    }

    fn pack_extra<P>(&self, copydefs: Vec<&CopyDef>, target: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let copydefs = copydefs
            .into_iter()
            .chain(self.additional_extra_resources.iter().by_ref())
            .collect::<Vec<_>>();
        if copydefs.is_empty() {
            // nothing to copy, don't bother looking
            return Ok(());
        }
        let target = target.as_ref();
        for (source, dest, _) in
            Walker::new(self.app.root.clone(), self.environment, copydefs, None)?
        {
            let unpack_dest = target.join(dest);
            fs::create_dir_all(unpack_dest.parent().unwrap())?;
            fs::copy(&source, &unpack_dest)?;
        }

        Ok(())
    }

    fn generate_desktop_file(&self) -> Result<()> {
        if self.environment.platform == Platform::Linux {
            fs::write(
                self.base_output_dir
                    .join(self.app.desktop_name()?),
                DesktopGenerator::new().generate(&self.app)?,
            )?;
        }

        Ok(())
    }

    fn generate_icons(&self) -> Result<()> {
        IconGenerator::new().generate(self.app.icon_locations(), &self.icons_output_dir)
    }
}
