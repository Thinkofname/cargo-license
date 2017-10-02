
extern crate cargo_license;


fn main() {
    let dependencies = cargo_license::get_dependencies_from_cargo_lock().unwrap();

    println!("[");
    for dependency in dependencies {
        let license = if let Some(dep) = dependency.get_license() {
            dep
        } else {
            continue
        };
        println!(r#"{{"name": {:?}, "version": {:?}, "license": {:?}}}"#, dependency.name, dependency.version, license);
    }
    println!("]");
}
