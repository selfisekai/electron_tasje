#![feature(is_some_with)]

mod types;
mod utils;

use asar::AsarWriter;
use types::PackageJson;
use utils::{gen_copy_list, get_globs_and_file_sets, refilter_copy_list};

use std::fs;
use std::fs::File;

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
    },
}

const STANDARD_FILTERS: [&str; 16] = [
    "!**/node_modules/.bin",
    "!**/*.{md,rst,markdown,txt}",
    "!**/{test,tests,__tests__,powered-test,example,examples,readme,README,Readme,changelog,CHANGELOG,Changelog}",
    "!**/test.*",
    "!**/*.test.*",
    "!**/._*",
    "!**/{.editorconfig,.DS_Store,.git,.svn,.hg,CVS,RCS,.gitattributes,.nvmrc,.nycrc,Makefile}",
    "!**/{__pycache__,thumbs.db,.flowconfig,.idea,.vs,.vscode,.nyc_output,.docker-compose.yml}",
    "!**/{.github,.gitlab,.gitlab-ci.yml,appveyor.yml,.travis.yml,circle.yml,.woodpecker.yml}",
    "!**/{package-lock.json,yarn.lock}",
    "!**/.{git,eslint,tslint,prettier,docker,npm,yarn}ignore",
    "!**/.{prettier,eslint,jshint,jsdoc}rc",
    "!**/{.prettierrc,webpack.config,.jshintrc,jsdoc,.eslintrc}{,.json,.js,.yml,yaml}",
    "!**/{yarn,npm}-{debug,error}{,.log,.json}",
    "!**/.{yarn,npm}-metadata,integrity",
    "!**/*.{iml,o,hprof,orig,pyc,pyo,rbc,swp,csproj,sln,xproj,c,h,cc,cpp,hpp,lzz,gyp,ts}",
];

fn main() {
    let args = Args::parse();

    match args {
        Args::Pack { verbose, output } => {
            let package: PackageJson =
                serde_json::from_str(&fs::read_to_string("package.json").unwrap()).unwrap();

            let ebuilder_conf = &package.build.clone().unwrap_or_else(|| {
                todo!("reading ebuilder config outside package.json");
            });

            let files: Vec<FileSet> = ebuilder_conf.files.as_ref().unwrap().into();
            let asar_unpack: Vec<String> = ebuilder_conf.asar_unpack.as_ref().unwrap().into();
            let extra_res: Vec<FileSet> = ebuilder_conf.extra_resources.as_ref().unwrap().into();

            if verbose {
                eprintln!("files: {:#?}", &files);
                eprintln!("asar_unpack: {:#?}", &asar_unpack);
                eprintln!("extra_resources: {:#?}", &extra_res);
            }

            let current_dir = std::env::current_dir().unwrap();

            let (mut asar_global_globs, asar_file_sets) = get_globs_and_file_sets(files.clone());
            asar_global_globs = asar_global_globs
                .into_iter()
                .chain(
                    ["/node_modules/**/*", "!/tasje_out"]
                        .into_iter()
                        .chain(STANDARD_FILTERS)
                        .map(str::to_string),
                )
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
            fs::create_dir_all(&output_dir).expect("create output_dir");
            let unpacked_dir = output_dir.join("app.asar.unpacked");

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
            asar.finalize(File::create(output_dir.join("app.asar")).unwrap())
                .unwrap();

            // copy unpacked asar resources
            for (copy_source, copy_target) in &unpacked_copy_list {
                let target = unpacked_dir.join(copy_target);
                fs::create_dir_all(target.parent().unwrap())
                    .expect("creating unpacked dir structure");
                fs::copy(copy_source, target).expect("copying unpacked file");
            }

            // copy extra resources
            for (copy_source, copy_target) in &extra_copy_list {
                let target = output_dir.join(copy_target);
                fs::create_dir_all(target.parent().unwrap())
                    .expect("creating extra resource dir structure");
                fs::copy(copy_source, output_dir.join(copy_target))
                    .expect("copying extra resource file");
            }
        }
    }
}
