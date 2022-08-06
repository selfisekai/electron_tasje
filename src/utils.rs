use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use globwalk::GlobWalkerBuilder;

use crate::types::FileSet;

pub fn get_globs_and_file_sets(files: Vec<FileSet>) -> (Vec<String>, Vec<FileSet>) {
    let global_globs = files
        .iter()
        .filter(|set| set.to.is_none())
        .map(|set| set.from.clone())
        .collect();

    let file_sets = files
        .into_iter()
        .filter(|set| set.to.is_some() || set.filter.is_some())
        .map(|mut set| {
            if set.filter.is_none() {
                set.filter = Some(vec!["**/*".to_string()]);
            } else if set
                .filter
                .is_some_and(|fl| fl.iter().all(|fi| fi.starts_with('!')))
            {
                set.filter = Some(
                    vec!["**/*".to_string()]
                        .into_iter()
                        .chain(set.filter.unwrap())
                        .collect(),
                );
            }
            set
        })
        .collect();

    (global_globs, file_sets)
}

pub fn gen_copy_list<P: AsRef<Path>, S: AsRef<str>>(
    base_dir: P,
    global_globs: &[S],
    file_sets: &[FileSet],
) -> HashSet<(PathBuf, PathBuf)> {
    let mut copy_list: HashSet<(PathBuf, PathBuf)> = HashSet::new();

    for dir_entry in GlobWalkerBuilder::from_patterns(&base_dir, &global_globs)
        .follow_links(true)
        .build()
        .unwrap()
        .filter_map(Result::ok)
    {
        let file_path = dir_entry.path();
        copy_list.insert((
            file_path.to_path_buf(),
            file_path.strip_prefix(&base_dir).unwrap().to_path_buf(),
        ));
    }

    for file_set in file_sets {
        let set_base_dir = base_dir.as_ref().join(&file_set.from);
        let target_dir = PathBuf::from(file_set.to.clone().unwrap_or_default());
        for dir_entry in
            GlobWalkerBuilder::from_patterns(&set_base_dir, file_set.filter.as_ref().unwrap())
                .follow_links(true)
                .build()
                .unwrap()
                .filter_map(Result::ok)
        {
            let file_path = dir_entry.path();
            copy_list.insert((
                file_path.to_path_buf(),
                target_dir.join(file_path.strip_prefix(&base_dir).unwrap()),
            ));
        }
    }

    return copy_list;
}
