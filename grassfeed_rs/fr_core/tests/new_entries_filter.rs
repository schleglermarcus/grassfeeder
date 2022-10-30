mod logger_config;

use feed_rs::parser;
use flume::Receiver;
use flume::Sender;
use fr_core::config::init_system::GrassFeederConfig;
use fr_core::controller::contentlist::match_new_entries_to_existing;
use fr_core::controller::contentlist::message_from_modelentry;
use fr_core::controller::contentlist::CJob;
use fr_core::controller::contentlist::FeedContents;
use fr_core::controller::contentlist::IFeedContents;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::downloader::util::workaround_https_declaration;
use fr_core::util;
use std::cell::RefCell;
use std::rc::Rc;

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

// #[ignore]
#[test]
fn parse_linuxcompati() {
    setup();
    let rss_str: String = std::fs::read_to_string("../target/td/feeds/linuxcomp_notitle.xml").unwrap();
    let feed_result = parser::parse(workaround_https_declaration(rss_str).as_bytes());
    if feed_result.is_err() {
        warn!("Err={:?}", feed_result.err());
        assert!(false);
        return;
    }
    let feeds = feed_result.unwrap();
    let list: Vec<MessageRow> = feeds
        .entries
        .iter()
        .map(|fe| message_from_modelentry(&fe).0)
        .collect();
    assert_eq!(list.len(), 1);
}

// test if feed update content matching works
// #[ignore]
#[test]
fn test_new_entries_filter() {
    setup();
    let gf_conf = GrassFeederConfig {
        path_config: "../target/db_entries_filter".to_string(),
        path_cache: "../target/db_entries_filter".to_string(),
        debug_mode: true,
        version: "db_entries_filter".to_string(),
    };
    let appcontext = fr_core::config::init_system::start(gf_conf);
    let feedcontents_r = appcontext.get_rc::<FeedContents>().unwrap();
    let msg_repo_r: Rc<RefCell<dyn IMessagesRepo>> = appcontext.get_rc::<MessagesRepo>().unwrap();
    let _r = (*msg_repo_r).borrow().get_ctx().delete_table();
    (*msg_repo_r).borrow().get_ctx().create_table();
    let mut existing: Vec<MessageRow> = Vec::default();
    let source_repo_id = 5;
    let timestamp_now = util::timestamp_now();
    let mut fce0 = MessageRow::new();
    fce0.subscription_id = source_repo_id;
    fce0.title = "Monday".to_string();
    fce0.post_id = "0x10".to_string();
    fce0.entry_src_date = timestamp_now;
    existing.push(fce0.clone());
    let mut fce1 = MessageRow::new();
    fce1.subscription_id = source_repo_id;
    fce1.title = "Tuesday".to_string();
    fce1.post_id = "0x20".to_string();
    fce1.entry_src_date = timestamp_now + 1;
    existing.push(fce1.clone());
    let mut fce2 = MessageRow::new();
    fce2.subscription_id = source_repo_id;
    fce2.title = "Wednesday".to_string();
    fce2.post_id = "0x30".to_string();
    fce2.entry_src_date = timestamp_now + 3;
    existing.push(fce2.clone());
    let _r = (*msg_repo_r).borrow().insert_tx(&existing);
    let job_receiver: Receiver<CJob> = (*feedcontents_r).borrow().get_job_receiver();
    let job_sender: Sender<CJob> = (*feedcontents_r).borrow().get_job_sender();
    // one entry new, that existed.   gives an empty insert list
    let mut new_list: Vec<MessageRow> = Vec::default();
    new_list.push(fce1.clone());
    let existing_entries = (*msg_repo_r).borrow().get_by_src_id(source_repo_id, false);
    assert_eq!(existing_entries.len(), 3);
    let insert_list =
        match_new_entries_to_existing(&new_list.to_vec(), &existing_entries, job_sender.clone());
    assert_eq!(insert_list.len(), 0);

    // one entry changed, only title change results in title update
    new_list.clear();
    let changed_title = "moon";
    fce0.title = changed_title.to_string();
    new_list.push(fce0);
    let insert_list =
        match_new_entries_to_existing(&new_list.to_vec(), &existing_entries, job_sender.clone());
    assert_eq!(insert_list.len(), 0);
    match job_receiver.recv().unwrap() {
        CJob::DbUpdateTitle(id, title) => {
            assert_eq!(id, 1);
            assert_eq!(title, changed_title);
        }
        _ => unimplemented!(),
    }
    //   two items changed
    let changed_post_id = "7411".to_string();
    let changed_timestamp = timestamp_now + 5;
    new_list.clear();
    fce1.post_id = changed_post_id.clone();
    new_list.push(fce1);
    fce2.entry_src_date = changed_timestamp;
    new_list.push(fce2);
    let insert_list =
        match_new_entries_to_existing(&new_list.to_vec(), &existing_entries, job_sender.clone());

    assert_eq!(insert_list.len(), 0);
    match job_receiver.recv().unwrap() {
        CJob::DbUpdatePostId(id, ti) => {
            assert_eq!(id, 2);
            assert_eq!(ti, changed_post_id);
        }
        CJob::DbUpdateEntryDate(id, da) => {
            assert_eq!(id, 2);
            assert_eq!(da, changed_timestamp as u64);
        }
        _ => unimplemented!(),
    }
}

// #[ignore]
#[test]
fn test_feed_text_to_entries() {
    let filename = "../target/td/feeds/gui_proc_rss2_v1.rss";
    let contents = std::fs::read_to_string(filename).unwrap();
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let msgrepo_r: Rc<RefCell<dyn IMessagesRepo>> = Rc::new(RefCell::new(msgrepo));
    let source_repo_id = 5;
    let (new_list, _num, _err_txt) =
        feed_text_to_entries(contents.clone(), source_repo_id, "some-url".to_string());
    let _r = (*msgrepo_r).borrow().insert_tx(&new_list);
    let r_list = (*msgrepo_r).borrow().get_by_src_id(source_repo_id, true);
    assert_eq!(r_list.len(), 2);
}

// #[ignore]
#[test]
fn parse_wissensmanufaktur() {
    setup();
    let rss_str: String = std::fs::read_to_string("../target/td/feeds/wissensmanufaktur_rss.xml").unwrap();
    let feeds = parser::parse(rss_str.as_bytes()).unwrap();
    let mut fce_list: Vec<MessageRow> = feeds
        .entries
        .iter()
        .map(|fe| message_from_modelentry(&fe).0)
        .collect();
    let msg18 = fce_list.get_mut(18).unwrap();
    assert_eq!(
        msg18.title,
        "Wer bildet Deine Meinung? Grundlagen der Manipulation – Rico Albrecht / Francine Weidlich"
            .to_string()
    );
}

// #[ignore]
#[test]
fn parse_youtube() {
    setup();
    let rss_str: String = std::fs::read_to_string("../target/td/feeds/suspiciousobservers.xml").unwrap();
    let feeds = parser::parse(rss_str.as_bytes()).unwrap();
    let mut fce_list: Vec<MessageRow> = feeds
        .entries
        .iter()
        .map(|fe| message_from_modelentry(&fe).0)
        .collect();
    let msg0 = fce_list.get_mut(0).unwrap();
    // debug!("msg0={:?}", msg0.content_text);
    assert!(msg0.content_text.len() > 2);
}

// ------------------------------------

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
