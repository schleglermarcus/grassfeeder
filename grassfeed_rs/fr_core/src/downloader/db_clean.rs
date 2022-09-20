use crate::controller::contentlist::CJob;
use crate::controller::sourcetree::SJob;
use crate::db::message::decompress;
use crate::db::message::MessageRow;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::util::db_time_to_display;
use crate::util::filter_by_iso8859_1;
use crate::util::Step;
use crate::util::StepResult;
use flume::Sender;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::sync::Mutex;

pub struct CleanerInner {
    pub cjob_sender: Sender<CJob>,
    pub sourcetree_job_sender: Sender<SJob>,
    pub subscriptionrepo: SubscriptionRepo,
    pub messgesrepo: MessagesRepo,
    pub fp_correct_subs_parent: Mutex<Vec<i32>>,
    pub subs_parents_active: Mutex<Vec<i32>>,
    pub need_update_subscriptions: bool,
    pub need_update_messages: bool,
    /// -1 : do not check
    pub max_messages_per_subscription: i32,
}

impl CleanerInner {
    pub fn new(
        c_se: Sender<CJob>,
        s_se: Sender<SJob>,
        sub_re: SubscriptionRepo,
        msg_re: MessagesRepo,
    ) -> Self {
        CleanerInner {
            cjob_sender: c_se,
            sourcetree_job_sender: s_se,
            subscriptionrepo: sub_re,
            messgesrepo: msg_re,
            fp_correct_subs_parent: Mutex::new(Vec::default()),
            subs_parents_active: Mutex::new(Vec::default()),
            need_update_messages: false,
            need_update_subscriptions: false,
            max_messages_per_subscription: -1,
        }
    }
}

impl PartialEq for CleanerInner {
    fn eq(&self, _other: &Self) -> bool {
        true // only one element shall be in the queue
    }
}

impl std::fmt::Debug for CleanerInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("san_par", &self.fp_correct_subs_parent.lock().unwrap())
            .finish()
    }
}

impl CleanerStart {
    pub fn new(i: CleanerInner) -> Self {
        CleanerStart(i)
    }
}
pub struct CleanerStart(pub CleanerInner);
impl Step<CleanerInner> for CleanerStart {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        StepResult::Continue(Box::new(RemoveNonConnected(self.0)))
    }
}

pub struct RemoveNonConnected(pub CleanerInner);
impl Step<CleanerInner> for RemoveNonConnected {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        let all_subs = inner.subscriptionrepo.get_all_entries();
        let mut connected_child_list: HashSet<isize> = HashSet::default();
        let mut folder_todo: Vec<isize> = Vec::default();
        folder_todo.push(0);
        while !folder_todo.is_empty() {
            let parent_subs_id = folder_todo.pop().unwrap();
            let childs = inner.subscriptionrepo.get_by_parent_repo_id(parent_subs_id);
            childs.iter().for_each(|se| {
                connected_child_list.insert(se.subs_id);
                if se.is_folder {
                    folder_todo.push(se.subs_id);
                }
            });
        }
        let mut delete_list: HashSet<isize> = HashSet::default();
        all_subs.iter().for_each(|se| {
            if se.deleted || se.parent_subs_id < 0 {
                delete_list.insert(se.subs_id);
            } else if !connected_child_list.contains(&se.subs_id) {
                if delete_list.contains(&se.parent_subs_id) {
                    delete_list.insert(se.subs_id);
                } else {
                    debug!("Cleanup:  NotConnectedSubscription: {}", &se);
                }
            }
        });
        if delete_list.len() > 3 {
            debug!(
                "Cleanup:  #connected: {}   #to_delete: {}",
                connected_child_list.len(),
                delete_list.len()
            );
            delete_list
                .iter()
                .for_each(|id| inner.subscriptionrepo.delete_by_index(*id));
            inner.need_update_subscriptions = true;
        }
        StepResult::Continue(Box::new(AnalyzeFolderPositions(inner)))
    }
}

pub struct AnalyzeFolderPositions(pub CleanerInner);
impl Step<CleanerInner> for AnalyzeFolderPositions {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        check_layer(
            &Vec::<u16>::default(),
            0,
            &inner.subscriptionrepo,
            &inner.fp_correct_subs_parent,
            &inner.subs_parents_active,
        );
        let to_correct_a = &inner.fp_correct_subs_parent.lock().unwrap().clone();
        if to_correct_a.is_empty() {
            StepResult::Continue(Box::new(CorrectNames(inner)))
        } else {
            inner.need_update_subscriptions = true;
            StepResult::Continue(Box::new(ReSortParentId(inner)))
        }
    }
}

