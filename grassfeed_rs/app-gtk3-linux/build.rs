// #[path = "../resources/src/gitversion.rs"]
// mod gitversion;

#[path = "../resources/src/changelog_debian.rs"]
mod changelog_debian;

pub fn main() {
	// not usable when using the source tree without git
    // if let Some(out_dir) = changelog_debian::get_env("OUT_DIR") {
    //     gitversion::build_rs_main(&out_dir);
    // }
    changelog_debian::create_debian_changelog(
        "../app-changes/",
        "src/changelog.txt",
        "grassfeeder",
        "unstable; urgency=low",
        "Marcus der Schlegler <schleglermarcus@posteo.de>",
    );
    println!("cargo:rerun-if-changed=build.rs");
}
