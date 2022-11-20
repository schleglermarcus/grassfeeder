mod downloader_dummy;
mod logger_config;
mod tree_drag_common;

use chrono::DateTime;
use fr_core::db::errors_repo::ErrorEntry;
use fr_core::downloader::db_clean::filter_error_entries;
use fr_core::downloader::db_clean::MAX_ERROR_LINES_PER_SUBSCRIPTION;
use fr_core::util::timestamp_now;
use regex::Regex;

#[ignore]
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

// #[test]
#[allow(dead_code)]
fn strange_date_formats() {
    setup();
    let strangers: [&str; 2] = [
        "Fri, 19 Aug 2022 21:56:36 Europe/Dublin", // https://feeds.breakingnews.ie/bnworld
        "Fri, 19 Aug 2022  15:29:5 CST",           // https://www.naturalnews.com/rss.xml
    ];
    for s in strangers {
        let r = DateTime::parse_from_rfc2822(&s);
        let regex = Regex::new(r":(\d) ").unwrap();
        let date_replaced = regex.replace(&s, ":0$1 ");
        debug!(" {}	\t\t{:?}	\t\t{:?}", s, r, date_replaced);
    }
}

// ------------------------------------

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
