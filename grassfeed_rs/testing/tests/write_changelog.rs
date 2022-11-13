// use flate2::bufread::GzEncoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use resources::changelog_debian;
use std::fs::File;
use std::io::Write;

const CHANGELOG_PLAINTEXT: &str = "../app-gtk3-linux/assets/changelog.txt";
const CHANGELOG_GZIP: &str = "../app-gtk3-linux/assets/changelog.gz";

#[test]
pub fn write_changelog() {
    let changelog_text = changelog_debian::create_debian_changelog(
        "../app-changes/",
        CHANGELOG_PLAINTEXT,
        "grassfeeder",
        "unstable; urgency=low",
        "Marcus der Schlegler <schleglermarcus@posteo.de>",
    );

    // let compressed_file = File::create("backup.tar.gz")?;
    // let mut encoder = GzEncoder::new(compressed_file, Compression::Default);
    // encoder.write(&archive.into_inner()?)?;
    // encoder.finish()?;

    let mut e = GzEncoder::new(Vec::new(), Compression::default());

    e.write_all(changelog_text.as_bytes()).unwrap();
    // e.write_str(changelog_text);
    let mut filegz = File::create(CHANGELOG_GZIP).unwrap();
    let compressed_bytes: Vec<u8> = e.finish().unwrap();
    filegz.write_all(&compressed_bytes).unwrap();
}
