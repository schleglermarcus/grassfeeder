use crate::web::HttpGetResult;
use crate::web::IHttpRequester;
use std::io::Read;
use ureq::ErrorKind;

const MAX_BUFFER_LENGTH: u64 = 1000000;
const NO_CONTENTLENGTH_BUFFER_SIZE: u64 = 50000;

// https://developer.mozilla.org/en-US/docs/Web/HTTP/Redirections
pub struct HttpFetcher {}
impl HttpFetcher {
    fn request_url(&self, url: String, is_binary: bool) -> HttpGetResult {
        let mut r_text = String::default();
        let mut r_status: u16 = 0;
        let mut r_errorkind: u8 = 0;
        let mut r_ed = String::default();
        let mut r_bytes: Vec<u8> = Vec::default();
        let agent = ureq::builder().user_agent("ferris/1.0").build();
        match agent.get(&url).call() {
            Ok(response) => {
                r_status = response.status() as u16;
                if is_binary {
                    let mut length: u64 = 0;
                    if let Some(h_cole) = response.header("Content-Length") {
                        length = h_cole.parse().unwrap();
                        // trace!("HttpFetcher:{}  Header content-length={}", url, h_cole);
                    }
                    if length > 0 {
                        length = std::cmp::min(length, MAX_BUFFER_LENGTH);
                    } else {
                        // trace!("HttpFetcher:  NO content-length , using maximum !");
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
                        Err(e) => warn!("HttpFetcher: {} response.into_string {:?}", url, e),
                    }
                }
            }
            Err(ureq::Error::Status(status, response)) => {
                r_status = status as u16;
                r_ed = response.status_text().to_string();
            }
            Err(ureq::Error::Transport(transp)) => {
                r_errorkind = ureq_error_kind_to_u8(transp.kind()) as u8;
                r_ed = format!(
                    "{:?} {}",
                    transp.kind(),
                    // match transp.message() {
                    //     Some(s) => s,
                    //     None => "",
                    // }
                    transp.message().unwrap_or("")
                );
            }
        }
        HttpGetResult {
            content: r_text,
            content_bin: r_bytes,
            status: HttpGetResult::combine_status(r_status, r_errorkind),
            error_description: r_ed,
        }
    }
}

impl IHttpRequester for HttpFetcher {
    fn request_url(&self, url: String) -> HttpGetResult {
        self.request_url(url, false)
    }
    fn request_url_bin(&self, url: String) -> HttpGetResult {
        self.request_url(url, true)
    }
}

pub fn ureq_error_kind_to_u8(e: ErrorKind) -> u8 {
    UREQ_ERRORKIND_LIST.iter().position(|&x| x == e).unwrap() as u8
}

const UREQ_ERRORKIND_LIST: [ErrorKind; 12] = [
    ErrorKind::HTTP,
    ErrorKind::InvalidUrl,
    ErrorKind::UnknownScheme,
    ErrorKind::Dns,
    ErrorKind::ConnectionFailed,
    ErrorKind::TooManyRedirects,
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

    // Lieferte Zeitweise 502, jetzt wieder 200
    #[ignore]
    #[test]
    fn test_502() {
        let r =
            prep_fetcher().request_url("https://lib.rs/development-tools/debugging".to_string());
        assert_eq!(r.get_status(), 502);
        assert_eq!(r.error_description, "Bad Gateway");
    }

    // #[ignore]
    #[test]
    fn test_local404() {
        let r = prep_fetcher().request_url("http://localhost::8123/nothing".to_string());
        assert_eq!(r.get_kind(), 1);
        assert!(r.error_description.contains("InvalidUrl"));
    }

    // #[ignore]
    #[test]
    fn test_remote_200() {
        let r = prep_fetcher()
            .request_url("https://www.heise.de/icons/ho/topnavi/nopur.gif".to_string());
        assert_eq!(r.get_status(), 200);
    }

    // #[ignore]
    // #[test]
    #[allow(dead_code)]
    fn test_remote_403() {
        let r = prep_fetcher().request_url("https://static.foxnews.com/unknown.png".to_string());
        assert_eq!(r.get_status(), 403);
    }

    // #[ignore]
    #[test]
    fn test_remote_connect() {
        let r = prep_fetcher()
            .request_url("https://www.hyundai-kefico.com/en/main/index.do".to_string());
        assert_eq!(r.get_kind(), 4);
        assert!(r.error_description.contains("ConnectionFailed"));
    }

    // #[ignore]
    #[test]
    fn test_remote_404() {
        let r = prep_fetcher()
            .request_url_bin("https://www.heise.de/icons/ho/touch-icons/none.png".to_string());
        //    debug!("R={:?}", &r);
        assert_eq!(r.get_status(), 404);
    }

    //  cargo test  web::httpfetcher::httpfetcher_t::test_remote_kodansha --lib -- --exact --nocapture
    #[test]
    fn test_remote_kodansha() {
        let r = prep_fetcher().request_url_bin("https://kodansha.us/favicon.ico".to_string());
        // dbg!(&r);
        assert_eq!(r.get_status(), 200);
    }

    /*
        //  cargo test  web::httpfetcher::httpfetcher_t::test_remote_redirect --lib -- --exact --nocapture
        #[test]
        fn test_remote_redirect() {
            // setup();
            let r = prep_fetcher().request_url_bin("https://report24.news/favicon.ico".to_string());
            // dbg!(&r);
            assert_eq!(r.get_status(), 302);
        }
    */
}
