use fr_core::downloader::util::extract_feed_from_website;

//  TODO:  store homepage texts static, no more remote

const HTML_BASE: &str = "../fr_core/tests/websites/";

#[ignore]
#[test]
fn extract_url_work() {
    setup();
    let pairs: [(&str, &str, &str); 1] = [
	("stackexchange.html",
	 "https://unix.stackexchange.com/questions/457584/gtk3-change-text-color-in-a-label-raspberry-pi",
	 "https://unix.stackexchange.com/feeds/question/457584")

	];

    for (file, req_page, url) in pairs {
        let fname = format!("{}{}", HTML_BASE, file);
        let o_page = std::fs::read_to_string(fname.clone());
        if o_page.is_err() {
            error!("{}  {:?}", &fname, &o_page.err());
            continue;
        }
        let page = o_page.unwrap();
        let r = extract_feed_from_website(&page, &req_page);
        // info!("{:?}", r);
        assert_eq!(r, Ok(url.to_string()));
    }
}

#[ignore]
#[test]
fn extract_feed_urls_ok() {
    setup();
    let pairs: [(&str, &str, &str); 3] = [
        (
            "hp_neopr.html",
            "https://www.neopresse.com/politik/teile-der-afd-fordern-atomwaffen-fuer-deutschland/",
            "https://www.neopresse.com/feed/",
        ),
        (
            "pleiteticker.html",
            "https://pleiteticker.de/dkg-chef-gass-warnt-vor-winter-der-krankenhaus-insolvenzen/",
            "https://pleiteticker.de/feed/",
        ),
		(
		 "stackexchange.html",
		 "https://unix.stackexchange.com/questions/457584/gtk3-change-text-color-in-a-label-raspberry-pi",
		 "https://unix.stackexchange.com/feeds/question/457584"
	    ),

    ];

    for (file, req_page, url) in pairs {
        let fname = format!("{}{}", HTML_BASE, file);
        let page = std::fs::read_to_string(fname).unwrap();
        let r = extract_feed_from_website(&page, &req_page);
        // info!("{:?}", r);
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
