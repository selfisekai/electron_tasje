use crate::app::App;
use crate::config::CopyDef;
use crate::desktop::DesktopGenerator;
use crate::environment::{Environment, Platform, HOST_ENVIRONMENT};
use crate::icons::IconGenerator;
use crate::walker::Walker;
use anyhow::Result;
use asar::AsarWriter;
use chaste::types::ModulePathSegment;
use once_cell::sync::Lazy;
use std::collections::{BTreeSet, HashSet};
use std::fs::{self, read, File};
use std::path::{Path, PathBuf};

static ROOT: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("/"));

static FORCED_FILTERS: Lazy<Vec<CopyDef>> = Lazy::new(|| {
    [
        "!**/node_modules/.bin",
        "!**/*.{md,rst,markdown}",
        "!**/{__tests__,powered-test,spec,example,examples,readme,README,Readme,changelog,CHANGELOG,Changelog,ChangeLog}",
        "!**/*.{spec,test}.*",
        "!**/._*",
        "!**/{.editorconfig,.DS_Store,.git,.svn,.hg,CVS,RCS,.gitattributes,.nvmrc,.nycrc,Makefile,CMakeLists.txt}",
        "!**/{__pycache__,thumbs.db,.flowconfig,.idea,.vs,.vscode,.nyc_output,.docker-compose.yml}",
        "!**/{.github,.gitlab,.gitlab-ci.yml,appveyor.yml,.travis.yml,circle.yml,.woodpecker.yml}",
        "!**/{package-lock.json,yarn.lock}",
        "!**/.{git,eslint,tslint,prettier,docker,npm,yarn}ignore",
        "!**/.{prettier,eslint,jshint,jsdoc}rc",
        "!**/{.prettierrc,webpack.config,.jshintrc,jsdoc,.eslintrc,tsconfig}{,.json,.js,.yml,yaml}",
        "!**/{yarn,npm}-{debug,error}{,.log,.json}",
        "!**/.{yarn,npm}-{metadata,integrity}",
        "!**/*.{iml,o,hprof,orig,pyc,pyo,rbc,swp,csproj,sln,xproj,c,h,cc,cpp,hpp,lzz,gyp,d.ts}",
    ].into_iter().map(str::to_string).map(CopyDef::Simple).collect()
});

#[derive(Clone, Debug)]
pub struct PackingProcessBuilder {
    app: App,
    base_output_dir: Option<PathBuf>,
    icons_output_dir: Option<PathBuf>,
    resources_output_dir: Option<PathBuf>,
    target_environment: Option<Environment>,
    additional_files: Vec<CopyDef>,
    additional_asar_unpack: Vec<String>,
    additional_extra_resources: Vec<CopyDef>,
}

impl PackingProcessBuilder {
    pub fn new(app: App) -> Self {
        PackingProcessBuilder {
            app,
            base_output_dir: None,
            icons_output_dir: None,
            resources_output_dir: None,
            target_environment: None,
            additional_files: Vec::new(),
            additional_asar_unpack: Vec::new(),
            additional_extra_resources: Vec::new(),
        }
    }

    pub fn base_output_dir<P>(mut self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.base_output_dir = Some(self.app.root.join(path.as_ref()));
        self
    }

    pub fn target_environment(mut self, env: Environment) -> Self {
        self.target_environment = Some(env);
        self
    }

    pub fn additional_files(mut self, add: Vec<CopyDef>) -> Self {
        self.additional_files.extend(add);
        self
    }

    pub fn additional_asar_unpack(mut self, add: Vec<String>) -> Self {
        self.additional_asar_unpack.extend(add);
        self
    }

    pub fn additional_extra_resources(mut self, add: Vec<CopyDef>) -> Self {
        self.additional_extra_resources.extend(add);
        self
    }

    pub fn build(self) -> PackingProcess {
        let environment = self
            .target_environment
            .unwrap_or(HOST_ENVIRONMENT);
        let base_output_dir = self.app.root.clone().join(
            self.base_output_dir
                .clone()
                .unwrap_or_else(|| self.app.output_dir(environment.platform)),
        );
        let icons_output_dir = base_output_dir.join(
            self.icons_output_dir
                .unwrap_or_else(|| "icons".into()),
        );
        let resources_output_dir = base_output_dir.join(
            self.resources_output_dir
                .unwrap_or_else(|| "resources".into()),
        );
        PackingProcess {
            app: self.app,
            base_output_dir,
            icons_output_dir,
            resources_output_dir,
            environment,
            additional_files: self.additional_files,
            additional_asar_unpack: self.additional_asar_unpack,
            additional_extra_resources: self.additional_extra_resources,
        }
    }
}

pub struct PackingProcess {
    pub app: App,
    base_output_dir: PathBuf,
    icons_output_dir: PathBuf,
    resources_output_dir: PathBuf,
    environment: Environment,
    additional_files: Vec<CopyDef>,
    additional_asar_unpack: Vec<String>,
    additional_extra_resources: Vec<CopyDef>,
}

impl PackingProcess {
    pub fn proceed(self) -> Result<()> {
        fs::create_dir_all(&self.resources_output_dir)?;
        fs::create_dir_all(&self.icons_output_dir)?;

        self.pack_asar()?;
        self.pack_extra(
            self.app
                .config()
                .extra_files(self.environment.platform),
            &self.base_output_dir,
        )?;
        self.pack_extra(
            self.app
                .config()
                .extra_resources(self.environment.platform),
            &self.resources_output_dir,
        )?;

        self.generate_desktop_file()?;
        self.generate_icons()?;

        Ok(())
    }

