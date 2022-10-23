use fr_core::downloader::util::extract_feed_from_homepage;

//  TODO:  store homepage texts static, no more remote

const HTML_BASE: &str = "../fr_core/tests/websites/";

// #[ignore]
#[test]
fn extract_feed_urls() {
    setup();
    let pairs: [(&str, &str); 1] = [("hp_neopr.html", "https://www.neopresse.com/feed/")];

    for (file, url) in pairs {
        let fname = format!("{}{}", HTML_BASE, file);
        let page = std::fs::read_to_string(fname).unwrap();
        let r = extract_feed_from_homepage(page);
        info!("{:?}", r);
        assert_eq!(r, Ok(url.to_string()));
    }
}

// ------------------------------------

mod logger_config;
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(0);
    });
}
