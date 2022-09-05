// #[path = "src/gitversion.rs"]
// mod gitversion;


pub fn main() {
	println!("cargo:rerun-if-changed=build.rs");

}


/*


// build.rs
// use crate::gitversion;
// mod src::gitversion;



use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;


// git rev-parse --short HEAD
pub fn main2() {
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

    let line1 = format!(
        "pub const RCS_VERSION : &'static str = \"{}\";",
        git_version
    );

    let line2 = format!(
        "pub const RCS_BRANCH : &'static str = \"{}\";",
        git_branchname
    );

    let line3 = format!(
        "pub const RCS_CARGO_PKG_VERSION : &'static str = \"{}\";",
        env!("CARGO_PKG_VERSION")
    );

    // let line4 = format!(
    //     "pub const APP_VERSION_COMBINED : &'static str = \"{} {} {}\";",
    //     env!("CARGO_PKG_VERSION"),
    //     git_branchname,
    //     git_version
    // );

    //  APP_VERSION_COMBINED: &str = format!("{}-{}-{}", CARGO_PKG_NAME ,CARGO_PKG_VERSION , RCS_VERSION ).as_str();

    let filecontent = [line1, line2, line3].join("\n");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("rcs_version.rs");
    fs::write(&dest_path, &filecontent).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
*/
