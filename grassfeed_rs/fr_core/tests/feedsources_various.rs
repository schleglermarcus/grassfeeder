mod logger_config;

// mockall cannot provide a consistent data set, needs to be instrumented for each request separately.
mod downloader_dummy;

use crate::downloader_dummy::DownloaderDummy;
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
use fr_core::timer::build_timer;
use fr_core::timer::ITimer;
use fr_core::ui_select::uimock::UIMock;
use fr_core::web::httpfetcher::HttpFetcher;
use fr_core::web::IHttpRequester;
use std::cell::RefCell;
use std::rc::Rc;

/// when there are messages with subscription_id=100, don't insert subscription with ID 99
// #[ignore]
#[test]
fn add_subscription_with_existing_messages() {
    setup();
    let subscriptions_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (mut subscription_controller, _r_fsource) = prepare_stc(subscriptions_list);
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let mut mr1: MessageRow = MessageRow::default();
    mr1.subscription_id = 20;
    let _mr1id = msgrepo.insert(&mr1).unwrap() as isize;
    let msg_r_r = Rc::new(RefCell::new(msgrepo));
    subscription_controller.messagesrepo_w = Rc::downgrade(&msg_r_r);
    let subs_id = subscription_controller
        .add_new_subscription("some-url-3".to_string(), "name-proc3".to_string());
    assert_eq!(subs_id, mr1.subscription_id + 1);
    let message = (*(msg_r_r.borrow())).get_by_src_id(subs_id, false);
    assert_eq!(message.len(), 0); // the new created subscription may not have messages attached
}

// #[ignore]
#[test]
fn add_feed_empty() {
    setup();
    let (mut fsc, r_fsource) = prepare_stc(Vec::default());
    fsc.add_new_subscription(
        "tests/data/gui_proc_rss2_v1.rss".to_string(),
        "name-proc2".to_string(),
    );
    let entries = (*(r_fsource.borrow())).get_all_entries();
    assert_eq!(entries.len(), 4); // plus 3 pre-existing entries
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
    // debug!("{:?}", &result);
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
    // result.iter().for_each(|fs| debug!("##  {}", fs));
    assert_eq!(result.get(2).unwrap().subs_id, 1);
    assert_eq!(result.get(2).unwrap().parent_subs_id, 22);
    assert_eq!(result.get(2).unwrap().folder_position, 33);
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

// ----------------------------

fn prepare_stc(
    fs_list: Vec<SubscriptionEntry>,
) -> (SourceTreeController, Rc<RefCell<dyn ISubscriptionRepo>>) {
    let mut subscrip_repo = SubscriptionRepo::new_inmem();
    subscrip_repo.startup_int();
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
