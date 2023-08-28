use std::env::current_dir;

use anyhow::Result;
use clap::Parser;

use electron_tasje::{
    app::App,
    pack::{PackingProcess, PackingProcessBuilder},
};

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
            verbose,
            output,
            config,
            additional_files,
            additional_asar_unpack,
            additional_extra_resources,
        } => {
            let root = current_dir()?;
            let app = App::new_from_package_file(root.join("package.json"))?;
            PackingProcessBuilder::new(app).build().proceed()?;
        }
    }

    Ok(())
}
