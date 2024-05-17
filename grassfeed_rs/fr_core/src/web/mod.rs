pub mod httpfetcher;
pub mod mockfilefetcher;

use std::sync::Arc;

pub type WebFetcherType = Arc<Box<dyn IHttpRequester + Send + Sync + 'static>>;

pub trait IHttpRequester {
    fn request_url(&self, url: &str ) -> HttpGetResult;
    fn request_url_bin(&self, url: &str) -> HttpGetResult;
}

#[derive(Debug, Default)]
pub struct HttpGetResult {
    pub content: String,
    pub content_bin: Vec<u8>,
    pub status: usize,
    pub error_description: String,
}

impl HttpGetResult {
    pub fn get_status(&self) -> u16 {
        (self.status & 0xffff) as u16
    }
    pub fn get_kind(&self) -> u8 {
        (self.status >> 16) as u8
    }
    //fn set_status(&mut self, status: u16, kind: u8) {        self.status = status as usize | (kind << 16) as usize;    }
    pub fn combine_status(status: u16, kind: u8) -> usize {
        status as usize | ((kind as usize) << 16)
    }
}
