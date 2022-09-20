mod logger_config;

use fr_core::controller::contentlist::CJob;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::check_consistency;
use fr_core::db::message::compress;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::db_clean::CleanerInner;
use fr_core::downloader::db_clean::CleanerStart;
use fr_core::util::timestamp_now;
use fr_core::util::StepResult;
use std::collections::HashSet;

#[ignore]
#[test]
fn db_check_manual() {
    setup();
    check_consistency::databases_consistency_check_u(
        &"/home/marcus/dbcheck/".to_string(),
        true,
        true,
    );
}

// #[ignore]
#[test]
fn cleanup_message_doublettes() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
    let subsrepo = SubscriptionRepo::new_inmem();
    subsrepo.scrub_all_subscriptions();
    let msgrepo1 = MessagesRepo::new_in_mem();
    msgrepo1.get_ctx().create_table();
    prepare_db_with_errors_1(&msgrepo1, &subsrepo);
    let cleaner_i = CleanerInner::new(c_q_s, stc_job_s, subsrepo, msgrepo1);
    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));
    let msg4 = inner.messgesrepo.get_by_src_id(4, false);
    assert_eq!(msg4.len(), 1); // the other 10 are set deleted
}

// #[ignore]
#[test]
fn db_cleanup_too_many_messages() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let (c_q_s, _c_q_r) = flume::bounded::<CJob>(9);
    let subsrepo = SubscriptionRepo::new_inmem();
    subsrepo.scrub_all_subscriptions();
    let msgrepo1 = MessagesRepo::new_in_mem();
    let msgrepo2 = MessagesRepo::new_by_connection(msgrepo1.get_ctx().get_connection());
    msgrepo1.get_ctx().create_table();
    prepare_db_with_errors_1(&msgrepo1, &subsrepo);
    let mut cleaner_i = CleanerInner::new(c_q_s, stc_job_s, subsrepo, msgrepo1);
    cleaner_i.max_messages_per_subscription = 5;
    let _inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));
    let msg1 = msgrepo2.get_by_src_id(5, false);
    // msg1.iter().for_each(|m| debug!("CR: {}", m));
    // debug!("#msg={}", msg1.len());
    assert_eq!(msg1.len(), 5);
}

fn clean_phase1(subs_repo: &SubscriptionRepo) {
    let all_entries = subs_repo.get_all_entries();
    let mut connected_child_list: HashSet<isize> = HashSet::default();
    let mut folder_todo: Vec<isize> = Vec::default();
    folder_todo.push(0);
    while folder_todo.len() > 0 {
        let parent_subs_id = folder_todo.pop().unwrap();
        // trace!("looking at parent {}", parent_subs_id);
        let childs = subs_repo.get_by_parent_repo_id(parent_subs_id);
        childs.iter().for_each(|se| {
            connected_child_list.insert(se.subs_id);
            if se.is_folder {
                folder_todo.push(se.subs_id);
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
    // debug!(        "  #connected: {}   #to_delete: {}",        connected_child_list.len(),        delete_list.len()    );
    delete_list
        .iter()
        .for_each(|id| subs_repo.delete_by_index(*id));
}

// #[ignore]
#[test]
fn db_cleanup_remove_deleted() {
    setup();
    let db_problem_json = "../fr_core/tests/data/san_subs_list_dmg1.json";

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
    debug!("Phase1: #all: {}", all_entries.len());
    assert_eq!(all_entries.len(), 309);
}

// #[ignore]
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
    let cleaner_i = CleanerInner::new(c_q_s, stc_job_s, subsrepo, msgrepo1);
    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));

    let parent_ids_to_correct = inner.fp_correct_subs_parent.lock().unwrap().clone();
    // debug!(" to_correct: {:?}", parent_ids_to_correct);
    assert_eq!(parent_ids_to_correct.len(), 1);

    assert!(subsrepo1
        .get_by_index(1)
        .unwrap()
        .display_name
        .starts_with("unnamed"));
    assert!(subsrepo1.get_by_index(2).unwrap().display_name.len() < 10);
    assert!(!subsrepo1.get_by_index(2).unwrap().expanded);
    // msgrepo2        .get_all_messages()        .iter()        .for_each(|m| debug!("MSG {}", m));
    assert_eq!(msgrepo2.get_by_index(1).unwrap().is_deleted, true); //  belongs to folder,   delete it
    assert_eq!(msgrepo2.get_by_index(2).unwrap().is_deleted, false); // belongs to subscription, keep it
}

fn prepare_db_with_errors_1(msgrepo: &MessagesRepo, subsrepo: &SubscriptionRepo) {
    let mut se = SubscriptionEntry::default();
    se.is_folder = true;
    assert!(subsrepo.store_entry(&se).is_ok()); // id 1
    se.is_folder = false;
    se.parent_subs_id = 1;
    assert!(subsrepo.store_entry(&se).is_ok());
    se.parent_subs_id = 1; // unchanged folder pos, that's an error
    se.display_name = "Japan 無料ダウンロード".to_string();
    se.expanded = true;
    assert!(subsrepo.store_entry(&se).is_ok());
    se.display_name = "fourth".to_string();
    se.expanded = false;
    se.folder_position = 3;
    assert!(subsrepo.store_entry(&se).is_ok());


	se.display_name = "fifth".to_string();
    se.expanded = false;
    se.folder_position = 4;
    assert!(subsrepo.store_entry(&se).is_ok());	// 5


    // subsrepo.debug_dump_tree("###");
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





// ------------------------------------

#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(
            // 0,
            logger_config::QuietFlags::Downloader as u64,
        );
    });
}
