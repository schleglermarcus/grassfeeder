mod logger_config;

use chrono::DateTime;
use context::appcontext::AppContext;
use feed_rs::parser;
use fr_core::config::configmanager::ConfigManager;
use fr_core::config::prepare_ini::prepare_config_by_path;
use fr_core::config::prepare_ini::GrassFeederConfig;
use fr_core::controller::browserpane::BrowserPane;
use fr_core::controller::contentdownloader::Downloader;
use fr_core::controller::contentlist::message_from_modelentry;
use fr_core::controller::contentlist::CJob;
use fr_core::controller::contentlist::FeedContents;
use fr_core::controller::contentlist::IFeedContents;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::timer::Timer;
use fr_core::ui_select::gui_context::GuiContext;
use fr_core::util;
use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;

// test if feed update content matching works
// #[ignore]
#[test]
fn test_new_entries_filter() {
    setup();
    let gfc = GrassFeederConfig {
        path_config: "../target/db_entries_filter".to_string(),
        path_cache: "../target/db_entries_filter".to_string(),
        debug_mode: true,
		version: "test_new_entries_filter".to_string(),
    };
    // "../target/db_entries_filter".to_string(),         "../target/db_entries_filter".to_string(),
    let ini_r = Rc::new(RefCell::new(prepare_config_by_path(&gfc)));
    let mut appcontext = AppContext::new_with_ini(ini_r.clone());
    let mut cm = ConfigManager::new_with_ini(ini_r);
    cm.load_config_file();
    appcontext.store_ini(Rc::new(RefCell::new(cm.get_conf())));
    appcontext.store_obj(Rc::new(RefCell::new(cm)));
    appcontext.build::<Timer>();
    appcontext.build::<GuiContext>();
    appcontext.build::<SubscriptionRepo>();
    appcontext.build::<MessagesRepo>();
    appcontext.build::<IconRepo>();
    appcontext.build::<Downloader>();
    appcontext.build::<BrowserPane>();
    appcontext.build::<FeedContents>();
    let feedcontents_r = appcontext.get_rc::<FeedContents>().unwrap();

    let msg_repo_r: Rc<RefCell<dyn IMessagesRepo>> = appcontext.get_rc::<MessagesRepo>().unwrap();
    let _r = (*msg_repo_r).borrow().get_ctx().delete_table();
    (*msg_repo_r).borrow().get_ctx().create_table();
    let mut existing: Vec<MessageRow> = Vec::default();
    let source_repo_id = 5;
    let timestamp_now = util::timestamp_now();
    let mut fce0 = MessageRow::new();
    fce0.feed_src_id = source_repo_id;
    fce0.title = "Monday".to_string();
    fce0.post_id = "0x10".to_string();
    fce0.entry_src_date = timestamp_now;
    existing.push(fce0.clone());
    let mut fce1 = MessageRow::new();
    fce1.feed_src_id = source_repo_id;
    fce1.title = "Tuesday".to_string();
    fce1.post_id = "0x20".to_string();
    fce1.entry_src_date = timestamp_now + 1;
    existing.push(fce1.clone());
    let mut fce2 = MessageRow::new();
    fce2.feed_src_id = source_repo_id;
    fce2.title = "Wednesday".to_string();
    fce2.post_id = "0x30".to_string();
    fce2.entry_src_date = timestamp_now + 3;
    existing.push(fce2.clone());

    let _r = (*msg_repo_r).borrow().insert_tx(&existing);
    //debug!("    ALL={:#?}", &(*msg_repo_r).borrow().get_all_messages());
    let job_receiver = (*feedcontents_r).borrow().get_job_receiver();

    // one entry new, that existed
    let mut new_list: Vec<MessageRow> = Vec::default();
    new_list.push(fce1.clone());
    let insert_list = (*feedcontents_r)
        .borrow()
        .match_new_entries_to_db(&new_list, source_repo_id);
    assert_eq!(insert_list.len(), 0);

    // one entry changed
    new_list.clear();
    let changed_title = "moon";
    fce0.title = changed_title.to_string();
    new_list.push(fce0);
    let insert_list = (*feedcontents_r)
        .borrow()
        .match_new_entries_to_db(&new_list, source_repo_id);
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
    let insert_list = (*feedcontents_r)
        .borrow()
        .match_new_entries_to_db(&mut new_list, source_repo_id);
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
    let filename = "tests/data/gui_proc_rss2_v1.rss";
    let contents = std::fs::read_to_string(filename).unwrap();
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let msgrepo_r: Rc<RefCell<dyn IMessagesRepo>> = Rc::new(RefCell::new(msgrepo));
    let source_repo_id = 5;
    let (new_list, _num, _err_txt) =
        feed_text_to_entries(contents.clone(), source_repo_id, "some-url".to_string());
    let _r = (*msgrepo_r).borrow().insert_tx(&new_list);
    let r_list = (*msgrepo_r).borrow().get_by_src_id(source_repo_id);
    assert_eq!(r_list.len(), 2);
}

// #[ignore]
#[test]
fn parse_wissensmanufaktur() {
    setup();
    let rss_str: String = std::fs::read_to_string("tests/data/wissensmanufaktur_rss.xml").unwrap();
    let feeds = parser::parse(rss_str.as_bytes()).unwrap();

    let mut fce_list: Vec<MessageRow> = feeds
        .entries
        .iter()
        .map(|fe| message_from_modelentry(&fe))
        .collect();

    let msg18 = fce_list.get_mut(18).unwrap();
    assert_eq!(
        msg18.title,
        "Wer bildet Deine Meinung? Grundlagen der Manipulation – Rico Albrecht / Francine Weidlich"
            .to_string()
    );
}

#[test]
fn parse_youtube() {
    setup();
    let rss_str: String = std::fs::read_to_string("tests/data/suspiciousobservers.xml").unwrap();
    let feeds = parser::parse(rss_str.as_bytes()).unwrap();
    let mut fce_list: Vec<MessageRow> = feeds
        .entries
        .iter()
        .map(|fe| message_from_modelentry(&fe))
        .collect();
    let msg0 = fce_list.get_mut(0).unwrap();
    // debug!("msg0={:?}", msg0.content_text);
    assert!(msg0.content_text.len() > 2);
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
        let _r = logger_config::setup_fern_logger(logger_config::QuietFlags::Config as u64);
    });
}
