#[macro_use]
extern crate lazy_static;

mod desktop;
mod icons;
mod types;
mod utils;

use anyhow::Context;
use asar::AsarWriter;
use desktop::gen_dotdesktop;
use icons::gen_icons;
use path_absolutize::Absolutize;
use types::{EBuilderConfig, PackageJson};
use utils::{fill_variable_template, gen_copy_list, get_globs_and_file_sets, refilter_copy_list};

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::FileSet;

use clap::Parser;

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

const STANDARD_FILTERS: [&str; 15] = [
    "!**/node_modules/.bin",
    "!**/*.{md,rst,markdown}",
    "!**/{__tests__,powered-test,spec,example,examples,readme,README,Readme,changelog,CHANGELOG,Changelog,ChangeLog}",
    "!**/*.{spec,test}.*",
    "!**/._*",
    "!**/{.editorconfig,.DS_Store,.git,.svn,.hg,CVS,RCS,.gitattributes,.nvmrc,.nycrc,Makefile,CMakeLists.txt}",
    "!**/{__pycache__,thumbs.db,.flowconfig,.idea,.vs,.vscode,.nyc_output,.docker-compose.yml}",
    "!**/{.github,.gitlab,.gitlab-ci.yml,appveyor.yml,.travis.yml,circle.yml,.woodpecker.yml}",
    "!**/{package-lock.json,yarn.lock}",
    "!**/.{git,eslint,tslint,prettier,docker,npm,yarn}ignore",
    "!**/.{prettier,eslint,jshint,jsdoc}rc",
    "!**/{.prettierrc,webpack.config,.jshintrc,jsdoc,.eslintrc,tsconfig}{,.json,.js,.yml,yaml}",
    "!**/{yarn,npm}-{debug,error}{,.log,.json}",
    "!**/.{yarn,npm}-{metadata,integrity}",
    "!**/*.{iml,o,hprof,orig,pyc,pyo,rbc,swp,csproj,sln,xproj,c,h,cc,cpp,hpp,lzz,gyp,d.ts}",
];

