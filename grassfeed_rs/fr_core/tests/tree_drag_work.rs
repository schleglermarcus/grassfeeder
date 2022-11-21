mod downloader_dummy;
mod logger_config;
mod tree_drag_common;

use chrono::DateTime;
use fr_core::db::errors_repo::ErrorEntry;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::downloader::db_clean::filter_error_entries;
use regex::Regex;

#[ignore]
#[test]
fn db_errorlist_filter() {
    setup();
    let err_repo = ErrorRepo::new("../fr_core/tests/data/"); // errors.json.txt

    let err_list: Vec<ErrorEntry> = err_repo.get_all_stored_entries();
    let (result, msg) = filter_error_entries(&err_list, Vec::default());

    debug!("before:{}   after:{}", err_list.len(), result.len());
    debug!("{}", msg);

    // let err_repo = ErrorRepo::new("../target/");

    err_repo.store_all_to_file(result, "../target/err_filtered.txt");
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
