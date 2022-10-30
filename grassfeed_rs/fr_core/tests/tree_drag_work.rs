mod downloader_dummy;
mod logger_config;
mod tree_drag_common;

use chrono::DateTime;
use regex::Regex;

/*
use fr_core::db::message::MessageRow;
use fr_core::downloader::messages::feed_text_to_entries;

//RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_::dl_entries_breakingnews    --lib -- --exact --nocapture "
/// Timestamp delivered   from    https://feeds.breakingnews.ie/bnworld
/// https://www.w3.org/Protocols/rfc822/#z28
// #[ignore]
#[test]
//  #[allow(dead_code)]
fn dl_entries_breakingnews_cmp() {
    setup();
    let filenames = [
        "tests/data/gui_proc_v2.rss",
        "tests/data/breakingnewsworld-2.xml",
    ];
    for filename in filenames {
        debug!("FILE={}", filename);
        let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) = feed_text_to_entries(
            std::fs::read_to_string(filename).unwrap(),
            5,
            "some-url".to_string(),
        );
        let pubdate = new_list.get(0).unwrap().entry_src_date;
        assert!(pubdate > 0);
    }
}
*/

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
