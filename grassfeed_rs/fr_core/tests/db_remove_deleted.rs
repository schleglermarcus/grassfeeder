use fr_core::controller::guiprocessor::Job;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::errorentry::ESRC;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::icon_repo::IIconRepo;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::message::compress;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::db_clean::AnalyzeFolderPositions;
use fr_core::downloader::db_clean::CheckErrorLog;
use fr_core::downloader::db_clean::CleanerInner;
use fr_core::downloader::db_clean::CleanerStart;
use fr_core::downloader::db_clean::CorrectIconsDoublettes;
use fr_core::downloader::db_clean::DeleteDoubleSameMessages;
use fr_core::downloader::db_clean::DeleteUnusedIcons;
use fr_core::downloader::db_clean::ReduceTooManyMessages;
use fr_core::downloader::db_clean::MAX_ERROR_LINES_PER_SUBSCRIPTION;
use fr_core::downloader::db_clean::MAX_ERROR_LINE_AGE_S;
use fr_core::util::timestamp_now;
use fr_core::util::Step;
use fr_core::util::StepResult;

// #[ignore]
#[test]
fn clean_errorlist_no_subscription() {
    setup();
    let date_now = timestamp_now();
    let cleaner_i = prepare_cleaner_inner(-1);
    for i in 1..9 {
        let _r = cleaner_i.error_repo.add_error_ts(
            (1 + i) as isize,
            ESRC::None,
            0,
            String::default(),
            String::default(),
            date_now + i,
        );
    }
    let sut = CheckErrorLog(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();
        let errlist = inner.error_repo.get_all_stored_entries();
        assert_eq!(errlist.len(), 4); //  	subs_ids [5, 3, 2, 4]
    } else {
        panic!()
    }
}

// #[ignore]
#[test]
fn clean_subscriptions_names_expanded() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
    prepare_db_with_errors_1(&cleaner_i.messagesrepo, &cleaner_i.subscriptionrepo);
    let sut = AnalyzeFolderPositions(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();
        let subsc1 = inner.subscriptionrepo.get_by_index(1).unwrap();
        // debug!("  sub1 {:?}", subsc1);
        assert!(subsc1.display_name.starts_with("folder1"));
        let subsc2 = inner.subscriptionrepo.get_by_index(2).unwrap();
        assert!(subsc2.display_name.len() < 10);
        assert!(!subsc2.expanded);
        let msg2 = inner.messagesrepo.get_by_index(2).unwrap();
        assert_eq!(msg2.is_deleted, false); // belongs to subscription, keep it
    }
}


// #[ignore]
#[test]
fn t_db_cleanup_1() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));
    let parent_ids_to_correct = inner.fp_correct_subs_parent.lock().unwrap().clone();
    assert_eq!(parent_ids_to_correct.len(), 1);
    let sub1 = inner.subscriptionrepo.get_by_index(1).unwrap();
    assert!(sub1.display_name.starts_with("folder1"));
    let sub2 = inner.subscriptionrepo.get_by_index(2).unwrap();
    assert!(sub2.display_name.len() < 10);
    assert!(!sub2.expanded);
    let msg2 = inner.messagesrepo.get_by_index(2).unwrap();
    assert_eq!(msg2.is_deleted, false); // belongs to subscription, keep it
}

// #[ignore]
#[test]
fn t_subscr_parent_ids_correction() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
    prepare_db_with_errors_1(&cleaner_i.messagesrepo, &cleaner_i.subscriptionrepo);
    let sut = AnalyzeFolderPositions(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();
        let parent_ids_to_correct = inner.fp_correct_subs_parent.lock().unwrap().clone();
        // info!("  B:  parent_ids_to_correct  {:?}", parent_ids_to_correct);
        assert_eq!(parent_ids_to_correct[0], 1);
        assert_eq!(parent_ids_to_correct[1], 0);
    }
}

