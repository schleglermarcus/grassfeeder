use std::fs;
use std::io;
use std::path::PathBuf;
use zip::result::ZipError;

// const ZIPS_DIR: &str = "../fr_core/tests/zips/";
// const UNPACK_DIR: &str = "../target/";
// #[test]
// fn unzip_1() {
//     setup();
//     let f1 = format!("{}websites.zip", ZIPS_DIR);
//     let r = unzip_one(&f1, UNPACK_DIR);
// }
//

pub fn unzip_one(src_file: &str, out_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let src_path = std::path::Path::new(&src_file);
    let file = fs::File::open(&src_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(io_to_boxed)?;
    // trace!("{} => {}", src_file, out_dir);
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpat_h: PathBuf = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        let pathstr = match outpat_h.to_str() {
            Some(op_str) => format!("{}{}", out_dir, op_str),
            None => continue,
        };
        if (*file.name()).ends_with('/') {
            // trace!("DIR {}  \"{}\"", i, pathstr);
            fs::create_dir_all(&pathstr)?;
        } else {
            // trace!("File {} extracted to \"{}\"", i, pathstr,);
            let mut outfile = fs::File::create(&pathstr).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    Ok(())
}

pub fn io_to_boxed(e: ZipError) -> Box<dyn std::error::Error> {
    Box::new(e)
}

/*
// ------------------------------------

mod logger_config;
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(
            logger_config::QuietFlags::Downloader as u64,
            // 0
        );
    });
}
*/
