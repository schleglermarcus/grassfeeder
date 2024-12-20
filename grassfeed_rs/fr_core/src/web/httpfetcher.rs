use crate::web::HttpGetResult;
use crate::web::IHttpRequester;
use chrono::DateTime;
use chrono::Local;
use std::io::Read;
use ureq::ErrorKind;

const MAX_BUFFER_LENGTH: u64 = 1000000;
const NO_CONTENTLENGTH_BUFFER_SIZE: u64 = 1000000;

// https://developer.mozilla.org/en-US/docs/Web/HTTP/Redirections

pub struct HttpFetcher {}
impl HttpFetcher {
    /// heap:   ureq::response::into_string() consumes about  20% of overall memory usage
    fn request_url(&self, url: &str, is_binary: bool) -> HttpGetResult {
        let mut r_text = String::default();
        let mut r_status: u16 = 0;
        let mut r_errorkind: u8 = 0;
        let mut r_ed = String::default();
        let mut r_bytes: Vec<u8> = Vec::default();
        let mut web_content_length: i64 = -1;
        let mut web_last_modified: i64 = -1;
        let agent = ureq::builder().user_agent("ferris/1.0").build();
        match agent.get(url).call() {
            Ok(response) => {
                r_status = response.status();
                if let Some(l_mod_str) = response.header("Last-Modified") {
                    match DateTime::parse_from_rfc2822(l_mod_str) {
                        Ok(parse_dt) => {
                            web_last_modified = DateTime::<Local>::from(parse_dt).timestamp();
                        }
                        Err(e) => {
                            r_ed = format!("Error parse_from_rfc2822(`{}`) {:?} ", l_mod_str, &e);
                        }
                    };
                }

                if is_binary {
                    let mut length: u64 = 0;
                    if let Some(h_cole) = response.header("Content-Length") {
                        length = h_cole.parse().unwrap();
                        web_content_length = length as i64;
                    }
                    if length > 0 {
                        length = std::cmp::min(length, MAX_BUFFER_LENGTH);
                    } else {
                        // trace!("HttpFetcher:  NO content-length , using maximum ! {} ", url);
                        length = NO_CONTENTLENGTH_BUFFER_SIZE;
                    }
                    r_bytes = Vec::with_capacity(length as usize);
                    match response
                        .into_reader()
                        .take(length)
                        .read_to_end(&mut r_bytes)
                    {
                        Ok(bytes) => {
                            r_bytes.truncate(bytes);
                        }
                        Err(e) => {
                            error!("HttpFetcher: {} read_to_end {:?}", url, e);
                            r_bytes = Vec::default();
                        }
                    }
                } else {
                    match response.into_string() {
                        Ok(r_str) => r_text = r_str,
                        Err(e) => {
                            warn!("HttpFetcher: {} response.into_string {:?}", url, e);
                            r_errorkind = 14;
                            r_ed = format!("ResponseError: {e:?}");
                        }
                    }
                }
            }
            Err(ureq::Error::Status(status, response)) => {
                r_status = status;
                r_ed = response.status_text().to_string();
            }
            Err(ureq::Error::Transport(transp)) => {
                r_errorkind = ureq_error_kind_to_u8(transp.kind());
                r_ed = format!(
                    "transport:{:?}  {}",
                    transp.kind(),
                    transp.message().unwrap_or("")
                );
            }
        }
        HttpGetResult {
            content: r_text,
            content_bin: r_bytes,
            http_status: r_status as i16,
            http_err_val: r_errorkind as i16,
            error_description: r_ed,
            content_length: web_content_length,
            timestamp: web_last_modified,
        }
    }
}

impl IHttpRequester for HttpFetcher {
    fn request_url(&self, url: &str) -> HttpGetResult {
        self.request_url(url, false)
    }
    fn request_url_bin(&self, url: &str) -> HttpGetResult {
        self.request_url(url, true)
    }
}

pub fn ureq_error_kind_to_u8(e: ErrorKind) -> u8 {
    UREQ_ERRORKIND_LIST.iter().position(|&x| x == e).unwrap() as u8
}

const UREQ_ERRORKIND_LIST: [ErrorKind; 12] = [
    ErrorKind::HTTP, // 0
    ErrorKind::InvalidUrl,
    ErrorKind::UnknownScheme,
    ErrorKind::Dns,
    ErrorKind::ConnectionFailed, // 4
    ErrorKind::TooManyRedirects, // 5
    ErrorKind::BadStatus,
    ErrorKind::BadHeader,
    ErrorKind::Io,
    ErrorKind::InvalidProxyUrl,
    ErrorKind::ProxyConnect,
    ErrorKind::ProxyUnauthorized,
];

#[cfg(test)]
mod httpfetcher_t {

    use super::*;

    fn prep_fetcher() -> impl IHttpRequester {
        HttpFetcher {}
    }

    #[test]
    fn test_local404() {
        let r = prep_fetcher().request_url("http://localhost::8123/nothing");
        assert_eq!(r.http_err_val, 1);
        assert!(r.error_description.contains("InvalidUrl"));
    }

    #[test]
    fn test_remote_200() {
        let r = prep_fetcher().request_url("https://www.heise.de/icons/ho/topnavi/nopur.gif");
        assert_eq!(r.http_status, 200);
    }

    #[test]
    fn test_remote_403() {
        let r = prep_fetcher().request_url("https://static.foxnews.com/unknown.png");
        assert_eq!(r.http_status, 403);
    }

    #[test]
    fn test_remote_connect() {
        let r = prep_fetcher().request_url("https://www.hyundai-kefico.com/en/main/index.do");
        assert_eq!(r.http_err_val, 4);
        assert!(r.error_description.contains("ConnectionFailed"));
    }

    #[test]
    fn test_remote_404() {
        let r =
            prep_fetcher().request_url_bin("https://www.heise.de/icons/ho/touch-icons/none.png");
        assert_eq!(r.http_status, 404);
    }

    //  cargo test  web::httpfetcher::httpfetcher_t::test_remote_kodansha --lib -- --exact --nocapture
    #[test]
    fn test_remote_kodansha() {
        let r = prep_fetcher().request_url_bin("https://kodansha.us/favicon.ico");
        assert_eq!(r.http_status, 200);
    }
}
