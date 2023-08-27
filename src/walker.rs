use anyhow::Result;
use globreeks::Globreeks;
use std::path::{Path, PathBuf};
use std::vec::IntoIter;
use walkdir::WalkDir;

use crate::config::{CopyDef, FileSet};

#[derive(Debug)]
pub(crate) struct Walker<'a> {
    root: PathBuf,
    globs: Globreeks,
    sets: IntoIter<&'a FileSet>,
    current_set: Option<&'a FileSet>,
    current_walk: walkdir::IntoIter,
    done_with_globs: bool,
}

impl<'a> Walker<'a> {
    pub fn new(root: PathBuf, to_copy: Vec<&'a CopyDef>) -> Result<Self> {
        let mut globs = Vec::new();
        let mut sets = Vec::new();
        for def in to_copy {
            match def {
                CopyDef::Simple(g) => globs.push(g.as_str()),
                CopyDef::Set(s) => sets.push(s),
            }
        }

        Ok(Self {
            root: root.clone(),
            globs: Globreeks::new(globs)?,
            sets: sets.into_iter(),
            current_set: None,
            current_walk: WalkDir::new(root).into_iter(),
            done_with_globs: false,
        })
    }

    fn next_current_walk(&mut self) -> Option<PathBuf> {
        while let Some(next) = self.current_walk.next() {
            if let Ok(direntry) = next {
                if direntry.file_type().is_dir() {
                    continue;
                }
                let path = direntry.path().strip_prefix(&self.root).unwrap();
                if let Some(path_str) = path.to_str() {
                    if self.globs.evaluate(path_str) {
                        let buf = path.to_path_buf();
                        return Some(buf);
                    }
                }
            }
        }
        return None;
    }
}

impl<'a> Iterator for Walker<'a> {
    /// source, dest
    type Item = (PathBuf, PathBuf);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.done_with_globs {
            if let Some(path) = self.next_current_walk() {
                return Some((self.root.join(&path), path));
            }
            self.done_with_globs = true;
        }

        loop {
            if let Some(set) = self.current_set {
                if let Some(path) = self.next_current_walk() {
                    return Some((
                        self.root.join(&path),
                        set.to
                            .as_ref()
                            .map(|to| Path::new(&to).join(&path.strip_prefix(&set.from).unwrap()))
                            .unwrap_or(path),
                    ));
                }
            }
            self.current_set = self.sets.next();
            if let Some(current_set) = self.current_set {
                self.current_walk = WalkDir::new(self.root.join(&current_set.from)).into_iter();
                let mut filters = current_set.filters();
                if !filters.iter().any(|f| !f.starts_with('!')) {
                    filters = vec!["**/*"];
                }
                self.globs = Globreeks::new(filters).unwrap();
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env::current_dir;

    use crate::app::App;

    use super::Walker;
    use anyhow::Result;

    #[test]
    fn test_walking() -> Result<()> {
        let root = current_dir()?.join("src").join("test_assets");
        let app = App::new_from_package_file(root.join("package.json"))?;
        let walker = Walker::new(root, app.config().files())?;

        let full_list: Vec<_> = walker.collect();

        assert_eq!(
            full_list
                .into_iter()
                .map(|(_, dest)| dest.to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            vec!["build/bundle.aoeuid.js", "cuild/bundle.aoeuid.js",]
        );

        Ok(())
    }
}
