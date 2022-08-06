#![feature(is_some_with)]

mod types;
mod utils;

use asar::AsarWriter;
use types::PackageJson;
use utils::{gen_copy_list, get_globs_and_file_sets};

use std::fs;
use std::fs::File;

use crate::types::FileSet;

use clap::Parser;

/// electron app packager/builder
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    verbose: bool,

    #[clap(short, long, value_parser)]
    /// directory to put build in, overrides directories.output
    output: Option<String>,
}

fn main() {
    let args = Args::parse();

    let package: PackageJson =
        serde_json::from_str(&fs::read_to_string("package.json").unwrap()).unwrap();

    let ebuilder_conf = &package.build.clone().unwrap_or_else(|| {
        todo!("reading ebuilder config outside package.json");
    });

    let files: Vec<FileSet> = ebuilder_conf.files.as_ref().unwrap().into();
    let asar_unpack: Vec<String> = ebuilder_conf.asar_unpack.as_ref().unwrap().into();
    let extra_res: Vec<FileSet> = ebuilder_conf.extra_resources.as_ref().unwrap().into();

    if args.verbose {
        eprintln!("files: {:#?}", &files);
        eprintln!("asar_unpack: {:#?}", &asar_unpack);
        eprintln!("extra_resources: {:#?}", &extra_res);
    }

    let current_dir = std::env::current_dir().unwrap();

    let (global_globs, file_sets) = get_globs_and_file_sets(files);
    let (extra_global_globs, extra_file_sets) = get_globs_and_file_sets(extra_res);

    let copy_list = gen_copy_list(&current_dir, &global_globs, &file_sets);
    let extra_copy_list = gen_copy_list(&current_dir, &extra_global_globs, &extra_file_sets);

    if args.verbose {
        eprintln!("copy_list: {:#?}", copy_list);
        eprintln!("extra_copy_list: {:#?}", extra_copy_list);
    }

    let output_dir = current_dir.join(args.output.unwrap_or_else(|| "tasje_out".to_string()));
    fs::create_dir_all(&output_dir).expect("create output_dir");

    let mut asar = AsarWriter::new();
    for (copy_source, copy_target) in copy_list {
        asar.write_file(
            copy_target,
            fs::read(copy_source).expect("reading source file"),
            true,
        )
        .unwrap();
    }
    asar.finalize(File::create(output_dir.join("app.asar")).unwrap())
        .unwrap();
}
