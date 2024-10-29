pub mod httpfetcher;
pub mod mockfilefetcher;

use std::sync::Arc;

pub type WebFetcherType = Arc<Box<dyn IHttpRequester + Send + Sync + 'static>>;

pub trait IHttpRequester {
    fn request_url(&self, url: &str) -> HttpGetResult;
    fn request_url_bin(&self, url: &str) -> HttpGetResult;
}

#[derive(Debug, Default)]
pub struct HttpGetResult {
    pub http_status: i16,
    pub http_err_val: i16,
    pub content: String,
    pub content_bin: Vec<u8>,
    pub error_description: String,
    pub timestamp: i64,
    pub content_length: i64,
}
