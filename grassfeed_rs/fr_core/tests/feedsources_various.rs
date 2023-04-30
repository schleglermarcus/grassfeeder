mod downloader_dummy;
mod logger_config;

use fr_core::controller::subscriptionmove::ISubscriptionMove;
use fr_core::controller::subscriptionmove::SubscriptionMove;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::icons::icon_analyser;
use fr_core::util::IconKind;
use fr_core::web::httpfetcher::HttpFetcher;
use fr_core::web::IHttpRequester;
use std::cell::RefCell;
use std::rc::Rc;

/// when there are messages with subscription_id=100, don't insert subscription with ID 99
#[test]
fn add_subscription_with_existing_messages() {
    setup();
    let subscriptions_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (mut subscription_controller, _r_fsource) = prepare_subscription_move(subscriptions_list);
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let mut mr1: MessageRow = MessageRow::default();
    mr1.subscription_id = 20;
    let _mr1id = msgrepo.insert(&mr1).unwrap() as isize;
    let msg_r_r = Rc::new(RefCell::new(msgrepo));
    subscription_controller.messagesrepo_r = msg_r_r.clone(); //  Rc::downgrade(&msg_r_r);

    let subs_id = subscription_controller
        .add_new_subscription("some-url-3".to_string(), "name-proc3".to_string());

    assert_eq!(subs_id, mr1.subscription_id + 1);
    let message = (*(msg_r_r.borrow())).get_by_src_id(subs_id, false);
    assert_eq!(message.len(), 0); // the new created subscription may not have messages attached
}

#[test]
fn add_feed_empty() {
    setup();
    let (mut fsc, r_fsource) = prepare_subscription_move(Vec::default());
    fsc.add_new_subscription(
        "tests/data/gui_proc_rss2_v1.rss".to_string(),
        "name-proc2".to_string(),
    );
    let entries = (*(r_fsource.borrow())).get_all_entries();
    assert_eq!(entries.len(), 4); // plus 3 pre-existing entries
}

#[test]
fn delete_feed_v1() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    // let (mut fsc, r_fsource) = prepare_stc(fs_list);
    let (mut fsc, r_fsource) = prepare_subscription_move(fs_list);
    fsc.set_delete_subscription_id(Some(2));
    fsc.move_subscription_to_trash();
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
    let (_fsc, r_fsource) = prepare_subscription_move(fs_list);
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
    let an_res = icon_analyser(&r.content_bin);
    assert_eq!(an_res.kind, IconKind::Svg);
}

/** remote  icon is gone
//RUST_BACKTRACE=1 cargo watch -s "cargo test  web::httpfetcher::httpfetcher_t::test_remote_redirect --lib -- --exact --nocapture"
// #[ignore]
#[test]
fn test_remote_redirect() {
    setup();
    let web_fetcher = HttpFetcher {};
    let r = web_fetcher.request_url_bin("https://kodansha.us/favicon.ico".to_string());
    let an_res = icon_analyser(&r.content_bin);
    debug!("test_remote_redirect  {:?}" , &an_res );
    assert_eq!(an_res.kind, IconKind::Ico);
}
 */

// ----------------------------

fn prepare_subscription_move(
    fs_list: Vec<SubscriptionEntry>,
) -> (SubscriptionMove, Rc<RefCell<dyn ISubscriptionRepo>>) {
    let mut subscrip_repo = SubscriptionRepo::new_inmem();
    subscrip_repo.startup_int();
    fs_list.iter().for_each(|e| {
        let _r = subscrip_repo.store_entry(e);
    });
    let r_subscriptions_repo: Rc<RefCell<dyn ISubscriptionRepo>> =
        Rc::new(RefCell::new(subscrip_repo));
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let mut mr1: MessageRow = MessageRow::default();
    mr1.subscription_id = 20;
    let _mr1id = msgrepo.insert(&mr1).unwrap() as isize;
    let msg_r_r = Rc::new(RefCell::new(msgrepo));
    let r_error_repo = Rc::new(RefCell::new(ErrorRepo::new(&String::default())));

    let subs_move = SubscriptionMove::new(r_subscriptions_repo.clone(), msg_r_r, r_error_repo);
    (subs_move, r_subscriptions_repo.clone())
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