pub struct ReSortParentId(pub CleanerInner);
impl Step<CleanerInner> for ReSortParentId {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let inner = self.0;
        let mut parent_ids: Vec<i32> = inner.fp_correct_subs_parent.lock().unwrap().clone();
        parent_ids.sort();
        parent_ids.dedup();
        if !parent_ids.is_empty() {
            debug!("Cleanup: resorting {:?}", parent_ids);
            parent_ids.iter().for_each(|p| {
                resort_parent_list(*p as isize, &inner.subscriptionrepo);
            });
        }
        StepResult::Continue(Box::new(CorrectNames(inner)))
    }
}

pub struct CorrectNames(pub CleanerInner);
impl Step<CleanerInner> for CorrectNames {
    //  Correct all Folder names that are empty
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner
            .subscriptionrepo
            .get_all_entries()
            .iter()
            .for_each(|fse| {
                if fse.display_name.is_empty() {
                    inner
                        .subscriptionrepo
                        .update_displayname(fse.subs_id, format!("unnamed-{}", fse.subs_id));
                    inner.need_update_subscriptions = true;
                } else {
                    let (filtered, truncated) = filter_by_iso8859_1(&fse.display_name);
                    if truncated {
                        inner
                            .subscriptionrepo
                            .update_displayname(fse.subs_id, filtered);
                        inner.need_update_subscriptions = true;
                    }
                }
            });
        StepResult::Continue(Box::new(CollapseSubscriptions(inner)))
    }
}

pub struct CollapseSubscriptions(pub CleanerInner);
impl Step<CleanerInner> for CollapseSubscriptions {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        let collapse_ids = inner
            .subscriptionrepo
            .get_all_entries()
            .iter()
            .filter(|fse| !fse.is_folder && fse.expanded)
            .map(|fse| fse.subs_id)
            .collect::<Vec<isize>>();
        if !collapse_ids.is_empty() {
            debug!("Cleanup:  collapsing folders: {:?}", collapse_ids);
            inner.subscriptionrepo.update_expanded(collapse_ids, false);
            inner.need_update_subscriptions = true;
        }
        StepResult::Continue(Box::new(MarkUnconnectedMessages(inner)))
    }
}

pub struct MarkUnconnectedMessages(pub CleanerInner);
impl Step<CleanerInner> for MarkUnconnectedMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        let parent_ids_active: Vec<i32> = inner
            .subscriptionrepo
            .get_all_nonfolder()
            .iter()
            .filter(|se| !se.isdeleted())
            .map(|se| se.subs_id as i32)
            .collect();
        let noncon_ids = inner
            .messgesrepo
            .get_src_not_contained(&parent_ids_active)
            .iter()
            .map(|fse| fse.message_id as i32)
            .collect::<Vec<i32>>();
        if !noncon_ids.is_empty() {
            if noncon_ids.len() < 100 && parent_ids_active.len() < 100 {
                debug!(
                    "Cleanup: not connected messages={:?}   parent-ids={:?}",
                    &noncon_ids, &parent_ids_active
                );
            } else {
                debug!(
                    "Cleanup: not connected messages: {}   parent-ids: {}",
                    &noncon_ids.len(),
                    &parent_ids_active.len()
                );
            }
            inner.need_update_messages = true;
            inner.messgesrepo.update_is_deleted_many(&noncon_ids, true);
        }

        StepResult::Continue(Box::new(ReduceTooManyMessages(inner)))
    }
}

pub struct ReduceTooManyMessages(pub CleanerInner);
impl Step<CleanerInner> for ReduceTooManyMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        if inner.max_messages_per_subscription > 1 {
            let subs_ids = inner
                .subscriptionrepo
                .get_all_entries()
                .iter()
                .filter(|fse| !fse.is_folder)
                .map(|fse| fse.subs_id)
                .collect::<Vec<isize>>();
            trace!(
                "ReduceTooManyMessages={:?}  SUBS={:?}",
                inner.max_messages_per_subscription,
                &subs_ids
            );

            for su_id in &subs_ids {
                let mut msg_per_subscription = inner.messgesrepo.get_by_src_id(*su_id, true);
                if msg_per_subscription.len() > inner.max_messages_per_subscription as usize {
                    inner.need_update_messages = true;
                    msg_per_subscription.sort_by(|a, b| b.entry_src_date.cmp(&a.entry_src_date));
                    let (_stay, remove) =
                        msg_per_subscription.split_at(inner.max_messages_per_subscription as usize);
                    // _stay.iter().for_each(|e| {                        debug!(                            "STAY  {}\t{}",                            e.message_id,                            db_time_to_display(e.entry_src_date)                        )                    });
                    //  remove.iter().for_each(|e| {                        debug!(                            "REMOVE  {}\t{}",                            e.message_id,                            db_time_to_display(e.entry_src_date)                        )                    });

                    if !remove.is_empty() {
                        let id_list: Vec<i32> =
                            remove.iter().map(|e| e.message_id as i32).collect();
                        let first_msg = remove.iter().next().unwrap();
                        debug!(
                            "Cleanup messages:  subsciption(id {}) ,  {} messages. Latest date: {}	\t message-ids={:?}",
                            su_id,
                            id_list.len(),
                            db_time_to_display(first_msg.entry_src_date),
                            id_list
                        );
                        inner.messgesrepo.update_is_deleted_many(&id_list, true);
                    }
                }
            }
        }
        StepResult::Continue(Box::new(DeleteDoubleSameMessages(inner)))
    }
}

