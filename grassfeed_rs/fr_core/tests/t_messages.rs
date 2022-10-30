use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::TD_BASE;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn feed_text_to_entries_local() {
    setup();
    let filename = format!("{}feeds/gui_proc_rss2_v1.rss", TD_BASE); //  "../target/td/feeds/gui_proc_rss2_v1.rss";
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

/*
#[ignore]
#[test]
fn test_feed_text_to_entries() {
    setup();
    let filename = format!("{}feeds/gui_proc_rss2_v1.rss", TD_BASE); //  "../target/td/feeds/gui_proc_rss2_v1.rss";
    let contents = std::fs::read_to_string(filename).unwrap();
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let source_repo_id = 5;
    let msgrepo_r: Rc<RefCell<dyn IMessagesRepo>> = Rc::new(RefCell::new(msgrepo));
    let (new_list, _num, _err_txt) =
        feed_text_to_entries(contents.clone(), source_repo_id, "some-url".to_string());
    let source_repo_id = 5;
    let _r = (*msgrepo_r).borrow().insert_tx(&new_list);
    let r_list = (*msgrepo_r).borrow().get_by_src_id(source_repo_id, true);
    assert_eq!(r_list.len(), 2);
}
*/

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
    });
    //   unzipper::unzip_some();
    debug!("UNZIPPED: {}", unzipper::unzip_some());
}
