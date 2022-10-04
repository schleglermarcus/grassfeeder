mod downloader_dummy;
mod logger_config;
mod tree_drag_common;

// use crate::tree_drag_common::dataset_three_folders;
// use crate::tree_drag_common::prepare_source_tree_controller;

use chrono::DateTime;
use feed_rs::parser;
use fr_core::controller::contentlist;
use fr_core::db::errors_repo::ErrorEntry;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::message::MessageRow;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::util::db_time_to_display_nonnull;
use regex::Regex;

#[test]
fn t_error_repo_store() {
    setup();
    let mut e_repo = ErrorRepo::new("../target/err_rep/");
    e_repo.startup_read();
    let mut e1 = ErrorEntry::default();
    e1.text = "Hello!".to_string();
    e_repo.add_error(
        5,
        0,
        "https://www.youtube.com/feeds/videos.xml?channel_id=UC7nMSUJjOr7_TEo95Koudbg".to_string(),
        "some y message".to_string(),
    );
    e_repo.check_or_store();

    let list = e_repo.get_by_subscription(5);
    debug!("LIST={:#?}", list);
    for ee in list {
        debug!("{}", ee.to_line("feed-name".to_string()));
    }
}

//  Maybe later:
//  The file contains an invalid  single  &  as title.   The parse does not like that and returns  no title.
#[allow(dead_code)]
fn parse_with_ampersand() {
    let rss_str = std::fs::read_to_string("../testing/tests/fr_htdocs/dieneuewelle.xml").unwrap();
    let feeds = parser::parse(rss_str.as_bytes()).unwrap();
    let entry2 = feeds.entries.get(2).unwrap();
    let msg2: MessageRow = contentlist::message_from_modelentry(&entry2).0;
    assert!(msg2.title.starts_with("Borderlands-"));
}

// #[test]
#[allow(dead_code)]
fn parse_naturalnews_aug() {
    let rss_str =
        std::fs::read_to_string("../testing/tests/fr_htdocs/naturalnews_aug.xml").unwrap();
    let feeds = parser::parse(rss_str.as_bytes()).unwrap();
    let entry0 = feeds.entries.get(0).unwrap();
    let msg0: MessageRow = contentlist::message_from_modelentry(&entry0).0;
    println!("title={}=", msg0.title);
}

//  #[test]
#[allow(dead_code)]
fn dl_naturalnews() {
    setup();
    let (new_list, ts_created, err): (Vec<MessageRow>, i64, String) = feed_text_to_entries(
        std::fs::read_to_string("tests/data/naturalnews_rss.xml").unwrap(),
        6,
        "some-url".to_string(),
    );
    debug!("ts_created={:?}  err={:?}", ts_created, err);
    debug!("list={:?}", new_list.len());
    // for entry in new_list {        debug!("date={:?}", db_time_to_display_nonnull(entry.entry_src_date));    }
    let e0: &MessageRow = new_list.get(0).unwrap();
    debug!("date={:?}  ", db_time_to_display_nonnull(e0.entry_src_date));
}

//RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_::dl_entries_breakingnews    --lib -- --exact --nocapture "
/// Timestamp delivered   from    https://feeds.breakingnews.ie/bnworld
/// https://www.w3.org/Protocols/rfc822/#z28
// #[ignore]
// #[test]
#[allow(dead_code)]
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
