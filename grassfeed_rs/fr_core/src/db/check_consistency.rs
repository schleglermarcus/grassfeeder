use crate::controller::contentlist::CJob;
use crate::controller::sourcetree::SJob;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::db_clean::CleanerInner;
use crate::downloader::db_clean::CleanerStart;
use crate::util::StepResult;

pub fn databases_consistency_check(foldername: &String) {
    databases_consistency_check_u(foldername, false, true);
}

pub fn databases_consistency_check_u(foldername: &String, set_undelete: bool, really_remove: bool) {
    let subs_fn = SubscriptionRepo::filename(foldername);
    let subs_copy = format!("{}.copy", subs_fn);
    std::fs::copy(subs_fn, subs_copy.clone()).unwrap();
    let subsrepo1 = SubscriptionRepo::by_file(&subs_copy);
    let all_subscriptions = subsrepo1.get_all_entries();
    debug!(
        "Start Check  Subscriptions: {}  #{}",
        &subs_copy,
        all_subscriptions.len()
    );
    let msg_fn = MessagesRepo::filename(foldername);
    let msg_copy = format!("{}.copy", msg_fn);
    std::fs::copy(msg_fn.clone(), msg_copy.clone()).unwrap();
    let msgrepo1 = MessagesRepo::by_filename(&msg_copy);
    let all_messages = msgrepo1.get_all_messages();
    debug!(
        "Messages  {}=>{}  #{}",
        &msg_fn,
        &msg_copy,
        &all_messages.len()
    );

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
    if really_remove {
        let to_delete: Vec<i32> = all_messages
            .iter()
            .filter_map(|m| {
                if m.is_deleted {
                    Some(m.message_id as i32)
                } else {
                    None
                }
            })
            .collect();
        let num_deleted = inner.messgesrepo.delete_by_index(&to_delete);
        if to_delete.len() != num_deleted {
            warn!("TO_DELETE: {}  DELETED:{}", to_delete.len(), num_deleted);
        } else {
            debug!("deleted {} messages", num_deleted);
        }
    }
    debug!("vacuuum subscriptions ... ");
    let v_sub = inner.subscriptionrepo.db_vacuum();
    debug!("vacuuum messages ... ");
    let v_msg = inner.messgesrepo.db_vacuum();
    debug!("vacuuum done #vmsg={}  #vsubs={}", v_msg, v_sub);

	let all_messages = inner.messgesrepo.get_all_messages();
    let count_not_deleted = all_messages.iter().filter(|m| !m.is_deleted).count();
    debug!("After cleanup  #MESSAGES= #{}", count_not_deleted);

    // inner        .messgesrepo        .get_all_messages()        .iter()        .for_each(|m| debug!("MSG {}", m));
}
