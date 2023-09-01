use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;

static PNG_SIZE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^(\d+)x(\d+)\.png$"#).unwrap());

pub struct IconGenerator {
    icon_sizes: HashSet<(u64, u64)>,
}

impl IconGenerator {
    pub fn new() -> Self {
        Self {
            icon_sizes: HashSet::new(),
        }
    }

    pub fn generate<P1, P2>(mut self, icon_locations: Vec<P1>, icons_dir: P2) -> Result<()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let icons_dir = icons_dir.as_ref();
        for location in icon_locations {
            let location = location.as_ref();
            self.handle_location(location, icons_dir)?;
        }

        let mut sizes = self.icon_sizes.into_iter().collect::<Vec<_>>();
        sizes.sort_by(|(w1, h1), (w2, h2)| w1.cmp(w2).then_with(|| h1.cmp(h2)));
        let sizes = sizes
            .into_iter()
            .map(|(w, h)| format!("{w}x{h}"))
            .collect::<Vec<_>>();
        fs::write(icons_dir.join("size-list"), sizes.join("\n"))?;

        Ok(())
    }

    fn handle_location(&mut self, location: &Path, icons_dir: &Path) -> Result<()> {
        if location.is_file() {
            self.handle_file(location, icons_dir)?;
        } else if location.is_dir() {
            // expected according to docs: multiple pngs
            for entry in fs::read_dir(location)? {
                let entry = entry?;
                self.handle_file(entry.path().as_ref(), icons_dir)?;
            }
        }
        Ok(())
    }

    fn handle_file(&mut self, location: &Path, icons_dir: &Path) -> Result<()> {
        let mut file = fs::File::open(location)?;
        let mut head = [0; 4];
        file.read_exact(&mut head)?;

        match &head {
            b"icns" => {
                self.handle_icns(location, icons_dir)?;
            }
            // ico
            [0x00, 0x00, 0x01, 0x00] => {
                self.handle_ico(location, icons_dir)?;
            }
            // png
            [0x89, 0x50, 0x4e, 0x47] => {
                self.handle_png(location, icons_dir)?;
            }

            // unknown, ignore
            _ => {}
        }

        Ok(())
    }

    fn handle_ico(&mut self, ico_path: &Path, icons_dir: &Path) -> Result<()> {
        let container = ico::IconDir::read(
            fs::File::open(ico_path)
                .with_context(|| format!("on reading ico icon: {ico_path:?}"))?,
        )
        .with_context(|| format!("on parsing ico icon: {ico_path:?}"))?;
        for entry in container.entries() {
            let (width, height) = (entry.width(), entry.height());
            if self
                .icon_sizes
                .insert((width.into(), height.into()))
            {
                let target_png = icons_dir.join(format!("{width}x{height}.png"));
                entry
                    .decode()
                    .with_context(|| format!("on decoding ico entry from: {ico_path:?}"))?
                    .write_png(
                        fs::File::create(&target_png)
                            .with_context(|| format!("on creating png icon: {target_png:?}"))?,
                    )
                    .with_context(|| format!("on writing png icon: {target_png:?}"))?;
                self.optimize_png(target_png)?;
            }
        }
        Ok(())
    }

    fn handle_icns(&mut self, icns_path: &Path, icons_dir: &Path) -> Result<()> {
        let family = icns::IconFamily::read(
            fs::File::open(icns_path).with_context(|| format!("on opening icns: {icns_path:?}"))?,
        )
        .with_context(|| format!("on parsing icns: {icns_path:?}"))?;

        for icon_type in family.available_icons() {
            let icon = family
                .get_icon_with_type(icon_type)
                .with_context(|| format!("on getting icns icon: {icon_type:?}, {icns_path:?}"))?;
            let (width, height) = (icon.width(), icon.height());
            if self
                .icon_sizes
                .insert((width.into(), height.into()))
            {
                let target_png = icons_dir.join(format!("{width}x{height}.png"));
                icon.write_png(
                    fs::File::create(&target_png)
                        .with_context(|| format!("on creating png icon: {target_png:?}"))?,
                )
                .with_context(|| format!("on writing png icon: {target_png:?}"))?;
                self.optimize_png(target_png)?;
            }
        }

        Ok(())
    }

    fn handle_png(&mut self, png_path: &Path, icons_dir: &Path) -> Result<()> {
        // this blindly trusts that the sizes in filename are correct
        if let Some((width, height)) = png_path
            .file_name()
            .map(OsStr::to_str)
            .flatten()
            .map(|filename| PNG_SIZE_REGEX.captures(filename))
            .flatten()
            .map(|c| -> (u64, u64) {
                (
                    c.get(1).unwrap().as_str().parse().unwrap(),
                    c.get(2).unwrap().as_str().parse().unwrap(),
                )
            })
        {
            if self.icon_sizes.insert((width, height)) {
                let target_path = icons_dir.join(format!("{width}x{height}.png"));
                fs::copy(png_path, &target_path)
                    .with_context(|| format!("on copying png icon: {png_path:?}"))?;
                self.optimize_png(target_path)?;
            }
        }

        Ok(())
    }

    fn optimize_png(&self, png_path: PathBuf) -> Result<()> {
        oxipng::optimize(
            &oxipng::InFile::Path(png_path.clone()),
            &oxipng::OutFile::Path(None),
            &oxipng::Options {
                fix_errors: true,
                ..Default::default()
            },
        )
        .with_context(|| format!("on optimizing png icon: {png_path:?}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::IconGenerator;
    use crate::app::App;

    use anyhow::Result;
    use std::fs::{create_dir_all, read_to_string};
    use std::path::Path;

    #[test]
    fn test_linux() -> Result<()> {
        let icons_dir = Path::new(".test-workspace/icons_linux");
        create_dir_all(icons_dir)?;
        let app = App::new_from_package_file("test_assets/package.json")?;
        IconGenerator::new().generate(app.icon_locations(), icons_dir)?;
        assert_eq!(
            read_to_string(icons_dir.join("size-list"))?,
            "10x10
128x128
256x256"
        );
        for name in ["10x10.png", "128x128.png", "256x256.png"] {
            assert!(icons_dir.join(name).is_file());
        }
        Ok(())
    }

    #[test]
    fn test_win() -> Result<()> {
        let icons_dir = Path::new(".test-workspace/icons_win");
        create_dir_all(icons_dir)?;
        let app = App::new_from_package_file("test_assets/package-win.json")?;
        IconGenerator::new().generate(app.icon_locations(), icons_dir)?;
        assert_eq!(read_to_string(icons_dir.join("size-list"))?, "32x32");
        for name in ["32x32.png"] {
            assert!(icons_dir.join(name).is_file());
        }
        Ok(())
    }

    #[test]
    fn test_mac() -> Result<()> {
        let icons_dir = Path::new(".test-workspace/icons_mac");
        create_dir_all(icons_dir)?;
        let app = App::new_from_package_file("test_assets/package-mac.json")?;
        IconGenerator::new().generate(app.icon_locations(), icons_dir)?;
        assert_eq!(
            read_to_string(icons_dir.join("size-list"))?,
            "128x128
256x256
512x512"
        );
        for name in ["128x128.png", "256x256.png", "512x512.png"] {
            assert!(icons_dir.join(name).is_file());
        }
        Ok(())
    }
}
