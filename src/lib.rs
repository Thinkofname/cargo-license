

extern crate cargo;
extern crate toml;
#[macro_use]
extern crate error_chain;
extern crate cargo_metadata;
extern crate semver;

use std::io;
use cargo::util::CargoResult;
use std::path::Path;

// I thought this crate is a good example to learn error_chain
// but looks like no need of it in this crate
error_chain! {
    types {
        Error, ErrorKind, ChainErr, Result;
    }

    links {}

    foreign_links {
        Io(io::Error);
        Metadata(cargo_metadata::Error);
    }

    errors {}
}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub source: String,
}


impl Dependency {
    fn get_cargo_package(&self) -> CargoResult<cargo::core::Package> {
        use cargo::core::{Source, SourceId};
        use cargo::core::Dependency as CargoDependency;
        use cargo::util::{Config, errors::internal};
        use cargo::sources::SourceConfigMap;

        // TODO: crates-license is only working for crates.io registry
        if !self.source.starts_with("registry") {
            Err(internal("registry sources are unimplemented"))?;
        }

        let config = Config::default()?;
        let source_id = SourceId::from_url(&self.source)?;

        let source_map = SourceConfigMap::new(&config)?;
        let mut source = source_map.load(source_id, &Default::default())?;

        // update crates.io-index registry
        source.update()?;

        let dep =
            CargoDependency::parse_no_deprecated(&self.name, Some(&self.version), source_id)?;
        let deps = source.query_vec(&dep)?;
        if let cargo::core::source::MaybePackage::Ready(pck) = deps.iter()
            .map(|p| p.package_id())
            .max()
            .map(|pkgid| source.download(pkgid))
            .unwrap_or(Err(internal("PKG download error")))?
        {
            Ok(pck)
        } else {
            Err(internal("package isn't downloaded"))
        }
    }

    fn normalize(&self, license_string: &Option<String>) -> Option<String> {
        match license_string {
            &None => None,
            &Some(ref license) => {
                let mut list: Vec<&str> = license.split('/').collect();
                for elem in list.iter_mut() {
                    *elem = elem.trim();
                }
                list.sort();
                list.dedup();
                Some(list.join("/"))
            }
        }
    }

    pub fn get_authors(&self) -> CargoResult<Vec<String>> {
        let pkg = self.get_cargo_package()?;
        Ok(pkg.manifest().metadata().authors.clone())
    }

    pub fn get_license(&self) -> Option<String> {
        match self.get_cargo_package() {
            Ok(pkg) => {
                self.normalize(&pkg.manifest().metadata().license)
            }
            Err(_) => None,
        }
    }
}

fn expand_package(
    metadata: &cargo_metadata::Metadata,
    package: &cargo_metadata::Package,
    source: &str,
    features: &[String],
    uses_default_features: bool,
    deps_out: &mut Vec<Dependency>,
) {
    // Add self (dups removed later hopefully)

    deps_out.push(Dependency {
        name: package.name.clone(),
        version: package.version.clone(),
        source: source.to_owned(),
    });

    let mut full_features = Vec::new();
    let mut tmp_features = features.to_owned();
    if uses_default_features {
        if let Some(features) = package.features.get("default") {
            tmp_features.extend(features.iter().cloned());
        }
    }
    while let Some(f) = tmp_features.pop() {
        if !f.contains('/') {
            if let Some(features) = package.features.get(&f) {
                tmp_features.extend(features.iter().cloned());
            }
        }
        full_features.push(f);
    }

    for dep in &package.dependencies {
        if dep.kind != cargo_metadata::DependencyKind::Normal {

            continue;
        }
        if dep.optional {
            if !full_features.contains(&dep.name) {

                continue;
            }
        }

        let prefix = format!("{}/", dep.name);
        let mut dep_features = full_features.iter()
            .filter(|v| v.contains('/'))
            .map(|v| v.trim_left_matches(&prefix))
            .map(|v| v.to_owned())
            .collect::<Vec<_>>();
        dep_features.extend(dep.features.iter().cloned());
        let package = metadata.packages
            .iter()
            .find(|v| v.name == dep.name && dep.req.matches(&semver::Version::parse(&v.version).unwrap()))
            .expect("Missing dep");
        expand_package(metadata, package, dep.source.as_ref().map_or("", |v| v.as_str()), &dep_features, dep.uses_default_features, deps_out);
    }
}

pub fn get_dependencies_from_cargo_lock() -> Result<Vec<Dependency>> {
    let metadata = cargo_metadata::metadata_deps(Some(Path::new("Cargo.toml")), true)?;

    let packages = {
        let mut args = std::env::args().skip_while(|val| !val.starts_with("--packages"));
        match args.next() {
            Some(ref p) if p == "--packages" => args.next(),
            Some(p) => Some(p.trim_left_matches("--packages=").to_string()),
            None => None,
        }.expect("Missing package")
    };
    let features = {
        let mut args = std::env::args().skip_while(|val| !val.starts_with("--features"));
        match args.next() {
            Some(ref p) if p == "--features" => args.next(),
            Some(p) => Some(p.trim_left_matches("--features=").to_string()),
            None => None,
        }
    };
    let packages = packages.split(',');
    let features = features
        .map_or_else(Vec::new, |v| v.split(',').map(|v| v.to_owned()).collect::<Vec<_>>());

    let mut deps = Vec::new();

    for package in packages {

        let package = metadata.packages
            .iter()
            .find(|v| v.name == package)
            .expect("Missing package");
        expand_package(&metadata, package, "local package", &features, true, &mut deps);
    }

    deps.sort_by(|a, b| {
        a.name.cmp(&b.name)
            .then_with(|| {
                a.version.cmp(&b.version)
            })
    });
    deps.dedup_by(|a, b| a.name == b.name && a.version == b.version);

    return Ok(deps);
}



#[cfg(test)]
mod test {
    use super::get_dependencies_from_cargo_lock;

    #[test]
    fn test() {

        for dependency in get_dependencies_from_cargo_lock().unwrap() {
            assert!(!dependency.get_license().unwrap().is_empty());
        }
    }
}
