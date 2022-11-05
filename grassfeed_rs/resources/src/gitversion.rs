use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const RCS_VERSION_FILENAME: &str = "gen_git_info.rs";

pub fn build_rs_main(build_out_folder: &str) {
    let gen_filename = format!("{}/{}", build_out_folder, RCS_VERSION_FILENAME);
    let r = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .output()
        .expect("ERROR on:   git rev-parse --short HEAD   !");
    let c_out = String::from_utf8_lossy(&r.stdout);
    let git_version = c_out.trim();

    let r = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .expect("ERROR on:   git rev-parse --abbrev-ref HEAD   !");
    let c_out = String::from_utf8_lossy(&r.stdout);
    let git_branchname = c_out.trim();

    let mut lines: Vec<String> = Vec::default();
    lines.push(format!("// {} ", gen_filename));
    lines.push(format!(
        "pub const RCS_VERSION : &'static str = \"{}\";	",
        git_version
    ));
    lines.push(format!(
        "pub const RCS_BRANCH : &'static str = \"{}\";",
        git_branchname
    ));
    lines.push(format!(
        "pub const RCS_CARGO_PKG_VERSION : &'static str = \"{}\";",
        env!("CARGO_PKG_VERSION")
    ));
    let filecontent = lines.join("\n");
    let dest_path = Path::new(&gen_filename);
    fs::write(dest_path, &filecontent).unwrap();
}
