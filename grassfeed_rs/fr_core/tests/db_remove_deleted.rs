use fr_core::controller::contentlist::CJob;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::errorentry::ESRC;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::icon_repo::IconEntry;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::message::compress;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::db_clean::CheckErrorLog;
use fr_core::downloader::db_clean::CleanerInner;
use fr_core::downloader::db_clean::CleanerStart;
use fr_core::downloader::db_clean::CorrectIconsDoublettes;
use fr_core::downloader::db_clean::RemoveNonConnectedSubscriptions;
use fr_core::downloader::db_clean::MAX_ERROR_LINES_PER_SUBSCRIPTION;
use fr_core::downloader::db_clean::MAX_ERROR_LINE_AGE_S;
use fr_core::util::timestamp_now;
use fr_core::util::Step;
use fr_core::util::StepResult;
use fr_core::TD_BASE;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::RwLock;

// #[ignore]
#[test]
fn db_cleanup_nonconnected() {
    setup();
    /*
       let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
       let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
       let subsrepo = SubscriptionRepo::new_inmem();
       subsrepo.scrub_all_subscriptions();
       let msgrepo1 = MessagesRepo::new_in_mem();
       let err_repo = ErrorRepo::new("../target/");
       msgrepo1.get_ctx().create_table();
       prepare_db_with_errors_1(&msgrepo1, &subsrepo);
       let mut iconrepo = IconRepo::new("../target/");
       iconrepo.startup();
       let cleaner_i = CleanerInner::new(c_q_s, stc_job_s, subsrepo, msgrepo1, iconrepo, 5, err_repo);
    */
    let cleaner_i = prepare_cleaner_inner(None);
    prepare_db_with_errors_1(&cleaner_i.messagesrepo, &cleaner_i.subscriptionrepo);
    // cleaner_i.subscriptionrepo.create_table();

    // let msgrepo2 =        MessagesRepo::new_by_connection(cleaner_i.messagesrepo.get_ctx().get_connection());

    let sut = RemoveNonConnectedSubscriptions(cleaner_i);
    // let inner = Box::new(sut).step();  //  = StepResult::start(Box::new());

    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();

        let msg1 = inner.messagesrepo.get_by_src_id(5, false);
        assert_eq!(msg1.len(), 5);
    }
}

#[ignore]
#[test]
fn cleanup_message_doublettes() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
    let subsrepo = SubscriptionRepo::new_inmem();
    subsrepo.scrub_all_subscriptions();
    let msgrepo1 = MessagesRepo::new_in_mem();
    let err_repo = ErrorRepo::new("../target/");
    msgrepo1.get_ctx().create_table();
    prepare_db_with_errors_1(&msgrepo1, &subsrepo);
    let mut iconrepo = IconRepo::new("../target/");
    iconrepo.startup();
    let cleaner_i = CleanerInner::new(
        c_q_s, stc_job_s, subsrepo, msgrepo1, iconrepo, 1000, err_repo,
    );
    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));
    let msg4 = inner.messagesrepo.get_by_src_id(4, false);
    assert_eq!(msg4.len(), 1); // the other 10 are set deleted
}

fn clean_phase1(subs_repo: &SubscriptionRepo) {
    let all_entries = subs_repo.get_all_entries();
    let mut connected_child_list: HashSet<isize> = HashSet::default();
    let mut folder_work: Vec<isize> = Vec::default();
    folder_work.push(0);
    while folder_work.len() > 0 {
        let parent_subs_id = folder_work.pop().unwrap();
        let childs = subs_repo.get_by_parent_repo_id(parent_subs_id);
        childs.iter().for_each(|se| {
            connected_child_list.insert(se.subs_id);
            if se.is_folder {
                folder_work.push(se.subs_id);
            }
        });
    }
    let mut delete_list: HashSet<isize> = HashSet::default();
    all_entries.iter().for_each(|se| {
        if se.deleted || se.parent_subs_id < 0 {
            delete_list.insert(se.subs_id);
        } else {
            if !connected_child_list.contains(&se.subs_id) {
                if delete_list.contains(&se.parent_subs_id) {
                    delete_list.insert(se.subs_id);
                } else {
                    trace!("NotConnected: {:?}", &se);
                }
            }
        }
    });
    delete_list
        .iter()
        .for_each(|id| subs_repo.delete_by_index(*id));
}

