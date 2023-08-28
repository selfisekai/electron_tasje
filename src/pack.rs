use anyhow::Result;
use asar::AsarWriter;
use once_cell::sync::Lazy;
use std::fs::{self, read, File};
use std::path::PathBuf;

use crate::app::App;
use crate::walker::Walker;

static ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/"));

#[derive(Clone, Debug)]
pub struct PackingProcessBuilder {
    app: App,
    base_output_dir: Option<PathBuf>,
    resources_output_dir: Option<PathBuf>,
}

impl PackingProcessBuilder {
    pub fn new(app: App) -> Self {
        PackingProcessBuilder {
            app,
            base_output_dir: None,
            resources_output_dir: None,
        }
    }

    pub fn base_output_dir<P>(mut self, path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.base_output_dir = Some(path.into());
        self
    }

    pub fn build(self) -> PackingProcess {
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
        }
    }
}

pub struct PackingProcess {
    pub app: App,
    base_output_dir: PathBuf,
    resources_output_dir: PathBuf,
}

impl PackingProcess {
    pub fn proceed(self) -> Result<()> {
        fs::create_dir(&self.base_output_dir)?;
        fs::create_dir(&self.resources_output_dir)?;

        self.pack_asar()?;

        Ok(())
    }

    fn pack_asar(&self) -> Result<()> {
        let mut asar = AsarWriter::new();
        let asar_file = File::create(self.resources_output_dir.join("app.asar"))?;
        for (source, dest) in Walker::new(self.app.root.clone(), self.app.config().files())? {
            asar.write_file(ROOT.join(dest), read(source)?, true)?;
        }
        asar.finalize(asar_file)?;

        Ok(())
    }
}
