use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use globwalk::{FileType, GlobWalkerBuilder};

use crate::types::FileSet;

pub fn get_globs_and_file_sets(files: Vec<FileSet>) -> (Vec<String>, Vec<FileSet>) {
    let global_globs = files
        .iter()
        .filter(|set| set.to.is_none())
        .map(|set| set.from.clone())
        .chain(
            [
                "node_modules/**/*",
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
                "!**/{.prettierrc,webpack,.jshintrc,jsdoc,.eslintrc}{,.json,.js}",
                "!**/{yarn,npm}-{debug,error}{,.log,.json}",
                "!**/.{yarn,npm}-metadata,integrity",
                "!**/*.{iml,o,hprof,orig,pyc,pyo,rbc,swp,csproj,sln,xproj,c,h,cc,cpp,hpp,lzz,gyp,ts}",
            ]
            .into_iter()
            .map(str::to_string),
        )
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
        .file_type(FileType::FILE)
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
                .file_type(FileType::FILE)
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
