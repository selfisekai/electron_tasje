use std::env::current_dir;

use anyhow::Result;
use clap::Parser;

use electron_tasje::app::App;
use electron_tasje::pack::PackingProcessBuilder;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
enum Args {
    /// pack the resources
    Pack {
        #[clap(short, long, value_parser)]
        verbose: bool,

        #[clap(short, long, value_parser)]
        /// directory to put build in, overrides directories.output
        output: Option<String>,

        #[clap(short, long, value_parser)]
        /// configuration file, if ebuilder configuration is outside package.json.
        /// can be YAML, TOML, JSON or JS
        config: Option<String>,

        #[clap(long, value_parser)]
        /// additional globs to be interpreted as a part of "files" in ebuilder config
        additional_files: Vec<String>,

        #[clap(long, value_parser)]
        /// additional globs to be interpreted as a part of "asarUnpack" in ebuilder config
        additional_asar_unpack: Vec<String>,

        #[clap(long, value_parser)]
        /// additional globs to be interpreted as a part of "extraResources" in ebuilder config
        additional_extra_resources: Vec<String>,
    },
}

use Args::*;

fn main() -> Result<()> {
    let args = Args::parse();

    println!("{:#?}", args);

    match args {
        Pack {
            verbose: _,
            output: _,
            config,
            additional_files: _,
            additional_asar_unpack: _,
            additional_extra_resources: _,
        } => {
            let root = current_dir()?;
            let package_path = root.join("package.json");
            let app = if let Some(config_path) = &config {
                App::new_from_files(&package_path, root.join(config_path))?
            } else {
                App::new_from_package_file(&package_path)?
            };
            PackingProcessBuilder::new(app)
                .build()
                .proceed()?;
        }
    }

    Ok(())
}
