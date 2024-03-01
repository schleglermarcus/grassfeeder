use fr_core::db::message::MessageRow;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::TD_BASE;

//RUST_BACKTRACE=1 cargo watch -s "cargo test   downloader::messages::t_:feed_text_to_entries_tages  --lib -- --exact --nocapture "
// A date entry is not contained here
#[test]
fn feed_text_to_entries_tages() {
    let filename = format!("{}feeds/tagesschau.rdf", TD_BASE);
    let contents = std::fs::read_to_string(filename).unwrap();
    let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
        feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
    assert_eq!(new_list.len(), 46);
    assert_eq!(
        new_list.get(0).unwrap().post_id,
        "https://www.tagesschau.de/inland/regierungserklaerung-scholz-gipfeltreffen-103.html"
    );
}


// #[ignore]
#[test]
fn parse_blogger_af() {
    setup();
    let filename = format!("{}feeds/blogger-af.xml", TD_BASE);
    let rss_txt = std::fs::read_to_string(filename).unwrap(); // "../target/td/feeds/blogger-af.xml"
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
    let filename = format!("{}feeds/blogger-pirates.xml", TD_BASE);
    let rss_txt = std::fs::read_to_string(filename).unwrap(); //  "../target/td/feeds/blogger-pirates.xml"
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
mod unzipper;

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
        unzipper::unzip_some();
    });
}
