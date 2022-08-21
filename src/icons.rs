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
    } else if let Some(icns_file_path) = ebuilder
        .mac
        .clone()
        .map(|ebl| ebl.icon)
        .flatten()
        .or(Some("build/icon.icns".to_string()))
        .map(|p| current_dir.as_ref().join(p))
        .as_ref()
        .filter(|p| p.exists())
    {
        let icns_file = fs::File::open(icns_file_path).expect("opening mac .icns file");
        let icns_contents = icns::IconFamily::read(icns_file).expect("parsing mac .icns file");
        for available_icon in icns_contents.available_icons() {
            let icns_entry = icns_contents.get_icon_with_type(available_icon).unwrap();

            let width = icns_entry.width();
            let height = icns_entry.height();
            icon_sizes.insert((width.try_into().unwrap(), height.try_into().unwrap()));

            let filename = format!("{width}x{height}.png");
            let out_path = icons_dir.as_ref().join(&filename);

            let png_file =
                fs::File::create(&out_path).expect("creating .png icon file (from .icns)");
            icns_entry
                .write_png(png_file)
                .expect("writing .png file from .icns");
            oxipng::optimize(
                &oxipng::InFile::Path(out_path),
                &oxipng::OutFile::Path(None),
                &oxipng::Options {
                    fix_errors: true,
                    ..Default::default()
                },
            )
            .expect("optimizing/writing icon file");
        }
    } else if let Some(ico_file_path) = ebuilder
        .win
        .clone()
        .map(|ebl| ebl.icon)
        .flatten()
        .or(Some("build/icon.ico".to_string()))
        .map(|p| current_dir.as_ref().join(p))
        .as_ref()
        .filter(|p| p.exists())
    {
        let ico_file = fs::File::open(ico_file_path).expect("opening win .ico file");
        let ico_contents = ico::IconDir::read(ico_file).expect("parsing win .ico file");
        for ico_entry in ico_contents.entries() {
            let width = ico_entry.width();
            let height = ico_entry.height();
            icon_sizes.insert((width.try_into().unwrap(), height.try_into().unwrap()));

            let filename = format!("{width}x{height}.png");
            let out_path = icons_dir.as_ref().join(&filename);

            let png_file =
                fs::File::create(&out_path).expect("creating .png icon file (from .ico)");
            ico_entry
                .decode()
                .expect("decoding .ico entry")
                .write_png(png_file)
                .expect("writing .png icon file from .ico");
            oxipng::optimize(
                &oxipng::InFile::Path(out_path),
                &oxipng::OutFile::Path(None),
                &oxipng::Options {
                    fix_errors: true,
                    ..Default::default()
                },
            )
            .expect("optimizing/writing icon file");
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

#[test]
fn test_gen_icons_mac() {
    use crate::types::PackageJson;
    use std::env::current_dir;

    let package: PackageJson =
        serde_json::from_str(include_str!("test_assets/package-mac.json")).unwrap();

    let current_dir = current_dir().unwrap();
    let icons_dir = current_dir.join(".test-workspace/icons_mac");
    fs::create_dir_all(&icons_dir).unwrap();

    gen_icons(
        package.build.as_ref().unwrap(),
        current_dir,
        icons_dir.clone(),
    );

    for size in [128, 256, 512] {
        assert!(icons_dir.join(format!("{size}x{size}.png")).exists());
    }
}

#[test]
fn test_gen_icons_win() {
    use crate::types::PackageJson;
    use std::env::current_dir;

    let package: PackageJson =
        serde_json::from_str(include_str!("test_assets/package-win.json")).unwrap();

    let current_dir = current_dir().unwrap();
    let icons_dir = current_dir.join(".test-workspace/icons_win");
    fs::create_dir_all(&icons_dir).unwrap();

    gen_icons(
        package.build.as_ref().unwrap(),
        current_dir,
        icons_dir.clone(),
    );

    // single size stored in test .ico
    assert!(icons_dir.join(format!("32x32.png")).exists());
}