fn main() {
    let args = Args::parse();

    match args {
        Args::Pack {
            verbose,
            output,
            config,
            additional_files,
            additional_asar_unpack,
            additional_extra_resources,
        } => {
            let current_dir = std::env::current_dir().unwrap();
            let package: PackageJson =
                serde_json::from_str(&fs::read_to_string("package.json").unwrap()).unwrap();

            let ebuilder_conf: EBuilderConfig = if let Some(config_path_) = config {
                let config_path = PathBuf::from(config_path_);
                let config_file = fs::read(&config_path)
                    .with_context(|| format!("on reading file: {:?}", config_path))
                    .unwrap();
                match config_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split('.')
                    .last()
                    .unwrap()
                {
                    "toml" => toml::from_str(std::str::from_utf8(&config_file).unwrap())
                        .with_context(|| format!("parsing toml config file: {:?}", config_path))
                        .unwrap(),
                    "yaml" | "yml" => serde_yaml::from_slice(&config_file)
                        .with_context(|| format!("parsing yaml config file: {:?}", config_path))
                        .unwrap(),
                    "json" => serde_json::from_slice(&config_file)
                        .with_context(|| format!("parsing json config file: {:?}", config_path))
                        .unwrap(),
                    "js" => {
                        let out = Command::new(
                            std::env::var("NODE")
                                .map(|s| s.to_string())
                                .unwrap_or_else(|_| "node".to_string()),
                        )
                        .args([
                            "-p",
                            // using absolute path to make sure it's recognized as path by node.js
                            // https://codeberg.org/selfisekai/electron_tasje/issues/7
                            &format!(
                                "JSON.stringify(require({}))",
                                serde_json::to_string(
                                    &Path::new(&config_path)
                                        .absolutize_from(&current_dir)
                                        .unwrap()
                                )
                                .unwrap()
                            ),
                        ])
                        .output()
                        .unwrap()
                        .stdout;
                        serde_json::from_slice(&out)
                            .with_context(|| format!("parsing js config file: {:?}", config_path))
                            .unwrap()
                    }
                    x => panic!("unknown config format '{}' (file: {:?})", x, config_path),
                }
            } else {
                package.build.clone().expect("no ebuilder config found, either specify one with --config or add it to package.json")
            };

            let files: Vec<FileSet> = ebuilder_conf.files.clone().unwrap_or_default().into();
            let asar_unpack: Vec<String> =
                ebuilder_conf.asar_unpack.clone().unwrap_or_default().into();
            let extra_res: Vec<FileSet> = ebuilder_conf
                .extra_resources
                .clone()
                .unwrap_or_default()
                .into();

            if verbose {
                eprintln!("files: {:#?}", &files);
                eprintln!("asar_unpack: {:#?}", &asar_unpack);
                eprintln!("extra_resources: {:#?}", &extra_res);
            }

            let (mut asar_global_globs, asar_file_sets) = get_globs_and_file_sets(files.clone());
            // order matters. add node_modules glob first to allow excluding specific globs in node_modules
            // https://codeberg.org/selfisekai/electron_tasje/issues/14
            asar_global_globs = ["/node_modules/**/*", "!/tasje_out"]
                .into_iter()
                .map(str::to_string)
                .chain(asar_global_globs)
                .chain(STANDARD_FILTERS.into_iter().map(str::to_string))
                .chain(additional_files)
                .map(fill_variable_template)
                .collect();
            let (mut extra_global_globs, extra_file_sets) = get_globs_and_file_sets(extra_res);
            extra_global_globs = extra_global_globs
                .into_iter()
                .chain(additional_extra_resources)
                .map(fill_variable_template)
                .collect();

            let asar_copy_list = gen_copy_list(&current_dir, &asar_global_globs, &asar_file_sets);
            let unpacked_copy_list = refilter_copy_list(
                &asar_copy_list,
                &asar_unpack
                    .into_iter()
                    .chain(additional_asar_unpack)
                    .map(fill_variable_template)
                    .collect::<Vec<String>>(),
            );
            let extra_copy_list =
                gen_copy_list(&current_dir, &extra_global_globs, &extra_file_sets);

            if verbose {
                eprintln!("asar_copy_list: {:#?}", asar_copy_list);
                eprintln!("unpacked_copy_list: {:#?}", unpacked_copy_list);
                eprintln!("extra_copy_list: {:#?}", extra_copy_list);
            }

            let output_dir = current_dir.join(output.unwrap_or_else(|| {
                ebuilder_conf
                    .directories
                    .clone()
                    .unwrap_or_default()
                    .output
                    .unwrap_or_else(|| "tasje_out".to_string())
            }));
            let resources_dir = output_dir.join("resources");
            fs::create_dir_all(&resources_dir)
                .with_context(|| format!("on creating resources directory: {:?}", resources_dir))
                .unwrap();
            let unpacked_dir = resources_dir.join("app.asar.unpacked");
            let icons_dir = output_dir.join("icons");
            fs::create_dir_all(&icons_dir)
                .with_context(|| format!("on creating icons directory: {:?}", icons_dir))
                .unwrap();

            // write files into the asar
            let mut asar = AsarWriter::new();
            for (copy_source, copy_target) in &asar_copy_list {
                asar.write_file(
                    copy_target,
                    fs::read(copy_source)
                        .with_context(|| format!("on reading file: {:?}", copy_source))
                        .unwrap(),
                    true,
                )
                .with_context(|| format!("on writing asar file: {:?}", copy_target))
                .unwrap();
            }
            let asar_path = resources_dir.join("app.asar");
            asar.finalize(
                File::create(&asar_path)
                    .with_context(|| format!("on creating final asar file: {:?}", asar_path))
                    .unwrap(),
            )
            .with_context(|| format!("on creating final asar: {:?}", asar_path))
            .unwrap();

            // copy unpacked asar resources
            for (copy_source, copy_target) in &unpacked_copy_list {
                let target = unpacked_dir.join(copy_target.strip_prefix("/").unwrap());
                let target_parent = target.parent().unwrap();
                fs::create_dir_all(target_parent)
                    .with_context(|| format!("on creating unpacked asar dir: {:?}", target_parent))
                    .unwrap();
                fs::copy(copy_source, &target)
                    .with_context(|| format!("on copying unpacked asar file: {:?}", target))
                    .unwrap();
            }

            // copy extra resources
            for (copy_source, copy_target) in &extra_copy_list {
                let target = resources_dir.join(copy_target.strip_prefix("/").unwrap());
                let target_parent = target.parent().unwrap();
                fs::create_dir_all(target_parent)
                    .with_context(|| format!("on creating extra resource dir: {:?}", target_parent))
                    .unwrap();
                fs::copy(copy_source, &target)
                    .with_context(|| format!("on writing unpacked asar file: {:?}", target))
                    .unwrap();
            }

            // create a .desktop file
            let (dotdesktop_filename, dotdesktop_content) =
                gen_dotdesktop(&ebuilder_conf, &package);
            let dotdesktop_location = output_dir.join(dotdesktop_filename);
            fs::write(&dotdesktop_location, dotdesktop_content)
                .with_context(|| {
                    format!(
                        "on writing generated .desktop file: {:?}",
                        dotdesktop_location
                    )
                })
                .unwrap();

            // copy/generate icons
            gen_icons(&ebuilder_conf, current_dir, icons_dir);
        }
    }
}
