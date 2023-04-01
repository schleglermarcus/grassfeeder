mod logger_config;
mod unzipper;

use fr_core::config::configmanager::ConfigManager;
use fr_core::controller::contentdownloader::Downloader;
use fr_core::controller::contentdownloader::IDownloader;
use fr_core::controller::contentlist::CJob;
use fr_core::controller::guiprocessor::Job;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::comprehensive::ComprStart;
use fr_core::downloader::comprehensive::ComprehensiveInner;
use fr_core::util::StepResult;
use fr_core::web::mockfilefetcher::FileFetcher;
use fr_core::web::WebFetcherType;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn comprehensive_feed_download() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let base_path = format!("{}feeds/", unzipper::TD_BASE);
    let fetcher: WebFetcherType = Arc::new(Box::new(FileFetcher::new(base_path)));
    let comp_inner = ComprehensiveInner {
        feed_url_edit: "gui_proc_rss2_v1.rss".to_string(),
        iconrepo: IconRepo::new(""),
        web_fetcher: fetcher,
        download_error_happened: false,
        icon_url: String::default(),
        icon_bytes: Vec::default(),
        sourcetree_job_sender: stc_job_s,
        feed_homepage: String::default(),
        feed_title: String::default(),
        url_download_text: String::default(),
        icon_id: -1,
    };
    let last = StepResult::start(Box::new(ComprStart::new(comp_inner)));
    assert_eq!(last.download_error_happened, false);
    assert_eq!(last.feed_title, "Ajax and XUL".to_string());
    assert_eq!(last.feed_homepage, "http://localhost/".to_string());
    assert_eq!(last.icon_url, "http://localhost/favicon.ico".to_string());
    let all_e = last.iconrepo.get_all_entries();
    assert_eq!(all_e.len(), 1);
}

// #[ignore]
#[test]
fn downloader_load_message_into_db() {
    setup();
    let (content_q_s, _content_q_r) = flume::bounded::<CJob>(9);
    let base_path = format!("{}feeds/", unzipper::TD_BASE);
    let fetcher: WebFetcherType = Arc::new(Box::new(FileFetcher::new(base_path)));

    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let (gp_s, _gp_r) = flume::bounded::<Job>(9);
    let r_configmanager = Rc::new(RefCell::new(ConfigManager::default()));
    let f_src_repo = SubscriptionRepo::new_inmem(); // new("");
    f_src_repo.scrub_all_subscriptions();
    let mut fse = SubscriptionEntry::from_new_url(
        "feed1-display".to_string(),
        "gui_proc_rss2_v1.rss".to_string(),
    );
    fse.subs_id = 1;
    fse.folder_position = 0;
    let _r = f_src_repo.store_entry(&fse);

    let fsrc_r: Rc<RefCell<dyn ISubscriptionRepo>> = Rc::new(RefCell::new(f_src_repo));
    let icon_repo_r = Rc::new(RefCell::new(IconRepo::new("")));

    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let msgrepo_r = Rc::new(RefCell::new(msgrepo));
    let erro_rep = ErrorRepo::new(&String::default());
    let erro_r = Rc::new(RefCell::new(erro_rep));
    let mut downloader = Downloader::new(
        fetcher,
        fsrc_r,
        icon_repo_r,
        r_configmanager,
        msgrepo_r.clone(),
        erro_r,
    );
    downloader.contentlist_job_sender = Some(content_q_s);
    downloader.source_c_sender = Some(stc_job_s);
    downloader.gp_job_sender = Some(gp_s.clone());
    downloader.startup();
    let dl_r: Rc<RefCell<dyn IDownloader>> = Rc::new(RefCell::new(downloader));
    (*dl_r).borrow().add_update_source(1);
    std::thread::sleep(std::time::Duration::from_millis(2));
    (*dl_r).borrow_mut().shutdown();
    assert!(!(*dl_r).borrow().is_running());
    let msg = (*msgrepo_r).borrow().get_by_index(1).unwrap();
    // debug!(" msg={:?}", msg);
    assert_eq!(msg.message_id, 1);
    assert_eq!(msg.post_id, "2345");
}

/// Timestamp delivered   from    https://feeds.breakingnews.ie/bnworld
/// https://www.w3.org/Protocols/rfc822/#z28
// #[ignore]
#[test]
fn chrono_broken_timestamp() {
    setup();
    let broken_ts = "Fri, 05 Aug 2022 23:28:01 Europe/Dublin";
    let pars_res = chrono::DateTime::parse_from_rfc2822(&broken_ts);
    assert!(pars_res.is_err());
    assert_eq!(
        pars_res.err().unwrap().to_string(),
        "trailing input".to_string()
    );
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
            (logger_config::QuietFlags::Downloader as u64)
                | (logger_config::QuietFlags::Controller as u64),
            // 0,
        );
        unzipper::unzip_some();
    });
}
