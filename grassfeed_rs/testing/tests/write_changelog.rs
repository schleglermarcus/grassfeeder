use resources::changelog_debian;


fn get_env(key: &str) -> Option<String> {
    if let Some(s1) = std::env::var_os(key) {
        if let Some(s2) = s1.to_str() {
            return Some(s2.to_string());
        }
    }
    None
}

#[test]
pub fn write_changelog() {
    setup();
    debug!("DISPLAY={:?}", get_env("DISPLAY"));
    debug!("HOME={:?}", get_env("HOME"));

    changelog_debian::create_debian_changelog(
        "../app-changes/",
        "../app-gtk3-linux/src/changelog.txt",
        "grassfeeder",
        "unstable; urgency=low",
        "Marcus der Schlegler <schleglermarcus@posteo.de>",
    );
}
/*
    resources::changelog_debian::create_debian_changelog(
        "../app-changes/",
        "../target/test_debian_changelog.txt",
        "grassfeeder",
        "unstable; urgency=low",
        "Marcus der Schlegler <schleglermarcus@posteo.de>",
    );
*/

// ------------------------------------
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config::setup_logger();
    });
}
