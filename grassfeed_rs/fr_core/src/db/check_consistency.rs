use crate::controller::contentlist::CJob;
use crate::controller::sourcetree::SJob;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::db_clean::CleanerInner;
use crate::downloader::db_clean::CleanerStart;
use crate::util::file_exists;
use crate::util::StepResult;

pub fn databases_check_manual(foldername: &str) {
    let set_undelete: bool = false;
    let subs_fn = SubscriptionRepo::filename(foldername);
    if !file_exists(&subs_fn) {
        error!("No file {} ", subs_fn);
        return;
    }
    let subs_copy = format!("{}.copy", subs_fn);
    std::fs::copy(&subs_fn, &subs_copy).unwrap();
    let subsrepo1 = SubscriptionRepo::by_file(&subs_fn);
    let all_subscriptions = subsrepo1.get_all_entries();
    debug!(
        "Start Check  Subscriptions: {}  #{}",
        &subs_fn,
        all_subscriptions.len()
    );
    let msg_fn = MessagesRepo::filename(foldername);
    let msg_copy = format!("{}.copy", msg_fn);
    std::fs::copy(&msg_fn, msg_copy).unwrap();
    let msgrepo1 = MessagesRepo::by_filename(&msg_fn);
    let all_messages = msgrepo1.get_all_messages();
    debug!("Messages  {}  #{}", &msg_fn, &all_messages.len());
    if set_undelete {
        debug!("setting all messages to undeleted!  ");
        let msg_ids: Vec<i32> = all_messages.iter().map(|m| m.message_id as i32).collect();
        msgrepo1.update_is_deleted_many(&msg_ids, false);
    }
    let (stc_job_s, stc_job_r) = flume::bounded::<SJob>(9);
    let (c_q_s, c_q_r) = flume::bounded::<CJob>(9);
    let cleaner_i = CleanerInner::new(c_q_s, stc_job_s, subsrepo1, msgrepo1);
    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));

    c_q_r.drain().for_each(|cjob| debug!("CJOB: {:?}", cjob));
    stc_job_r
        .drain()
        .for_each(|sjob| debug!("SJOB: {:?}", sjob));

    let parent_ids_to_correct = inner.fp_correct_subs_parent.lock().unwrap().clone();
    if !parent_ids_to_correct.is_empty() {
        debug!(" to_correct: {:?}", parent_ids_to_correct);
    }
    let all_messages = inner.messgesrepo.get_all_messages();
    let count_not_deleted = all_messages.iter().filter(|m| !m.is_deleted).count();
    debug!("After cleanup  #MESSAGES= #{}", count_not_deleted);
}
