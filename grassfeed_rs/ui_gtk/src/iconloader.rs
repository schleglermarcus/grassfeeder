// use dd::lz4_compression;

use gtk::glib::Bytes;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::Cancellable;
use gtk::gio::MemoryInputStream;
use lz4_compression::prelude;
use std::fs::File;
use std::io::Read;

#[allow(dead_code)]
impl IconLoader {
    ///
    pub fn file_to_bin(filename: &str) -> std::io::Result<Vec<u8>> {
        let mut f = File::open(filename)?;
        let mut buffer: Vec<u8> = Vec::new();
        let _readsize = f.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    ///  Compress the data, then  encode base64  into String
    pub fn compress_vec_to_string(uncompressed: &[u8]) -> String {
        let compressed_data = prelude::compress(uncompressed);
        base64::encode(compressed_data)
    }

    pub fn vec_to_pixbuf(buffer: &[u8]) -> Result<Pixbuf, gtk::glib::error::Error> {
        let mis: MemoryInputStream = MemoryInputStream::from_bytes(&Bytes::from(buffer));
        let cancellable: Option<&Cancellable> = None;
        let pb: Pixbuf = Pixbuf::from_stream(&mis, cancellable)?;
        Ok(pb)
    }

    ///  decode String from  base64  , then decompress the data, return String
    pub fn decompress_string_to_vec(compr_b64: &str) -> Vec<u8> {
        match base64::decode(compr_b64) {
            Ok(buffer) => match prelude::decompress(&buffer) {
                Ok(vec_u8) => {
                    return vec_u8;
                }
                Err(e) => {
                    error!("icon-decompress: {:?}", e);
                }
            },
            Err(e) => {
                error!("icon-decode:  {:?}", e);
            }
        }
        Vec::default()
    }
}

pub struct IconLoader {}

pub fn get_missing_icon() -> Pixbuf {
    IconLoader::vec_to_pixbuf(&IconLoader::decompress_string_to_vec(ICON_MISSING_STR)).unwrap()
}

#[allow(dead_code)]
pub const ICON_MISSING_STR: &str = "8AWJUE5HDQoaCgAAAA1JSERSAAAAQAQA+dQIAwAAAJ23gewAAAAEZ0FNQQAAsY8L/GEFAAAAIGNIUk0AAHomAACAhAAA+gAAAIDoAAB1MAAA6mAAADqYAAAXcJy6UTwAAAAGUExURQAAAP8AABv/jSIAAAABdFJOUwBA5thmAAAAB3RJTUUH5QwbBCYjrgVHhAAAAC9JREFUWMPtzDEBACAMA7Di3zQaNr6SCEjOowiqggwJBAKBQCAQCASCf4ItQUVwAfOIBwFYkamfAAAAJXRFWHRkYXRlOmNyZWF0ZQAyMDIxLTEyLTI3VDAzOjM4OjM1KzAxOjAwcFZSLTEAb21vZGlmeTEAB/ABAQvqkQAAAABJRU5ErkJggg==";