#[ignore]
#[test]
fn db_cleanup_remove_deleted() {
    setup();
    let db_problem_json = format!("{}websites/san_subs_list_dmg1.json", TD_BASE);
    let subsrepo = SubscriptionRepo::new_inmem();
    subsrepo.scrub_all_subscriptions();
    let lines = std::fs::read_to_string(db_problem_json.to_string()).unwrap();
    let dec_r: serde_json::Result<Vec<SubscriptionEntry>> = serde_json::from_str(&lines);
    let json_vec = dec_r.unwrap();
    json_vec.iter().enumerate().for_each(|(n, entry)| {
        let r = subsrepo.store_entry(&entry);
        if r.is_err() {
            warn!(
                "importing {}/{}  \t {:?}  {:?}",
                n,
                json_vec.len(),
                &entry,
                r.err()
            );
        }
    });
    clean_phase1(&subsrepo);
    let all_entries = subsrepo.get_all_entries();
    assert_eq!(all_entries.len(), 309);
}

#[ignore]
#[test]
fn t_db_cleanup_1() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
    let subsrepo = SubscriptionRepo::new_inmem(); // new("");
    subsrepo.scrub_all_subscriptions();
    let msgrepo1 = MessagesRepo::new_in_mem();
    let msgrepo2 = MessagesRepo::new_by_connection(msgrepo1.get_ctx().get_connection());
    msgrepo1.get_ctx().create_table();
    prepare_db_with_errors_1(&msgrepo1, &subsrepo);
    let subsrepo1 = SubscriptionRepo::by_existing_connection(subsrepo.get_connection()); // by_existing_list(subsrepo.get_list());
    let mut iconrepo = IconRepo::new("../target/");
    iconrepo.startup();
    let err_repo = ErrorRepo::new("../target/");
    let cleaner_i = CleanerInner::new(c_q_s, stc_job_s, subsrepo, msgrepo1, iconrepo, 5, err_repo);
    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));
    let parent_ids_to_correct = inner.fp_correct_subs_parent.lock().unwrap().clone();
    assert_eq!(parent_ids_to_correct.len(), 1);
    let sub1 = subsrepo1.get_by_index(1).unwrap();
    assert!(sub1.display_name.starts_with("folder1"));
    assert!(subsrepo1.get_by_index(2).unwrap().display_name.len() < 10);
    assert!(!subsrepo1.get_by_index(2).unwrap().expanded);
    assert_eq!(msgrepo2.get_by_index(2).unwrap().is_deleted, false); // belongs to subscription, keep it
}

fn prepare_db_with_errors_1(msgrepo: &MessagesRepo, subsrepo: &SubscriptionRepo) {
    let mut se = SubscriptionEntry::default();
    se.is_folder = true;
    se.display_name = "folder1".to_string();
    se.icon_id = 30;
    assert!(subsrepo.store_entry(&se).is_ok()); // id 1
    se.is_folder = false;
    se.parent_subs_id = 1;
    se.icon_id = 31;
    assert!(subsrepo.store_entry(&se).is_ok()); // id 2
    se.parent_subs_id = 1; // unchanged folder pos, that's an error
    se.display_name = "Japan 無料ダウンロード".to_string();
    se.expanded = true;
    se.icon_id = 32;
    assert!(subsrepo.store_entry(&se).is_ok()); // id 3
    se.display_name = "fourth".to_string();
    se.expanded = false;
    se.folder_position = 3;
    assert!(subsrepo.store_entry(&se).is_ok()); // id 4
    se.display_name = "fifth".to_string();
    se.expanded = false;
    se.folder_position = 4;
    assert!(subsrepo.store_entry(&se).is_ok()); // id 5
    let mut m1 = MessageRow::default();
    m1.fetch_date = timestamp_now();
    m1.subscription_id = 1;
    let _r = msgrepo.insert(&m1);
    m1.subscription_id = 2;
    let _r = msgrepo.insert(&m1);
    m1.is_deleted = false;
    m1.subscription_id = 4;
    m1.entry_src_date = 1000000000_i64;
    m1.title = compress("fifth"); // .to_string();
    for i in 0..10 {
        m1.post_id = format!("post-{}", i);
        m1.fetch_date = 1000000000_i64 + 100000 * i;
        let _r = msgrepo.insert(&m1);
    }
    m1.is_deleted = false;
    m1.subscription_id = 5;
    m1.title = compress("fifth"); // .to_string();
    for i in 0..10 {
        m1.post_id = format!("post-{}", i);
        m1.entry_src_date = 1000000000_i64 + 100000 * i;
        let _r = msgrepo.insert(&m1);
    }
    let r = std::fs::copy("tests/data/icons_sane.json", "../target/icons_list.json");
    assert!(r.is_ok());
}

// ----------------------