// #[ignore]
#[test]
fn clean_message_doublettes() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
    prepare_db_with_errors_1(&cleaner_i.messagesrepo, &cleaner_i.subscriptionrepo);
    let sut = DeleteDoubleSameMessages(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let mut inner: CleanerInner = s.take();
        let msg4_i = inner.messagesrepo.get_by_subscription(4);
        assert_eq!(msg4_i.len(), 1); // the other 10 are set deleted
    }
}

// #[ignore]
#[test]
fn clean_too_many_messages() {
    setup();
    let c_max_messages: i32 = 5;
    let cleaner_i = prepare_cleaner_inner(c_max_messages);
    prepare_db_with_errors_1(&cleaner_i.messagesrepo, &cleaner_i.subscriptionrepo);
    let sut = ReduceTooManyMessages(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let mut inner: CleanerInner = s.take();
        let msg1_i = inner.messagesrepo.get_by_subscription(5);
        assert_eq!(msg1_i.len(), c_max_messages as usize);
    }
}

// #[ignore]
#[test]
fn clean_icon_doublettes() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
    prepare_add_icons(&cleaner_i);
    let sut = CorrectIconsDoublettes(cleaner_i);
    if let StepResult::Continue(s) = Box::new(sut).step() {
        let inner: CleanerInner = s.take();
        let all = inner.iconrepo.get_all_entries();
        assert_eq!(all.len(), 61);
    }
}

// #[ignore]
#[test]
fn clean_errorlist_too_old() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
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
        // debug!("errors list:  {:?} ", list);
        assert_eq!(list.len(), 6);
    }
}

// #[ignore]
#[test]
fn clean_errorlist_too_many() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
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
    } else {
        assert!(false);
    }
}

//  cargo watch -s "(cd fr_core; cargo test --test db_remove_deleted   )  "
// #[ignore]
#[test]
fn t_delete_unused_icons() {
    setup();
    let cleaner_i = prepare_cleaner_inner(-1);
    prepare_add_icons(&cleaner_i);
    let sut = DeleteUnusedIcons(cleaner_i);
    let r = Box::new(sut).step();
    if let StepResult::Stop(ref _s) = r {
        assert!(false);
    }
    if let StepResult::Continue(s) = r {
        let inner: CleanerInner = s.take();
        let icon_ids: Vec<isize> = inner
            .iconrepo
            .get_all_entries()
            .iter()
            .map(|ie| ie.icon_id)
            .collect::<Vec<isize>>();
        assert_eq!(icon_ids.len(), 24);
    } else {
        assert!(false);
    }
}

const SRC_ICONSDB: &str = "tests/data/icons_testing.db";

fn prepare_add_icons(inner: &CleanerInner) {
    let iconrepo = IconRepo::new_by_filename(SRC_ICONSDB);
    iconrepo.get_all_entries().into_iter().for_each(|ic| {
        let r = inner
            .iconrepo
            .store_icon(ic.icon_id, ic.icon, ic.compression_type);
        assert!(r.is_ok());
    });
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
    se.icon_id = 33;
    assert!(subsrepo.store_entry(&se).is_ok()); // id 4
    se.display_name = "fifth".to_string();
    se.expanded = false;
    se.folder_position = 4;
    se.icon_id = 34;
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
}

fn prepare_cleaner_inner(max_messages: i32) -> CleanerInner {
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(99);
    let (gpj_s, _gpj_r) = flume::bounded::<Job>(99);
    let subsrepo = SubscriptionRepo::new_inmem();
    subsrepo.scrub_all_subscriptions();
    let msgrepo1 = MessagesRepo::new_in_mem();
    let err_repo = ErrorRepo::new_in_mem();
    msgrepo1.get_ctx().create_table();
    prepare_db_with_errors_1(&msgrepo1, &subsrepo);
    let iconrepo = IconRepo::new_in_mem();
    let cleaner_i = CleanerInner::new(
        gpj_s,
        stc_job_s,
        subsrepo,
        msgrepo1,
        iconrepo,
        max_messages,
        err_repo,
    );
    cleaner_i
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
              logger_config::QuietFlags::Downloader as u64,
           // 0,
        );
        unzipper::unzip_some();
    });
}
