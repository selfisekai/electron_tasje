use anyhow::Result;
use asar::AsarWriter;
use once_cell::sync::Lazy;
use std::fs::{self, read, File};
use std::path::{Path, PathBuf};

use crate::app::App;
use crate::config::CopyDef;
use crate::environment::{Environment, HOST_ENVIRONMENT};
use crate::walker::Walker;

static ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/"));

#[derive(Clone, Debug)]
pub struct PackingProcessBuilder {
    app: App,
    base_output_dir: Option<PathBuf>,
    resources_output_dir: Option<PathBuf>,
    target_environment: Option<Environment>,
}

impl PackingProcessBuilder {
    pub fn new(app: App) -> Self {
        PackingProcessBuilder {
            app,
            base_output_dir: None,
            resources_output_dir: None,
            target_environment: None,
        }
    }

    pub fn base_output_dir<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.base_output_dir = Some(path.into());
        self
    }

    pub fn target_environment(mut self, env: Environment) -> Self {
        self.target_environment = Some(env);
        self
    }

    pub fn build<'a>(self) -> PackingProcess {
        let base_output_dir = self.app.root.clone().join(
            self.base_output_dir
                .clone()
                .unwrap_or_else(|| "tasje_out".into()),
        );
        let resources_output_dir = base_output_dir.join(
            self.resources_output_dir
                .unwrap_or_else(|| "resources".into()),
        );
        PackingProcess {
            app: self.app,
            base_output_dir,
            resources_output_dir,
            environment: self
                .target_environment
                .unwrap_or(HOST_ENVIRONMENT),
        }
    }
}

pub struct PackingProcess {
    pub app: App,
    base_output_dir: PathBuf,
    resources_output_dir: PathBuf,
    environment: Environment,
}

impl PackingProcess {
    pub fn proceed(self) -> Result<()> {
        fs::create_dir_all(&self.resources_output_dir)?;

        self.pack_asar()?;
        self.pack_extra(self.app.config().extra_files(), &self.base_output_dir)?;
        self.pack_extra(
            self.app.config().extra_resources(),
            &self.resources_output_dir,
        )?;

        Ok(())
    }

    fn pack_asar(&self) -> Result<()> {
        let mut asar = AsarWriter::new();
        let asar_file = File::create(self.resources_output_dir.join("app.asar"))?;
        let unpack_dir = self
            .resources_output_dir
            .join("app.asar.unpacked");
        for (source, dest, unpack) in Walker::new(
            self.app.root.clone(),
            self.environment,
            self.app.config().files(),
            Some(self.app.config().asar_unpack()).filter(|a| !a.is_empty()),
        )? {
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
}
