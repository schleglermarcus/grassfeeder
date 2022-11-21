use fr_core::db::errors_repo::ErrorEntry;
use fr_core::db::message::MessageRow;
use fr_core::downloader::db_clean::filter_error_entries;
use fr_core::downloader::db_clean::MAX_ERROR_LINES_PER_SUBSCRIPTION;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::downloader::util::extract_icon_from_homepage;
use fr_core::util::timestamp_now;
use fr_core::TD_BASE;

#[test]
fn db_errorlist_filter() {
    setup();
    let date_now = timestamp_now();
    let mut err_list: Vec<ErrorEntry> = Vec::default();
    for i in 0..10 {
        err_list.push(ErrorEntry {
            err_id: i * 100,
            subs_id: i,
            date: date_now - i as i64 * 10000000,
            err_code: 0,
            remote_address: String::default(),
            text: String::default(),
        });
    }
    let (result, _msg) = filter_error_entries(&err_list, Vec::default());
    assert_eq!(result.len(), 4);
    let mut err_list: Vec<ErrorEntry> = Vec::default();
    for i in 0..MAX_ERROR_LINES_PER_SUBSCRIPTION * 2 {
        err_list.push(ErrorEntry {
            err_id: i as isize,
            subs_id: 3,
            date: date_now + i as i64,
            err_code: 0,
            remote_address: String::default(),
            text: String::default(),
        });
    }
    let (result, _msg) = filter_error_entries(&err_list, Vec::default());
    // debug!("before:{}   after:{}", err_list.len(), result.len());
    assert_eq!(result.len(), MAX_ERROR_LINES_PER_SUBSCRIPTION);
}

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
fn test_extract_icon_relay_rd() {
    setup();
    let filename = format!("{}websites/relay_rd.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &"https://www.relay.fm/rd".to_string());
    assert_eq!(
        r,
        Ok(
            "https://www.relay.fm/assets/favicon-fd28d8fa5c60ac2860b452a36991933e905f82f1349c4a5ad171dd0586b2b331.ico"
                .to_string()
        )
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
