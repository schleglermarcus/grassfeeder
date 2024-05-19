use crate::controller::contentlist::CJob;
use crate::controller::guiprocessor::Job;
use crate::controller::sourcetree::SJob;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IIconRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::db_clean::CleanerInner;
use crate::downloader::db_clean::CleanerStart;
use crate::util::file_exists;
use crate::util::StepResult;

pub fn databases_check_manual(config_folder: &str, cache_folder: &str) {
    let subs_fn = SubscriptionRepo::filename(config_folder);
    if !file_exists(&subs_fn) {
        error!("No file {} ", subs_fn);
        return;
    }
    let subs_copy = format!("{subs_fn}.copy");
    std::fs::copy(&subs_fn, subs_copy).unwrap();
    let subsrepo1 = SubscriptionRepo::by_file(&subs_fn);
    let all_subscriptions = subsrepo1.get_all_entries();
    debug!(
        "Start Check  Subscriptions: {}  #{}",
        &subs_fn,
        all_subscriptions.len()
    );
    let msg_fn = MessagesRepo::filename(config_folder);
    let msg_copy = format!("{msg_fn}.copy");
    std::fs::copy(&msg_fn, msg_copy).unwrap();

    let msg_repo = MessagesRepo::new_by_filename_add_column(&msg_fn);
    let err_repo = ErrorRepo::new(cache_folder);
    let iconrepo = IconRepo::new(config_folder);
    iconrepo.create_table();
    let sum_all_msg = msg_repo.get_all_sum();
    trace!("{} has  {} Messages  ", &msg_fn, sum_all_msg,);
    let (stc_job_s, stc_job_r) = flume::bounded::<SJob>(9);
    let (gp_job_s, _gp_job_r) = flume::bounded::<Job>(9);
    let (_c_q_s, c_q_r) = flume::bounded::<CJob>(9);
    let cleaner_i = CleanerInner::new(
        gp_job_s, stc_job_s, subsrepo1, msg_repo, iconrepo, 100000, err_repo,
    );
    let inner = StepResult::start(Box::new(CleanerStart::new(cleaner_i)));
    c_q_r.drain().for_each(|cjob| debug!("CJOB: {:?}", cjob));
    let sjobs = stc_job_r.drain().collect::<Vec<SJob>>();
    let parent_ids_to_correct = inner.fp_correct_subs_parent.lock().unwrap().clone();
    if !parent_ids_to_correct.is_empty() {
        debug!(" to_correct: {:?} {:?} ", parent_ids_to_correct, sjobs);
    }
    let sum_all_msg = inner.messagesrepo.get_all_sum();
    debug!("After cleanup  number of messages: {}", sum_all_msg);
}
