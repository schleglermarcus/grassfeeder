#[path = "../resources/src/gitversion.rs"]
mod gitversion;

#[path = "../resources/src/changelog_debian.rs"]
mod changelog_debian;

pub fn main() {
    if let Some(out_dir) = changelog_debian::get_env("OUT_DIR") {
        gitversion::build_rs_main(&out_dir);
    }
    // let changelog_file = format!(        "{}/changelog.txt",        changelog_debian::get_env("CARGO_TARGET_DIR").unwrap_or("../target/".to_string())    );
    // &changelog_debian::get_env("CARGO_PKG_AUTHORS")		.unwrap_or("Marcus der Schlegler <schleglermarcus@posteo.de>".to_string())
    changelog_debian::create_debian_changelog(
        "../app-changes/",
        "src/changelog.txt",
        "grassfeeder",
        "unstable; urgency=low",
        "Marcus der Schlegler <schleglermarcus@posteo.de>",
    );
    println!("cargo:rerun-if-changed=build.rs");
}


// https://doc.rust-lang.org/cargo/reference/environment-variables.html
