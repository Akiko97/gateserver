use std::env;
use std::process::Command;
use std::path::Path;
use std::fs;

pub fn main() {
    let project_path = env::var("CARGO_MANIFEST_DIR").unwrap();
    let web_path = format!("{project_path}/web");
    println!("cargo:rerun-if-changed={project_path}/src/config/server.json");
    println!("cargo:rerun-if-changed={web_path}");

    let mut yarn = Command::new("yarn")
        .current_dir(web_path.clone())
        .spawn()
        .expect(format!("Could not run `yarn` in {web_path}").as_str());
    yarn.wait().expect("Error in running `yarn`");

    if Path::new(format!("{web_path}/server.json").as_str()).exists() {
        fs::remove_file(format!("{web_path}/server.json"))
            .expect(format!("Could not delete {web_path}/server.json").as_str());
    }
    fs::copy(
        format!("{project_path}/src/config/server.json"),
        format!("{web_path}/server.json")
    ).unwrap();

    let mut yarn = Command::new("yarn")
        .arg("build")
        .current_dir(web_path.clone())
        .spawn()
        .expect(format!("Could not run `yarn build` in {web_path}").as_str());
    yarn.wait().expect("Error in running `yarn build`");

    if Path::new(format!("{web_path}/dist").as_str()).exists() {
        if Path::new(format!("{project_path}/dist").as_str()).exists() {
            fs::remove_dir_all(format!("{project_path}/dist"))
                .expect(format!("Could not delete {project_path}/dist").as_str());
        }
        fs::rename(
            format!("{web_path}/dist"),
            format!("{project_path}/dist")
        ).unwrap();
    }
}
