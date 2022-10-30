use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use zip::result::ZipError;

pub const TD_BASE: &str = "../target/td/";
pub const TD_SRC: &str = "../fr_core/tests/zips/";

pub fn unzip_some() -> bool {
    if Path::new(TD_BASE).is_dir() {
        // debug!("unzip_some: destination exists already, quit. {} ", TD_BASE);
        return false;
    }
    for n in ["websites.zip", "feeds.zip", "icons.zip"] {
        let r = unzip_one(&format!("{}{}", TD_SRC, n), TD_BASE);
        if !r.is_ok() {
            return false;
        }
    }
    true
}

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
