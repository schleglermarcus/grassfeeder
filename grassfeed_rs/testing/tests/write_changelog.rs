use resources::changelog_debian;

#[test]
pub fn write_changelog() {
    changelog_debian::create_debian_changelog(
        "../app-changes/",
        "../app-gtk3-linux/assets/changelog.txt",
        "grassfeeder",
        "unstable; urgency=low",
        "Marcus der Schlegler <schleglermarcus@posteo.de>",
    );
}
