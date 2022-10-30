use fr_core::db::message::MessageRow;
use fr_core::downloader::messages::feed_text_to_entries;


// #[ignore]
#[test]
fn parse_blogger_af() {
    setup();
    let rss_txt = std::fs::read_to_string("../target/td/feeds/blogger-af.xml").unwrap();
    let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
        feed_text_to_entries(rss_txt, 6, "some-url".to_string());
    let e0: &MessageRow = new_list.get(0).unwrap();
    assert_eq!(
        e0.link.as_str(),
        "http://antifeministsite.blogspot.com/2022/09/husband-murdered-because-of-rotten-wife.html"
    );
}

// #[ignore]
#[test]
fn parse_blogger_pirat() {
    setup();
    let rss_txt = std::fs::read_to_string("../target/td/feeds/blogger-pirates.xml").unwrap();
    let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
        feed_text_to_entries(rss_txt, 6, "some-url".to_string());
    let e0: &MessageRow = new_list.get(0).unwrap();
    assert_eq!(
        e0.link.as_str(),
        "http://stopthepirates.blogspot.com/2021/07/traffic-court-procedure-basics.html"
    );
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
        let _r = logger_config::setup_fern_logger(
            logger_config::QuietFlags::Config as u64 | logger_config::QuietFlags::Db as u64,
        );
    });
}
