mod desktop;
mod types;
mod utils;

use asar::AsarWriter;
use desktop::gen_dotdesktop;
use types::{EBuilderConfig, PackageJson};
use utils::{gen_copy_list, get_globs_and_file_sets, refilter_copy_list};

use std::fs;
use std::fs::File;
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
        } => {
            let package: PackageJson =
                serde_json::from_str(&fs::read_to_string("package.json").unwrap()).unwrap();

            let ebuilder_conf: EBuilderConfig = if let Some(config_path) = config {
                let config_file = fs::read(&config_path).expect("reading ebuilder config file");
                match config_path.split('.').last().unwrap() {
                    "toml" => toml::from_slice(&config_file).unwrap(),
                    "yaml" | "yml" => serde_yaml::from_slice(&config_file).unwrap(),
                    "json" => serde_json::from_slice(&config_file).unwrap(),
                    "js" => {
                        let out = Command::new(
                            std::env::var("NODE")
                                .map(|s| s.to_string())
                                .unwrap_or_else(|_| "node".to_string()),
                        )
                        .args(["-p", &format!("JSON.stringify(require('{}'))", config_path)])
                        .output()
                        .unwrap()
                        .stdout;
                        serde_json::from_slice(&out).unwrap()
                    }
                    x => panic!("unknown config format '{}'", x),
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

            let current_dir = std::env::current_dir().unwrap();

            let (mut asar_global_globs, asar_file_sets) = get_globs_and_file_sets(files.clone());
            // order matters. add node_modules glob first to allow excluding specific globs in node_modules
            // https://codeberg.org/selfisekai/electron_tasje/issues/14
            asar_global_globs = ["/node_modules/**/*", "!/tasje_out"]
                .into_iter()
                .map(str::to_string)
                .chain(asar_global_globs)
                .chain(STANDARD_FILTERS.into_iter().map(str::to_string))
                .collect();
            let (extra_global_globs, extra_file_sets) = get_globs_and_file_sets(extra_res);

            let asar_copy_list = gen_copy_list(&current_dir, &asar_global_globs, &asar_file_sets);
            let unpacked_copy_list = refilter_copy_list(&asar_copy_list, &asar_unpack);
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
            fs::create_dir_all(&resources_dir).expect("create resources_dir");
            let unpacked_dir = resources_dir.join("app.asar.unpacked");

            // write files into the asar
            let mut asar = AsarWriter::new();
            for (copy_source, copy_target) in &asar_copy_list {
                asar.write_file(
                    copy_target,
                    fs::read(copy_source).expect("reading source file"),
                    true,
                )
                .unwrap();
            }
            asar.finalize(File::create(resources_dir.join("app.asar")).unwrap())
                .unwrap();

            // copy unpacked asar resources
            for (copy_source, copy_target) in &unpacked_copy_list {
                let target = unpacked_dir.join(copy_target.strip_prefix("/").unwrap());
                fs::create_dir_all(target.parent().unwrap())
                    .expect("creating unpacked dir structure");
                fs::copy(copy_source, target).expect("copying unpacked file");
            }

            // copy extra resources
            for (copy_source, copy_target) in &extra_copy_list {
                let target = resources_dir.join(copy_target.strip_prefix("/").unwrap());
                fs::create_dir_all(target.parent().unwrap())
                    .expect("creating extra resource dir structure");
                fs::copy(copy_source, target).expect("copying extra resource file");
            }

            // create a .desktop file
            let (dotdesktop_filename, dotdesktop_content) =
                gen_dotdesktop(&ebuilder_conf, &package);
            fs::write(output_dir.join(dotdesktop_filename), dotdesktop_content)
                .expect("writing generated .desktop file");
        }
    }
}
