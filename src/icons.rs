use std::{collections::HashSet, fs, path::Path};

use regex::Regex;

use crate::types::EBuilderConfig;

pub fn gen_icons<P: AsRef<Path>>(ebuilder: &EBuilderConfig, current_dir: P, icons_dir: P) {
    let mut icon_sizes = HashSet::new();
    if let Some(original_icons_dir) = ebuilder
        .linux
        .clone()
        .map(|ebl| ebl.icon)
        .flatten()
        .map(|p| current_dir.as_ref().join(p))
        .as_ref()
    {
        for file in fs::read_dir(original_icons_dir)
            .expect("reading icon directory (ebuilder.linux.icon) contents")
            .filter_map(Result::ok)
        {
            let filename__ = file.file_name();
            let filename = filename__.to_str().expect("reading file name in icons dir");

            lazy_static! {
                static ref ICON_FILE_REGEX: Regex = Regex::new(r#"^(\d+)x(\d+)\.png$"#).unwrap();
            }

            if let Some(captures) = ICON_FILE_REGEX.captures(filename) {
                let width: usize = captures.get(1).unwrap().as_str().parse().unwrap();
                let height: usize = captures.get(2).unwrap().as_str().parse().unwrap();
                icon_sizes.insert((width, height));

                oxipng::optimize(
                    &oxipng::InFile::Path(original_icons_dir.join(filename)),
                    &oxipng::OutFile::Path(Some(icons_dir.as_ref().join(filename))),
                    &oxipng::Options {
                        force: true, // always write
                        fix_errors: true,
                        ..Default::default()
                    },
                )
                .expect("optimizing/writing icon file");
            }
        }
    }

    // write a file with a list of sizes
    fs::write(
        icons_dir.as_ref().join("size-list"),
        icon_sizes
            .into_iter()
            .map(|(w, h)| format!("{w}x{h}"))
            .fold(String::new(), |a, b| a + &b + "\n"),
    )
    .expect("writing icon size list");
}

#[test]
fn test_gen_icons() {
    use crate::types::PackageJson;
    use std::env::current_dir;

    let package: PackageJson =
        serde_json::from_str(include_str!("test_assets/package.json")).unwrap();

    let current_dir = current_dir().unwrap();
    let icons_dir = current_dir.join(".test-workspace/icons_linux");
    fs::create_dir_all(&icons_dir).unwrap();

    gen_icons(
        package.build.as_ref().unwrap(),
        current_dir,
        icons_dir.clone(),
    );

    for size in [10, 128, 256] {
        assert!(icons_dir.join(format!("{size}x{size}.png")).exists());
    }
}
