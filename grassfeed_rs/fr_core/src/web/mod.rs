pub mod httpfetcher;
pub mod mockfilefetcher;

use std::sync::Arc;

pub type WebFetcherType = Arc<Box<dyn IHttpRequester + Send + Sync + 'static>>;

pub trait IHttpRequester {
    fn request_url(&self, url: &str) -> HttpGetResult;
    fn request_url_bin(&self, url: &str) -> HttpGetResult;
}

// TODO make 2 values
#[derive(Debug, Default)]
pub struct HttpGetResult {
    pub http_status: i16,
    pub http_err_val: i16,

    pub content: String,
    pub content_bin: Vec<u8>,

    //
    // #[deprecated]
    // pub status: usize,
    //
    //
    pub error_description: String,
    pub timestamp: i64,
    pub content_length: i64,
}

impl HttpGetResult {

/*
    #[deprecated]
    pub fn get_status(&self) -> u16 {
        (self.status & 0xffff) as u16
    }
    #[deprecated]
    pub fn get_kind(&self) -> u8 {
        (self.status >> 16) as u8
    }
 */
    #[deprecated]
    pub fn combine_status(status: u16, kind: u8) -> usize {
        status as usize | ((kind as usize) << 16)
    }
}
