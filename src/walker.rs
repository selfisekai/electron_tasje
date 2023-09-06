use crate::config::{CopyDef, FileSet};
use crate::environment::Environment;
use crate::utils::{fill_variable_template, try_flatten};
use anyhow::Result;
use globreeks::Globreeks;
use std::path::{Path, PathBuf};
use std::vec::IntoIter;
use walkdir::WalkDir;

#[derive(Debug)]
pub(crate) struct Walker<'a> {
    root: PathBuf,
    globs: Globreeks,
    sets: IntoIter<(&'a FileSet, Vec<String>)>,
    current_set: Option<&'a FileSet>,
    current_walk: walkdir::IntoIter,
    done_with_globs: bool,
    unpack_globs: Option<Globreeks>,
}

impl<'a> Walker<'a> {
    pub(crate) fn new(
        root: PathBuf,
        environment: Environment,
        to_copy: Vec<&'a CopyDef>,
        unpack_list: Option<Vec<&str>>,
    ) -> Result<Self> {
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
            globs: Globreeks::new(try_flatten(
                globs
                    .iter()
                    .map(|f| fill_variable_template(f, environment)),
            )?)?,
            sets: try_flatten(sets.into_iter().map(|s| {
                Ok((
                    s,
                    try_flatten(
                        s.filters()
                            .into_iter()
                            .map(|f| fill_variable_template(f, environment)),
                    )?,
                ))
            }))?
            .into_iter(),
            current_set: None,
            current_walk: WalkDir::new(root).follow_links(true).into_iter(),
            done_with_globs: globs.is_empty(),
            unpack_globs: if let Some(gl) = unpack_list {
                Some(Globreeks::new(gl)?)
            } else {
                None
            },
        })
    }

    fn next_current_walk(&mut self) -> Option<(PathBuf, bool)> {
        while let Some(next) = self.current_walk.next() {
            if let Ok(direntry) = next {
                let path = direntry.path().strip_prefix(&self.root).unwrap();
                let path_cand = globreeks::Candidate::new(path);
                if self.globs.evaluate_candidate(&path_cand) && direntry.file_type().is_file() {
                    let unpack = self
                        .unpack_globs
                        .as_ref()
                        .map(|r| r.evaluate_candidate(&path_cand))
                        .unwrap_or(false);
                    let buf = path.to_path_buf();
                    return Some((buf, unpack));
                }
            }
        }
        return None;
    }
}

impl<'a> Iterator for Walker<'a> {
    /// source, dest
    type Item = (PathBuf, PathBuf, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.done_with_globs {
            if let Some((path, unpack)) = self.next_current_walk() {
                return Some((self.root.join(&path), path, unpack));
            }
            self.done_with_globs = true;
        }

        loop {
            if let Some(set) = self.current_set {
                if let Some((path, unpack)) = self.next_current_walk() {
                    return Some((
                        self.root.join(&path),
                        set.to()
                            .map(|to| {
                                Path::new(&to).join(
                                    &path
                                        .strip_prefix(set.from().unwrap_or_default())
                                        .unwrap(),
                                )
                            })
                            .unwrap_or(path),
                        unpack,
                    ));
                }
            }
            if let Some((new_set, new_globs)) = self.sets.next() {
                self.current_set = Some(new_set);
                self.current_walk =
                    WalkDir::new(self.root.join(new_set.from().unwrap_or_default()))
                        .follow_links(true)
                        .into_iter();
                let mut filters = new_globs;
                if !filters.iter().any(|f| !f.starts_with('!')) {
                    let mut new_filters = vec!["**/*".to_string()];
                    new_filters.extend(filters);
                    filters = new_filters;
                }
                self.globs =
                    Globreeks::new(filters.into_iter().by_ref().collect::<Vec<_>>()).unwrap();
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Walker;
    use crate::app::App;
    use crate::environment::HOST_ENVIRONMENT;
    use anyhow::Result;
    use std::path::PathBuf;

    #[test]
    fn test_walking() -> Result<()> {
        let root = PathBuf::from("test_assets");
        let app = App::new_from_package_file(root.join("package.json"))?;
        let walker = Walker::new(root, HOST_ENVIRONMENT, app.config().files(), None)?;

        let full_list: Vec<_> = walker.collect();

        assert_eq!(
            full_list
                .into_iter()
                .map(|(_, dest, _)| dest.to_str().unwrap().to_string())
                .collect::<Vec<_>>(),
            vec!["build/bundle.aoeuid.js", "cuild/bundle.aoeuid.js",]
        );

        Ok(())
    }
}
