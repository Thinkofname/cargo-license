
extern crate cargo_license;


fn main() {
    let dependencies = cargo_license::get_dependencies_from_cargo_lock().unwrap();

    println!("[");
    for dependency in dependencies {
        let license = dependency.get_license().unwrap_or("N/A".to_owned());
        println!(r#"{{"name": {:?}, "version": {:?}, "license": {:?}}}"#, dependency.name, dependency.version, license);
    }
    println!("]");
}