#[ignore]
#[test]
fn clean_icon_doublettes() {
    setup();
    /*

       let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
       let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
       let subsrepo = SubscriptionRepo::new_inmem();
       subsrepo.scrub_all_subscriptions();
       let msgrepo1 = MessagesRepo::new_in_mem();
       let err_repo = ErrorRepo::new("../target/");
       msgrepo1.get_ctx().create_table();
       prepare_db_with_errors_1(&msgrepo1, &subsrepo);
       let _r = std::fs::create_dir("../target/iconc");
       let r = std::fs::copy(
           "tests/data/icons_list.json",
           "../target/iconc/icons_list.json",
       );
       assert!(r.is_ok());
       let mut iconrepo = IconRepo::new("../target/iconc");
       iconrepo.startup();

       let cleaner_i = CleanerInner::new(
           c_q_s, stc_job_s, subsrepo, msgrepo1, iconrepo, 1000, err_repo,
       );
    */
    let cleaner_i = prepare_cleaner_inner(Some("../target/iconc"));

    let sut = CorrectIconsDoublettes(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();

        // let mut iconrepo = IconRepo::new("../target/iconc");
        // iconrepo.startup();
        let all = inner.iconrepo.get_all_entries();
        assert_eq!(all.len(), 3);
    }

    // let cib = Box::new(CorrectIconsDoublettes(cleaner_i));
    // cib.step();
}

#[ignore]
#[test]
fn clean_errorlist_too_old() {
    setup();
    let cleaner_i = prepare_cleaner_inner(None);
    let date_now = timestamp_now();
    let timediff: usize = MAX_ERROR_LINE_AGE_S / 5;
    for i in 0..11 {
        let _r = cleaner_i.error_repo.add_error_ts(
            2,
            ESRC::None,
            0,
            String::default(),
            String::default(),
            date_now - (i * timediff) as i64,
        );
    }
    let sut = CheckErrorLog(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();
        let list = inner.error_repo.get_by_subscription(2);
        assert_eq!(list.len(), 6);
    }
}

#[ignore]
#[test]
fn clean_errorlist_too_many() {
    setup();
    let cleaner_i = prepare_cleaner_inner(None);
    let date_now = timestamp_now();
    for i in 0..(MAX_ERROR_LINES_PER_SUBSCRIPTION * 2) {
        let _r = cleaner_i.error_repo.add_error_ts(
            2,
            ESRC::None,
            0,
            String::default(),
            String::default(),
            date_now + i as i64,
        );
    }
    let sut = CheckErrorLog(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();
        let list = inner.error_repo.get_by_subscription(2);
        assert_eq!(list.len(), MAX_ERROR_LINES_PER_SUBSCRIPTION); // retain the upper half
        assert!(list[0].date > (date_now + MAX_ERROR_LINES_PER_SUBSCRIPTION as i64));
    }
}

#[ignore]
#[test]
fn clean_errorlist_no_subscription() {
    setup();
    let cleaner_i = prepare_cleaner_inner(None);
    for i in 1..9 {
        let _r = cleaner_i.error_repo.add_error_ts(
            (1 + i) as isize,
            ESRC::None,
            0,
            String::default(),
            String::default(),
            0,
        );
    }
    let sut = CheckErrorLog(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();
        let list = inner.error_repo.get_all_stored_entries();
        assert_eq!(list.len(), 4); //  	subs_ids [5, 3, 2, 4]
    } else {
        panic!()
    }
}

fn prepare_cleaner_inner(copy_icons: Option<&str>) -> CleanerInner {
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
    let subsrepo = SubscriptionRepo::new_inmem();
    subsrepo.scrub_all_subscriptions();
    let msgrepo1 = MessagesRepo::new_in_mem();

    let err_repo = ErrorRepo::new_in_mem();
    msgrepo1.get_ctx().create_table();
    prepare_db_with_errors_1(&msgrepo1, &subsrepo);
    let mut iconrepo: IconRepo;
    if let Some(i_p) = copy_icons {
        copy_icon_json(i_p); //  "../target/iconc"
        iconrepo = IconRepo::new(i_p);
        iconrepo.startup();
    } else {
        let dummy_icon_list: Arc<RwLock<HashMap<isize, IconEntry>>> =
            Arc::new(RwLock::new(HashMap::default()));
        iconrepo = IconRepo::by_existing_list(dummy_icon_list);
    }
    let cleaner_i = CleanerInner::new(
        c_q_s, stc_job_s, subsrepo, msgrepo1, iconrepo, 100, err_repo,
    );
    cleaner_i
}

fn copy_icon_json(icn_path: &str) {
    let _r = std::fs::create_dir(icn_path);
    let r = std::fs::copy(
        "tests/data/icons_list.json",
        format!("{}/icons_list.json", icn_path),
    );
    assert!(r.is_ok());
    let mut iconrepo = IconRepo::new(icn_path);
    iconrepo.startup();
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
        let _r = logger_config::setup_fern_logger(
            // logger_config::QuietFlags::Downloader as u64
            0,
        );
        unzipper::unzip_some();
    });
}
