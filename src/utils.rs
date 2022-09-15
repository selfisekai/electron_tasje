use std::{
    collections::HashSet,
    env,
    fmt::Debug,
    path::{Path, PathBuf},
};

use globset::{Glob, GlobSetBuilder};
use globwalk::{FileType, GlobWalkerBuilder};
use path_absolutize::Absolutize;
use regex::{Captures, Regex};

use crate::{
    types::{FileSet, NodeArch, StringOrMultiple},
    STANDARD_FILTERS,
};

lazy_static! {
    static ref ROOT: PathBuf = PathBuf::from("/");
}

pub fn get_globs_and_file_sets(files: Vec<FileSet>) -> (Vec<String>, Vec<FileSet>) {
    let global_globs = files
        .iter()
        .filter(|set| set.to.is_none())
        .map(|set| {
            if set.from.starts_with("./") {
                set.from[1..].to_string()
            } else if set.from.starts_with("!./") {
                "!/".to_owned() + &set.from[3..]
            } else if set.from.starts_with('!') || set.from.starts_with('/') {
                set.from.clone()
            } else {
                "/".to_owned() + &set.from
            }
        })
        .collect();

    let file_sets = files
        .into_iter()
        .filter(|set| set.to.is_some() || set.filter.is_some())
        .map(|mut set| {
            if set.filter.is_none() {
                set.filter = Some(StringOrMultiple::Multiple(vec!["**/*".to_string()]));
            } else if let Some(fl) = set.filter.as_ref() {
                if Vec::<String>::from(fl).iter().all(|fi| fi.starts_with('!')) {
                    set.filter = Some(StringOrMultiple::Multiple(
                        vec!["**/*".to_string()]
                            .into_iter()
                            .chain(Vec::<String>::from(set.filter.unwrap()))
                            .collect(),
                    ));
                }
            }
            set.filter = set.filter.map(|f| {
                StringOrMultiple::Multiple(
                    Vec::<String>::from(&f)
                        .into_iter()
                        .chain(STANDARD_FILTERS.into_iter().map(str::to_string))
                        .collect(),
                )
            });
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
        .file_type(FileType::FILE)
        .follow_links(true)
        .build()
        .unwrap()
        .filter_map(Result::ok)
    {
        let file_path = dir_entry.path();
        copy_list.insert((
            file_path
                .absolutize()
                .expect("absolutizing copy source path")
                .to_path_buf(),
            file_path
                .strip_prefix(&base_dir)
                .unwrap()
                .absolutize_from(&PathBuf::from("/"))
                .expect("absolutizing copy target path")
                .to_path_buf(),
        ));
    }

    for file_set in file_sets {
        let set_base_dir = base_dir.as_ref().join(&file_set.from);
        let target_dir = PathBuf::from(file_set.to.clone().unwrap_or_default());
        for dir_entry in GlobWalkerBuilder::from_patterns(
            &set_base_dir,
            &Vec::<String>::from(file_set.filter.as_ref().unwrap()),
        )
        .file_type(FileType::FILE)
        .follow_links(true)
        .build()
        .unwrap()
        .filter_map(Result::ok)
        {
            let file_path = dir_entry.path();
            copy_list.insert((
                file_path
                    .absolutize()
                    .expect("absolutizing copy source path")
                    .to_path_buf(),
                ROOT.join(&target_dir)
                    .join(file_path.strip_prefix(&set_base_dir).unwrap())
                    .absolutize_from(ROOT.as_path())
                    .expect("absolutizing copy target path")
                    .to_path_buf(),
            ));
        }
    }

    return copy_list;
}

pub fn refilter_copy_list<S: AsRef<str> + Debug>(
    original_copy_list: &HashSet<(PathBuf, PathBuf)>,
    new_globs: &[S],
) -> HashSet<(PathBuf, PathBuf)> {
    let mut set_b = GlobSetBuilder::new();
    let set_negations: Vec<bool> = new_globs
        .iter()
        .map(|g| g.as_ref().starts_with('!'))
        .collect();
    for (i, g) in new_globs.iter().enumerate() {
        let mut glob = g.as_ref();
        if *set_negations.get(i).unwrap() {
            // strip negation
            glob = &glob[1..];
        }
        set_b.add(Glob::new(glob).unwrap());
    }
    let set = set_b.build().unwrap();
    let mut copy_list = HashSet::new();

    copy_list.extend(
        original_copy_list
            .clone()
            .iter()
            .filter(|(_, copy_target)| {
                let matches = set.matches(copy_target);
                // if no matches (including negations), filter out
                if matches.len() == 0 {
                    false
                } else {
                    matches.iter().any(|ms| {
                        // if any negated glob matches, filter out
                        !set_negations.get(ms.clone()).unwrap().to_owned()
                    })
                }
            })
            .map(|s| s.to_owned()),
    );

    return copy_list;
}

pub fn host_node_arch() -> NodeArch {
    #[cfg(target_arch = "x86_64")]
    return NodeArch::X64;

    #[cfg(target_arch = "x86")]
    return NodeArch::IA32;

    #[cfg(target_arch = "aarch64")]
    return NodeArch::Arm64;

    #[cfg(target_arch = "arm")]
    return NodeArch::Arm;
}

pub fn fill_variable_template<S: AsRef<str>>(template: S) -> String {
    lazy_static! {
        static ref TEMPLATE_REGEX: Regex = Regex::new(r#"\$\{([a-zA-Z_. ]+)\}"#).unwrap();
    }
    TEMPLATE_REGEX
        .replace_all(template.as_ref(), |captures: &Captures| -> String {
            let variable = captures.get(1).unwrap().as_str().trim();
            match variable {
                "arch" => host_node_arch().to_string(),
                "platform" => "linux".to_string(),
                v => {
                    if let Some(envar) = v.strip_prefix("env.") {
                        env::var(envar).expect("getting env variable contents")
                    } else {
                        todo!("unknown template variable: '{variable}'")
                    }
                }
            }
        })
        .to_string()
}
