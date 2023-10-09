use flume::Receiver;
use fr_core::controller::guiprocessor::Job;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::db_clean::CleanerInner;
use fr_core::downloader::db_clean::CleanerStart;
use fr_core::util::StepResult;

const CONF_PATH: &str = "../target/db_cleaner";

// #[ignore]
#[test]
fn investigate_cleaning_proc() {
    setup();

    info!("copy ...");
    copy_big_files(true);
    info!("prepare_inner  ..");
    let (cleaner_i, gpj_r) = prepare_cleaner_inner();
    info!("starting clean  ...");

    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));
    info!("stopped  clean  , printing Events ");
    gpj_r.try_iter().for_each(|ev| debug!(" {:?} ", ev));
    info!("===========================");
}

fn prepare_cleaner_inner() -> (CleanerInner, Receiver<Job>) {
    let max_messages: i32 = 1000;
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(99);
    let (gpj_s, gpj_r) = flume::bounded::<Job>(99);
    let subsrepo = SubscriptionRepo::by_file(&format!("{}/subscriptions.db", CONF_PATH));

    let msgrepo1 = MessagesRepo::new_by_filename_add_column(&format!("{}/errors.db", CONF_PATH));
    let err_repo = ErrorRepo::new(&format!("{}/", CONF_PATH));

    let mut iconrepo: IconRepo;
    iconrepo = IconRepo::new(CONF_PATH);
    iconrepo.startup();

    let cleaner_i = CleanerInner::new(
        gpj_s,
        stc_job_s,
        subsrepo,
        msgrepo1,
        iconrepo,
        max_messages,
        err_repo,
    );
    (cleaner_i, gpj_r)
}

fn copy_big_files(perform_copy: bool) {
    let mut homedir: String = String::from("~");
    if let Ok(s) = std::env::var("HOME") {
        homedir = s;
    }
    let r = std::fs::create_dir_all(CONF_PATH);
    assert!(r.is_ok());
    if perform_copy {
        for (path, file) in [
            (".cache/grassfeeder/", "errors.db"),
            (".config/grassfeeder/", "icons_list.json"),
            (".config/grassfeeder/", "messages.db"),
            (".config/grassfeeder/", "subscriptions.db"),
        ] {
            let sorc = format!("{}/{}{}", homedir, path, file);
            let dest = format!("{}/{}", CONF_PATH, file);
            debug!("copy  {} =>  {} ", sorc, dest);
            assert!(std::fs::copy(sorc, dest).is_ok());
        }
    }
}

// ------------------------------------

#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config_local::setup_logger();
    });
}
