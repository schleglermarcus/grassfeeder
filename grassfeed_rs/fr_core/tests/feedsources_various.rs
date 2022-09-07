mod logger_config;

// mockall cannot provide a consistent data set, needs to be instrumented for each request separately.
mod downloader_dummy;

use crate::downloader_dummy::DownloaderDummy;
use chrono::DateTime;
use fr_core::config::configmanager::ConfigManager;
use fr_core::controller::contentdownloader::IDownloader;
use fr_core::controller::sourcetree::ISourceTreeController;
use fr_core::controller::sourcetree::SourceTreeController;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::icons::blob_is_icon;
use fr_core::downloader::messages::feed_text_to_entries;
use fr_core::timer::build_timer;
use fr_core::timer::ITimer;
use fr_core::ui_select::uimock::UIMock;
use fr_core::util::db_time_to_display_nonnull;
use fr_core::web::httpfetcher::HttpFetcher;
use fr_core::web::IHttpRequester;
use std::cell::RefCell;
use std::rc::Rc;

#[ignore]
#[test]
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
#[ignore]
#[test]
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

/// Timestamp delivered   from    https://feeds.breakingnews.ie/bnworld
/// https://www.w3.org/Protocols/rfc822/#z28
#[ignore]
#[test]
fn chrono_broken_timestamp() {
    setup();
    let broken_ts = "Fri, 05 Aug 2022 23:28:01 Europe/Dublin";
    let pars_res = DateTime::parse_from_rfc2822(&broken_ts);

    assert!(pars_res.is_err());
    assert_eq!(
        pars_res.err().unwrap().to_string(),
        "trailing input".to_string()
    );
    debug!("err {:?}", pars_res.err().unwrap().to_string());
}

// #[ignore]
#[test]
fn add_feed_with_existing() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (mut fsc, _r_fsource) = prepare_stc(fs_list);
    let msgrepo = MessagesRepo:: new_in_mem(); // new(":memory:".to_string());
    msgrepo.get_ctx().create_table();
    let mut mr1: MessageRow = MessageRow::default();
    mr1.feed_src_id = 20;
    let _mr1id = msgrepo.insert(&mr1).unwrap() as isize;
    let msg_r_r = Rc::new(RefCell::new(msgrepo));
    fsc.messagesrepo_w = Rc::downgrade(&msg_r_r);
    let new_id = fsc.add_new_feedsource("some-url-3".to_string(), "name-proc3".to_string());
    assert_eq!(new_id, mr1.feed_src_id + 1);
    // let fse = (*(r_fsource.borrow())).get_by_index(new_id).unwrap();    debug!("FSE={:?}", fse);
}

// #[ignore]
#[test]
fn add_feed_empty() {
    setup();
    let (mut fsc, r_fsource) = prepare_stc(Vec::default());
    fsc.add_new_feedsource(
        "tests/data/gui_proc_rss2_v1.rss".to_string(),
        "name-proc2".to_string(),
    );
    let entries = (*(r_fsource.borrow())).get_all_entries();
    assert_eq!(entries.len(), 1);
}

// #[ignore]
#[test]
fn delete_feed_v1() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (mut fsc, r_fsource) = prepare_stc(fs_list);
    fsc.set_fs_delete_id(Some(2));
    fsc.feedsource_move_to_trash();
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    assert_eq!(result.get(0).unwrap().folder_position, 0);
    assert_eq!(result.get(1).unwrap().folder_position, 1);
}

// #[ignore]
#[test]
fn update_folder_pos() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (_fsc, r_fsource) = prepare_stc(fs_list);
    r_fsource
        .borrow()
        .update_parent_and_folder_position(1, 22, 33);
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_all_entries();
    // result.iter().for_each(|fs| info!("  {}", fs));
    assert_eq!(result.get(0).unwrap().subs_id, 1);
    assert_eq!(result.get(0).unwrap().parent_subs_id, 22);
    assert_eq!(result.get(0).unwrap().folder_position, 33);
}

fn prepare_stc(
    fs_list: Vec<SubscriptionEntry>,
) -> (SourceTreeController, Rc<RefCell<dyn ISubscriptionRepo>>) {
    let subscrip_repo = SubscriptionRepo::new("");
    fs_list.iter().for_each(|e| {
        let _r = subscrip_repo.store_entry(e);
    });
    let r_subscriptions_repo: Rc<RefCell<dyn ISubscriptionRepo>> =
        Rc::new(RefCell::new(subscrip_repo));
    let r_timer: Rc<RefCell<dyn ITimer>> = Rc::new(RefCell::new(build_timer()));
    let uimock = UIMock::new();
    let downloaderdummy = DownloaderDummy::default();
    let r_dl: Rc<RefCell<dyn IDownloader>> = Rc::new(RefCell::new(downloaderdummy));
    let r_configmanager = Rc::new(RefCell::new(ConfigManager::default()));
    let r_icons_repo = Rc::new(RefCell::new(IconRepo::new("")));
    let fs = SourceTreeController::new(
        r_timer,
        r_subscriptions_repo.clone(),
        r_configmanager,
        r_icons_repo,
        uimock.upd_adp(),
        uimock.val_sto(),
        r_dl,
    );
    (fs, r_subscriptions_repo)
}

fn dataset_simple_trio() -> Vec<SubscriptionEntry> {
    let mut fs_list: Vec<SubscriptionEntry> = Vec::default();
    let mut fse =
        SubscriptionEntry::from_new_url("feed1-display".to_string(), "feed1-url".to_string());
    fse.subs_id = 1;
    fse.folder_position = 0;
    fs_list.push(fse.clone());

    fse.display_name = "feed2-display".to_string();
    fse.url = "feed2-url".to_string();
    fse.subs_id = 2;
    fse.folder_position = 1;
    fs_list.push(fse.clone());

    fse.display_name = "feed3-display".to_string();
    fse.url = "feed3-url".to_string();
    fse.subs_id = 3;
    fse.folder_position = 2;
    fs_list.push(fse.clone());
    fs_list
}

//RUST_BACKTRACE=1 cargo watch -s "cargo test  web::httpfetcher::httpfetcher_t::test_heise_svg  --lib -- --exact --nocapture"
// #[ignore]
#[test]
fn test_heise_svg() {
    setup();
    let web_fetcher = HttpFetcher {};
    let r = web_fetcher.request_url_bin(
        "https://www.heise.de/icons/ho/touch-icons/safari-pinned-tab.svg".to_string(),
    );
    let (ii, _msg) = blob_is_icon(&r.content_bin);
    // trace!(        "#content_bin={} #content={}   msg={}",        r.content_bin.len(),        r.content.len(),        msg    );
    assert_eq!(ii, 0);
}

//RUST_BACKTRACE=1 cargo watch -s "cargo test  web::httpfetcher::httpfetcher_t::test_remote_redirect --lib -- --exact --nocapture"
// #[ignore]
#[test]
fn test_remote_redirect() {
    setup();
    let web_fetcher = HttpFetcher {};
    let r = web_fetcher.request_url_bin("https://kodansha.us/favicon.ico".to_string());
    // debug!(        "#content_bin={} #content={}",        r.content_bin.len(),       r.content.len()    );
    let (is_icon, _msg) = blob_is_icon(&r.content_bin);
    assert_eq!(is_icon, 0);
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
