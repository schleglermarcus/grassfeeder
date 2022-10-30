mod logger_config;

use chrono::DateTime;
use fr_core::controller::contentlist::CJob;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::messages::FetchInner;
use fr_core::downloader::messages::FetchStart;
use fr_core::util::timestamp_now;
use fr_core::util::StepResult;
use fr_core::web::mockfilefetcher::FileFetcher;
use fr_core::web::WebFetcherType;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn single_dl_regular() {
    setup();
    let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
    let f_src_repo = SubscriptionRepo::new_inmem();
    f_src_repo.scrub_all_subscriptions();
    let icon_repo = IconRepo::new("");
    let (stc_job_s, stc_job_r) = flume::bounded::<SJob>(9);
    let msgrepo = MessagesRepo::new_in_mem();
    msgrepo.get_ctx().create_table();
    let msgrepo_req = MessagesRepo::new_by_connection(msgrepo.get_ctx().get_connection());
    let erro_rep = ErrorRepo::new(&String::default());
    let inner = FetchInner {
        fs_repo_id: 1,
        url: "gui_proc_rss2_v1.rss".to_string(),
        cjob_sender: c_q_s,
        subscriptionrepo: f_src_repo,
        iconrepo: icon_repo,
        web_fetcher: get_file_fetcher(),
        download_text: String::default(),
        download_error_happened: false,
        sourcetree_job_sender: stc_job_s,
        timestamp_created: 0,
        messgesrepo: msgrepo,
        download_error_text: String::default(),
        erro_repo: erro_rep,
    };
    let ts_now = timestamp_now();
    let date_copied_from_example =
        DateTime::parse_from_rfc2822("Wed, 10 Nov 2021 14:51:28 EST").unwrap();

    let f_inner = StepResult::start(Box::new(FetchStart::new(inner)));
    assert_eq!(f_inner.download_error_happened, false);
    assert_eq!(f_inner.download_text.len(), 0);
    assert_eq!(stc_job_r.recv(), Ok(SJob::SetFetchInProgress(1)));
    assert_eq!(
        stc_job_r.recv(),
        Ok(SJob::StoreFeedCreateUpdate(
            1,
            ts_now,
            date_copied_from_example.timestamp()
        ))
    );
    assert_eq!(stc_job_r.recv(), Ok(SJob::SetFetchFinished(1, false)));
    assert!(stc_job_r.is_empty());
    let all = msgrepo_req.get_all_messages();
    assert_eq!(all.len(), 2);
}

// #[ignore]
#[test]
fn download_with_create_date() {
    setup();
    let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
    let subsc_r = SubscriptionRepo::new_inmem();
    subsc_r.scrub_all_subscriptions();
    let icon_repo = IconRepo::new("");
    let (stc_job_s, stc_job_r) = flume::bounded::<SJob>(9);
    let msgrepo = MessagesRepo::new_in_mem();
    let msgrepo_req = MessagesRepo::new_by_connection(msgrepo.get_ctx().get_connection());
    msgrepo.get_ctx().create_table();
    let erro_rep = ErrorRepo::new(&String::default());
    let inner = FetchInner {
        fs_repo_id: 3,
        url: "gui_proc_rss2_v1.rss".to_string(),
        cjob_sender: c_q_s,
        subscriptionrepo: subsc_r,
        iconrepo: icon_repo,
        web_fetcher: get_file_fetcher(),
        download_error_happened: false,
        sourcetree_job_sender: stc_job_s,
        timestamp_created: 0,
        messgesrepo: msgrepo,
        download_text: String::default(),
        download_error_text: String::default(),
        erro_repo: erro_rep,
    };
    let ts_now = timestamp_now();
    let date_copied_from_example =
        DateTime::parse_from_rfc2822("Wed, 10 Nov 2021 14:51:28 EST").unwrap();
    // debug!(" {:?} stamp={}", dt_example, dt_example.timestamp()); // -> ParseResult<DateTime<FixedOffset>>
    let f_inner = StepResult::start(Box::new(FetchStart::new(inner)));
    assert_eq!(f_inner.download_error_happened, false);
    assert_eq!(f_inner.download_text.len(), 0);
    assert_eq!(stc_job_r.recv().unwrap(), SJob::SetFetchInProgress(3));
    assert_eq!(
        stc_job_r.recv().unwrap(),
        SJob::StoreFeedCreateUpdate(3, ts_now, date_copied_from_example.timestamp())
    );
    assert_eq!(stc_job_r.recv().unwrap(), SJob::SetFetchFinished(3, false));
    let all = msgrepo_req.get_all_messages();
    assert_eq!(all.len(), 2);
}

fn get_file_fetcher() -> WebFetcherType {
    Arc::new(Box::new(FileFetcher::new(
        // "../fr_core/tests/feeds/".to_string(),
        "../target/td/feeds/".to_string(),
    )))
}

#[allow(dead_code)]
fn prepare_feedsource_dummy() -> Rc<RefCell<dyn ISubscriptionRepo>> {
    let mut fse = SubscriptionEntry::from_new_url(
        "feed1-display".to_string(),
        "gui_proc_rss2_v1.rss".to_string(),
    );
    fse.subs_id = 1;
    fse.folder_position = 0;
    let subscription_repo = SubscriptionRepo::new_inmem();
    let _r = subscription_repo.store_entry(&fse);
    let r_fsource: Rc<RefCell<dyn ISubscriptionRepo>> = Rc::new(RefCell::new(subscription_repo));
    r_fsource
}

// ------------------------------------

#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(logger_config::QuietFlags::Db as u64);
    });
}
