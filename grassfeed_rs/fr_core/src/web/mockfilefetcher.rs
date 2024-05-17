use crate::web::HttpGetResult;
use crate::web::IHttpRequester;
use std::fs::File;
use std::io::Read;

pub struct FileFetcher {
    base_folder: String,
}

impl FileFetcher {
    pub fn new(folder: String) -> Self {
        FileFetcher {
            base_folder: folder,
        }
    }
}

const REPLACE_LOCALHOST: &str = "http://localhost/";

impl IHttpRequester for FileFetcher {
    fn request_url(&self, url: &str) -> HttpGetResult {
        let mut r = HttpGetResult::default();
        let mut p_url: String = url.to_string();
        if p_url.starts_with(REPLACE_LOCALHOST) {
            p_url = p_url.split_off(REPLACE_LOCALHOST.len());
        }
        let fs_file = format!("{}{}", self.base_folder, p_url);
        match std::fs::read_to_string(fs_file.clone()) {
            Ok(s) => {
                r.content = s;
                r.status = 200;
            }
            Err(e) => {
                r.status = 404;
                r.error_description = format!("{e} {fs_file}");
            }
        }
        r
    }

    fn request_url_bin(&self, url: &str) -> HttpGetResult {
        let mut p_url: String = url.to_string();
        if p_url.starts_with(REPLACE_LOCALHOST) {
            p_url = p_url.split_off(REPLACE_LOCALHOST.len());
        }
        let mut r = HttpGetResult::default();
        let fs_file = format!("{}{}", self.base_folder, p_url);
        match file_to_bin(&fs_file) {
            Ok(bytes_vec) => {
                r.content_bin = bytes_vec;
                r.status = 200;
            }
            Err(e) => {
                r.status = 404;
                r.error_description = format!("{e} {fs_file}");
            }
        }
        r
    }
}

pub fn file_to_bin(filename: &str) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(filename)?;
    let mut buffer: Vec<u8> = Vec::new();
    let _readsize = f.read_to_end(&mut buffer)?;
    Ok(buffer)
}
