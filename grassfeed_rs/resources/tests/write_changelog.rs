use flate2::write::GzEncoder;
use flate2::Compression;
use resources::changelog_debian;
use std::fs::File;
use std::io::Write;

const PKGNAME: &str = "grassfeeder-gtk3";

const DISTRIBUTION_SOURCE_DEB: &str = "buster";
const DISTRIBUTION_SOURCE_UBU: &str = "jammy";
const NAME_EMAIL: &str = "Marcus der Schlegler <schlegler_marcus@posteo.de>";

const ENABLE_GZ: bool = false;
const PATH_DEB: &str = "../app-gtk3-linux/";
const PATH_UBU: &str = "../app-gtk3-ubuntu/";
const CHANGELOG_PLAINTEXT: &str = "assets/changelog.txt";
const CHANGELOG_GZIP: &str = "assets/changelog.gz";

#[test]
pub fn write_changelog() {
    let recent_version = include_str!("../version.txt");
    for (d_path, d_ident) in [
        (PATH_DEB, DISTRIBUTION_SOURCE_DEB),
        (PATH_UBU, DISTRIBUTION_SOURCE_UBU),
    ] {
        let changelog_text = changelog_debian::create_debian_changelog(
            "../app-changes/",
            &format!("{}{}", d_path, CHANGELOG_PLAINTEXT),
            PKGNAME,
            &format!("{}; urgency=low", d_ident),
            NAME_EMAIL,
            recent_version,
        );
        let mut e = GzEncoder::new(Vec::new(), Compression::best());
        e.write_all(changelog_text.as_bytes()).unwrap();
        if ENABLE_GZ {
            let mut filegz = File::create(&format!("{}{}", d_path, CHANGELOG_GZIP,)).unwrap();
            let compressed_bytes: Vec<u8> = e.finish().unwrap();
            filegz.write_all(&compressed_bytes).unwrap();
        }
    }
}
