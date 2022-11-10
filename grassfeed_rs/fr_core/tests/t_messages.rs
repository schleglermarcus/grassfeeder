use fr_core::db::message::decompress;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::downloader::messages::strange_datetime_recover;
use fr_core::TD_BASE;
use std::cell::RefCell;
use std::rc::Rc;

// RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_:feed_text_to_entries_xkcd  --lib -- --exact --nocapture "
#[test]
fn feed_text_to_entries_xkcd() {
    setup();
    // let filename = "tests/data/xkcd_atom.xml";
    let filename = format!("{}feeds/xkcd_atom.xml", TD_BASE);
    let contents = std::fs::read_to_string(filename).unwrap();
    let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
        feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
    assert_eq!(new_list.len(), 4);
}

//RUST_BACKTRACE=1 cargo watch -s "cargo test   downloader::messages::t_::feed_text_to_entries_naturalnews  --lib -- --exact --nocapture   "
#[test]
fn feed_text_to_entries_naturalnews() {
    setup();
    let filename = format!("{}feeds/naturalnews_rss.xml", TD_BASE);
    let contents = std::fs::read_to_string(filename).unwrap();
    let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
        feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
    assert_eq!(new_list.len(), 30);
    assert_eq!(new_list[1].entry_src_date, 1655877600);
    assert_eq!(new_list[2].entry_src_date, 1655877600);
    // new_list.iter().enumerate().for_each(|(n, le)| {        debug!("{} \t {}", n, decompress(&le.title));    });
    assert_eq!( decompress (&new_list.get(10).unwrap().title),
	   "White House press secretary ripped for claiming it'll take days to count ballots so many winners won't be immediately known".to_string()   );
}

//RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_::t_strange_datetime_recover    --lib -- --exact --nocapture "
#[test]
fn t_strange_datetime_recover() {
    setup();
    let filename = format!("{}feeds/naturalnews_rss.xml", TD_BASE);
    let mtext = std::fs::read_to_string(filename).unwrap();
    let (mut new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
        feed_text_to_entries(mtext.clone(), 5, "some-url".to_string());
    // debug!("prev:   {:?}", new_list[0].entry_src_date);
    let o_msg = strange_datetime_recover(&mut new_list, &mtext);
    assert!(o_msg.is_none());
    assert_eq!(new_list[0].entry_src_date, 1655935140)
}

#[test]
fn feed_text_to_entries_local() {
    setup();
    let filename = format!("{}feeds/gui_proc_rss2_v1.rss", TD_BASE);
    let contents = std::fs::read_to_string(filename).unwrap();
    let (new_list, ts_created, _err): (Vec<MessageRow>, i64, String) =
        feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
    assert_eq!(new_list.len(), 2);
    assert_eq!(ts_created, 1636573888);
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let msgrepo_r: Rc<RefCell<dyn IMessagesRepo>> = Rc::new(RefCell::new(msgrepo));
    let source_repo_id = 5;
    let _r = (*msgrepo_r).borrow().insert_tx(&new_list);
    let r_list = (*msgrepo_r).borrow().get_by_src_id(source_repo_id, true);
    assert_eq!(r_list.len(), 2);
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
        let _r = logger_config::setup_fern_logger(logger_config::QuietFlags::Controller as u64);
        unzipper::unzip_some();
    });
}