pub struct DeleteDoubleSameMessages(pub CleanerInner);
impl Step<CleanerInner> for DeleteDoubleSameMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let inner = self.0;
        let subs_ids_active: Vec<i32> = inner
            .subscriptionrepo
            .get_all_nonfolder()
            .iter()
            .map(|se| se.subs_id as i32)
            .collect();
        for subs_id in subs_ids_active {
            let mut msglist: Vec<MessageRow> =
                inner.messgesrepo.get_by_src_id(subs_id as isize, true);
            if msglist.is_empty() {
                continue;
            }
            msglist.sort_by(|a, b| a.fetch_date.cmp(&b.fetch_date));

            let mut known: HashSet<(i64, String)> = HashSet::new();
            let mut delete_list: Vec<MessageRow> = Vec::default();
            msglist.iter().for_each(|msg| {
                if known.contains(&(msg.entry_src_date, msg.title.clone())) {
                    delete_list.push(msg.clone());
                } else {
                    known.insert((msg.entry_src_date, msg.title.clone()));
                };
            });
            for d in &delete_list {
                trace!(
                    "{} double: ID:{}\tdate:{} fetch:{}\t{}",
                    subs_id,
                    d.message_id,
                    db_time_to_display(d.entry_src_date),
                    db_time_to_display(d.fetch_date),
                    decompress(&d.title),
                );
            }
            let del_indices: Vec<i32> = delete_list.iter().map(|m| m.message_id as i32).collect();
            debug!("setting deleted: {} messages", del_indices.len());
            inner
                .messgesrepo
                .update_is_deleted_many(del_indices.as_slice(), true);
        }

        StepResult::Continue(Box::new(Notify(inner)))
    }
}

// later : clean out all deleted messages

pub struct Notify(pub CleanerInner);
impl Step<CleanerInner> for Notify {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let inner = self.0;
        if inner.need_update_subscriptions {
            let _r = inner.sourcetree_job_sender.send(SJob::FillSourcesTree);
            let _r = inner.sourcetree_job_sender.send(SJob::UpdateTreePaths);
        }
        // later: refresh message display
        StepResult::Stop(inner)
    }
}

// Walk the Path downwards and find all  parents with   folder-pos not in a row.
// Recursive
pub fn check_layer(
    localpath: &[u16],
    parent_subs_id: i32,
    subs_repo: &SubscriptionRepo,
    fp_correct_subs_parent: &Mutex<Vec<i32>>,
    subs_active_parents: &Mutex<Vec<i32>>,
) {
    let entries = (*subs_repo)
        .borrow()
        .get_by_parent_repo_id(parent_subs_id as isize);
    if !entries.is_empty() {
        subs_active_parents.lock().unwrap().push(parent_subs_id);
    }
    // debug!("check_layer: {:?} {:?}", &localpath, &entries);
    entries.iter().enumerate().for_each(|(folderpos, fse)| {
        let mut path: Vec<u16> = Vec::new();
        path.extend_from_slice(localpath);
        if fse.folder_position != (folderpos as isize) {
            // debug!(                "check_layer: unequal folderpos {:?} {:?}",                fse.folder_position, folderpos            );

            let mut fpc = fp_correct_subs_parent.lock().unwrap();
            if !fpc.contains(&(fse.parent_subs_id as i32)) {
                fpc.push(fse.parent_subs_id as i32);
            }
        }
        path.push(fse.folder_position as u16);
        check_layer(
            &path,
            fse.subs_id as i32,
            subs_repo,
            fp_correct_subs_parent,
            subs_active_parents,
        );
    });
}

/// straightens the folder_pos
pub fn resort_parent_list(parent_subs_id: isize, subscriptionrepo: &SubscriptionRepo) {
    let mod_list = subscriptionrepo.get_by_parent_repo_id(parent_subs_id);
    mod_list.iter().enumerate().for_each(|(n, fse)| {
        if fse.folder_position != n as isize {
            subscriptionrepo.update_folder_position(fse.subs_id, n as isize);
        }
    });
}
