use anyhow::Result;
use clap::{Parser, Subcommand};
use electron_tasje::app::App;
use electron_tasje::config::CopyDef;
use electron_tasje::desktop::DesktopGenerator;
use electron_tasje::environment::HOST_PLATFORM;
use electron_tasje::pack::PackingProcessBuilder;
use std::env::current_dir;

#[derive(Subcommand, Debug)]
#[clap(author, version, about, long_about = None)]
enum Command {
    /// pack the resources
    Pack {
        #[clap(short, long, value_parser)]
        /// directory to put build in, overrides directories.output
        output: Option<String>,

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
    /// generate the desktop entry file (this is done as part of "tasje pack", too)
    GenerateDesktop {
        #[clap(short, long, value_parser)]
        /// file or directory to put the generated entry in
        output: Option<String>,
    },
}

use Command::*;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,

    #[clap(short, long, value_parser)]
    /// configuration file, if ebuilder configuration is outside package.json.
    /// can be YAML, TOML, JSON or JS
    config: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let Args { config, .. } = args;

    let root = current_dir()?;
    let package_path = root.join("package.json");
    let app = if let Some(config_path) = &config {
        App::new_from_files(&package_path, root.join(config_path))?
    } else {
        App::new_from_package_file(&package_path)?
    };

    match args.command {
        Pack {
            output,
            additional_files,
            additional_asar_unpack,
            additional_extra_resources,
        } => {
            let mut builder = PackingProcessBuilder::new(app);
            if let Some(out) = output {
                builder = builder.base_output_dir(out);
            }
            builder
                .additional_files(
                    additional_files
                        .into_iter()
                        .map(CopyDef::Simple)
                        .collect(),
                )
                .additional_asar_unpack(additional_asar_unpack)
                .additional_extra_resources(
                    additional_extra_resources
                        .into_iter()
                        .map(CopyDef::Simple)
                        .collect(),
                )
                .build()
                .proceed()?;
        }

        GenerateDesktop { output } => {
            DesktopGenerator::new().write_to_output_dir(&app, HOST_PLATFORM, output)?;
        }
    }

    Ok(())
}