    /// this reads the lockfile (and package.json, for some packages) to tell apart
    /// dev and production dependencies, and return paths to be packed or not.
    ///
    /// let's say, hypothetically, production dependencies are stored in the paths
    /// `["node_modules/hoistable", "node_modules/packable", "node_modules/packable/node_modules/unhoistable",
    ///  "node_modules/unpackable/node_modules/unhoistable"]`,
    /// and dev dependencies are stored in `["node_modules/hoistable/node_modules/not_needed", "node_modules/unpackable"]`.
    /// in this case, while `"node_modules/unpackable/node_modules/unhoistable"` would come from an included package,
    /// it's also inside `"node_modules/unpackable"`, which is on the excluded list, therefore it should not be included.
    /// on the other hand, while `"node_modules/hoistable"` is to be included, `"node_modules/hoistable/node_modules/not_needed"`
    /// is not, and therefore lands on the explicitly excluded list.
    fn find_node_modules(&self) -> Result<Vec<CopyDef>> {
        let chastefile = chaste::from_root_path(&self.app.root)?;
        let root_pid = chastefile.root_package_id();
        let included_pids: HashSet<chaste::PackageID> = HashSet::from_iter(
            chastefile
                .recursive_prod_package_dependencies(root_pid)
                .into_iter()
                .map(|dep| dep.on),
        );
        let mut excluded_paths: BTreeSet<Vec<ModulePathSegment>> = BTreeSet::new();
        for pid in chastefile
            .recursive_package_dependencies(root_pid)
            .into_iter()
            .filter(|dep| !included_pids.contains(&dep.on))
            .map(|dep| dep.on)
        {
            for installation in chastefile.package_installations(pid) {
                excluded_paths.insert(installation.path().iter().collect());
            }
        }
        let mut included_paths: BTreeSet<Vec<ModulePathSegment>> = BTreeSet::new();
        let mut explicitly_excluded_paths: BTreeSet<Vec<ModulePathSegment>> = BTreeSet::new();
        for pid in &included_pids {
            'installation: for installation in chastefile.package_installations(*pid) {
                let path: Vec<ModulePathSegment> = installation.path().iter().collect();
                for idx in path.len()..=1 {
                    if excluded_paths.contains(&path[..idx]) {
                        debug_assert_ne!(idx, path.len());
                        continue 'installation;
                    }
                }
                for exc in excluded_paths.range(path.clone()..) {
                    if exc.len() >= path.len() && path.starts_with(exc) {
                        debug_assert_ne!(exc, &path);
                        explicitly_excluded_paths.insert(exc.clone());
                    } else {
                        break;
                    }
                }
                included_paths.insert(path);
            }
        }
        Ok(included_paths
            .into_iter()
            .map(|p| {
                CopyDef::Simple(
                    p.iter()
                        .map(|s| s.as_ref())
                        .collect::<Vec<&str>>()
                        .join("/"),
                )
            })
            .chain(explicitly_excluded_paths.into_iter().map(|p| {
                CopyDef::Simple(
                    "!".to_string()
                        + &p.iter()
                            .map(|s| s.as_ref())
                            .collect::<Vec<&str>>()
                            .join("/"),
                )
            }))
            .collect())
    }

    fn pack_asar(&self) -> Result<()> {
        let mut asar = AsarWriter::new();
        let asar_file = File::create(self.resources_output_dir.join("app.asar"))?;
        let unpack_dir = self
            .resources_output_dir
            .join("app.asar.unpacked");
        let modules = self.find_node_modules()?;
        let mut files: Vec<&CopyDef> = modules.iter().collect();
        files.extend(self.app.config().files(self.environment.platform));
        files.extend(self.additional_files.as_slice());
        files.extend(FORCED_FILTERS.as_slice());
        let unpack_list = Some(
            self.app
                .config()
                .asar_unpack(self.environment.platform)
                .iter()
                .chain(self.additional_asar_unpack.iter())
                .collect::<Vec<_>>(),
        )
        .filter(|l| !l.is_empty());

        // adding package.json separately, to handle extraMetadata
        asar.write_file(
            "/package.json",
            self.app
                .patched_package(self.environment.platform)?,
            false,
        )?;

        for (source, dest, unpack) in
            Walker::new(self.app.root.clone(), self.environment, files, unpack_list)?
        {
            // always packing package.json above
            if dest == Path::new("package.json") {
                continue;
            }
            asar.write_file(ROOT.join(&dest), read(&source)?, true)?;
            if unpack {
                let unpack_dest = unpack_dir.join(dest);
                fs::create_dir_all(unpack_dest.parent().unwrap())?;
                fs::copy(&source, &unpack_dest)?;
            }
        }
        asar.finalize(asar_file)?;

        Ok(())
    }

    fn pack_extra<P>(&self, copydefs: &[CopyDef], target: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let copydefs = copydefs
            .iter()
            .chain(self.additional_extra_resources.iter().by_ref())
            .collect::<Vec<_>>();
        if copydefs.is_empty() {
            // nothing to copy, don't bother looking
            return Ok(());
        }
        let target = target.as_ref();
        for (source, dest, _) in
            Walker::new(self.app.root.clone(), self.environment, copydefs, None)?
        {
            let unpack_dest = target.join(dest);
            fs::create_dir_all(unpack_dest.parent().unwrap())?;
            fs::copy(&source, &unpack_dest)?;
        }

        Ok(())
    }

    fn generate_desktop_file(&self) -> Result<()> {
        if self.environment.platform == Platform::Linux {
            DesktopGenerator::new().write_to_output_dir(
                &self.app,
                self.environment.platform,
                Some(&self.base_output_dir),
            )?;
        }

        Ok(())
    }

    fn generate_icons(&self) -> Result<()> {
        IconGenerator::new().generate(self.app.icon_locations(), &self.icons_output_dir)
    }
}
