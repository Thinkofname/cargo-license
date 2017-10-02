
extern crate cargo_license;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[derive(Serialize)]
struct Dep {
    name: String,
    version: String,
    license: String,
}

fn main() {
    let dependencies = cargo_license::get_dependencies_from_cargo_lock().unwrap();

    let mut deps = vec![];

    for dependency in dependencies {
        let license = if let Some(dep) = dependency.get_license() {
            dep
        } else {
            continue
        };
        deps.push(Dep {
            name: dependency.name,
            version: dependency.version,
            license: license,
        });
    }

    serde_json::to_writer(::std::io::stdout(), &deps).unwrap();
}
