use crate::controller::contentlist::CJob;
use crate::controller::sourcetree::SJob;
use crate::db::errors_repo::ErrorEntry;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::message::MessageRow;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::util::filter_by_iso8859_1;
use crate::util::timestamp_now;
use crate::util::Step;
use crate::util::StepResult;
use flume::Sender;
use resources::gen_icons;
use resources::gen_icons::ICON_LIST;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::sync::Mutex;

/// Later:  sanity for recursion
pub const MAX_PATH_DEPTH: usize = 30;

pub const MAX_ERROR_LINES_PER_SUBSCRIPTION: usize = 100;
pub const MAX_ERROR_LINE_AGE_S: usize = 60 * 60 * 24 * 360;

pub struct CleanerInner {
    pub cjob_sender: Sender<CJob>,
    pub sourcetree_job_sender: Sender<SJob>,
    pub subscriptionrepo: SubscriptionRepo,
    pub messgesrepo: MessagesRepo,
    pub iconrepo: IconRepo,
    pub error_repo: ErrorRepo,
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
        ico_re: IconRepo,
        max_msg: i32,
        err_re: ErrorRepo,
    ) -> Self {
        CleanerInner {
            cjob_sender: c_se,
            sourcetree_job_sender: s_se,
            subscriptionrepo: sub_re,
            messgesrepo: msg_re,
            iconrepo: ico_re,
            fp_correct_subs_parent: Mutex::new(Vec::default()),
            subs_parents_active: Mutex::new(Vec::default()),
            need_update_messages: false,
            need_update_subscriptions: false,
            max_messages_per_subscription: max_msg,
            error_repo: err_re,
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
        let mut folder_work: Vec<isize> = Vec::default();
        folder_work.push(0);
        while !folder_work.is_empty() {
            let parent_subs_id = folder_work.pop().unwrap();
            let childs = inner.subscriptionrepo.get_by_parent_repo_id(parent_subs_id);
            childs.iter().for_each(|se| {
                connected_child_list.insert(se.subs_id);
                if se.is_folder {
                    folder_work.push(se.subs_id);
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
                "Sanitize Subscriptions:  #connected: {}   #to_delete: {}",
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

///  Correct all Folder names that are empty
pub struct CorrectNames(pub CleanerInner);
impl Step<CleanerInner> for CorrectNames {
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
        StepResult::Continue(Box::new(CorrectIconsOfFolders(inner)))
    }
}

pub struct CorrectIconsOfFolders(pub CleanerInner);
impl Step<CleanerInner> for CorrectIconsOfFolders {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        let all_folders: Vec<SubscriptionEntry> = inner
            .subscriptionrepo
            .get_all_entries()
            .into_iter()
            .filter(|fse| fse.is_folder)
            .collect();
        let mut reset_icon_subs_ids: Vec<i32> = all_folders
            .iter()
            .filter_map(|se| {
                if se.icon_id != gen_icons::IDX_08_GNOME_FOLDER_48 {
                    Some(se.subs_id as i32)
                } else {
                    None
                }
            })
            .collect::<Vec<i32>>();
        reset_icon_subs_ids.sort();
        if !reset_icon_subs_ids.is_empty() {
            debug!("CorrectIconsOfFolders: {:?}", reset_icon_subs_ids);
            inner
                .subscriptionrepo
                .update_icon_id_many(reset_icon_subs_ids, gen_icons::IDX_08_GNOME_FOLDER_48);
            inner.need_update_subscriptions = true;
        }
        StepResult::Continue(Box::new(CorrectIconsOnSubscriptions(inner)))
    }
}

pub struct CorrectIconsOnSubscriptions(pub CleanerInner);
impl Step<CleanerInner> for CorrectIconsOnSubscriptions {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        let all_folders: Vec<SubscriptionEntry> = inner
            .subscriptionrepo
            .get_all_entries()
            .into_iter()
            .filter(|fse| !fse.is_folder)
            .collect();
        let mut reset_icon_subs_ids: Vec<i32> = Vec::default();
        for se in all_folders {
            if se.icon_id < ICON_LIST.len() && se.icon_id != gen_icons::IDX_05_RSS_FEEDS_GREY_64_D {
                reset_icon_subs_ids.push(se.subs_id as i32);
                continue;
            }
            let o_icon = inner.iconrepo.get_by_index(se.icon_id as isize);
            if o_icon.is_none() {
                reset_icon_subs_ids.push(se.subs_id as i32);
                continue;
            }
        }
        reset_icon_subs_ids.sort();
        if !reset_icon_subs_ids.is_empty() {
            debug!("CorrectIconsOnSubscriptions : {:?}", reset_icon_subs_ids);
            inner
                .subscriptionrepo
                .update_icon_id_many(reset_icon_subs_ids, gen_icons::IDX_05_RSS_FEEDS_GREY_64_D);
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
            .filter(|se| !se.is_deleted)
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
            // trace!(                "ReduceTooManyMessages(max={})  #folders:{}",                inner.max_messages_per_subscription,                subs_ids.len()            );
            for su_id in &subs_ids {
                let mut msg_per_subscription = inner.messgesrepo.get_by_src_id(*su_id, true);
                let length_before = msg_per_subscription.len();
                if length_before > inner.max_messages_per_subscription as usize {
                    inner.need_update_messages = true;
                    msg_per_subscription.sort_by(|a, b| b.entry_src_date.cmp(&a.entry_src_date));
                    let (_stay, remove) =
                        msg_per_subscription.split_at(inner.max_messages_per_subscription as usize);
                    if !remove.is_empty() {
                        let id_list: Vec<i32> =
                            remove.iter().map(|e| e.message_id as i32).collect();
                        //  let first_msg = remove.iter().next().unwrap();
                        // trace!(                            "Reduce(ID {}), has {}, reduce {} messages. Latest date: {}	", // \t message-ids={:?}                            su_id,                            length_before,                            id_list.len(),                            db_time_to_display(first_msg.entry_src_date),                        );
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
            // for d in &delete_list {                trace!(                    "ID{} == ID:{}\tdate:{} fetch:{}\t{}  post-id:{}",                    subs_id,                    d.message_id,                    db_time_to_display(d.entry_src_date),                    db_time_to_display(d.fetch_date),                    crate::db::message::decompress(&d.title),                    d.post_id                );            }
            if !delete_list.is_empty() {
                let del_indices: Vec<i32> =
                    delete_list.iter().map(|m| m.message_id as i32).collect();
                inner
                    .messgesrepo
                    .update_is_deleted_many(del_indices.as_slice(), true);
            }
        }
        StepResult::Continue(Box::new(PurgeMessages(inner)))
    }
}

pub struct PurgeMessages(pub CleanerInner);
impl Step<CleanerInner> for PurgeMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let inner = self.0;
        let allmsg = inner.messgesrepo.get_all_messages();
        let all_count = allmsg.len();
        let to_delete: Vec<i32> = allmsg
            .into_iter()
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
            warn!(
                "PurgeMessages: #all={}   TO_DELETE: {}  DELETED:{}",
                all_count,
                to_delete.len(),
                num_deleted,
            );
        } else if num_deleted > 0 {
            debug!(
                "PurgeMessages: #all={}  Deleted {} messages",
                all_count, num_deleted
            );
        }
        StepResult::Continue(Box::new(CheckErrorLog(inner)))
    }
}

pub struct CheckErrorLog(pub CleanerInner);
impl Step<CleanerInner> for CheckErrorLog {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let inner = self.0;
        let parent_ids_active: HashSet<isize> = inner
            .subscriptionrepo
            .get_all_nonfolder()
            .iter()
            .filter(|se| !se.isdeleted())
            .map(|se| se.subs_id as isize)
            .collect::<HashSet<isize>>();
        inner.error_repo.startup_read();
        let list: Vec<ErrorEntry> = inner.error_repo.get_all_stored_entries();
        let num_errors_before = list.len();
        let subs_ids: Vec<isize> = parent_ids_active.into_iter().collect();
        let (mut new_errors, msg) = filter_error_entries(&list, subs_ids);
        if new_errors.len() < num_errors_before {
            debug!(
                "reduced error lines: {}->{}  {}",
                num_errors_before,
                new_errors.len(),
                msg
            );
            new_errors.sort_by(|a, b| a.err_id.cmp(&b.err_id));
            inner.error_repo.replace_errors_file(new_errors);
        }
        StepResult::Continue(Box::new(Notify(inner)))
    }
}

pub struct Notify(pub CleanerInner);
impl Step<CleanerInner> for Notify {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let inner = self.0;
        inner.subscriptionrepo.db_vacuum();
        inner.messgesrepo.db_vacuum();
        if inner.need_update_subscriptions {
            let _r = inner.sourcetree_job_sender.send(SJob::FillSourcesTree);
            let _r = inner.sourcetree_job_sender.send(SJob::UpdateTreePaths);
        }
        // later: refresh message display
        StepResult::Stop(inner)
    }
}

pub fn filter_error_entries(
    existing: &[ErrorEntry],
    subs_ids: Vec<isize>,
) -> (Vec<ErrorEntry>, String) {
    let subs_ids_h: HashSet<isize> = if subs_ids.is_empty() {
        existing.iter().map(|l| l.subs_id).collect()
    } else {
        subs_ids.into_iter().collect()
    };
    let mut new_errors: Vec<ErrorEntry> = Vec::default();
    let mut msg = String::default();
    for subs_id in subs_ids_h {
        let mut errors: Vec<ErrorEntry> = existing
            .iter()
            .filter(|e| e.subs_id == subs_id)
            .cloned()
            .collect();
        let previous_len = errors.len();
        let min_date: i64 = timestamp_now() - MAX_ERROR_LINE_AGE_S as i64;
        errors.sort_by(|a, b| a.date.cmp(&b.date));
        // errors            .iter()            .filter(|e| e.date <= min_date)            .for_each(|e| trace!("too-old: {:?}", e));
        errors = errors
            .iter()
            .filter(|e| e.date > min_date)
            .cloned()
            .collect::<Vec<ErrorEntry>>();
        let deleted_by_date = previous_len - errors.len();
        let mut deleted_by_sum = errors.len() as isize - MAX_ERROR_LINES_PER_SUBSCRIPTION as isize;
        if deleted_by_sum > 0 {
            let (p0, _p1) = errors.split_at(errors.len() - MAX_ERROR_LINES_PER_SUBSCRIPTION);
            // p1.iter().for_each(|e| trace!("too-many: {:?}", e));
            errors = p0.to_vec();
        } else {
            deleted_by_sum = 0;
        }
        if errors.len() < previous_len {
            msg.push_str(&format!(
                "ID{} B{} A{} S{} \t",
                subs_id, previous_len, deleted_by_date, deleted_by_sum
            ));
        }
        errors.into_iter().for_each(|e| new_errors.push(e));
    }
    new_errors.sort_by(|a, b| a.err_id.cmp(&b.err_id));
    (new_errors, msg)
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
    if localpath.len() == MAX_PATH_DEPTH {
        warn!(
            "Subscriptions nested too deep: {} back to top level.",
            parent_subs_id
        );
        subs_repo.update_parent_and_folder_position(parent_subs_id as isize, 0, 0);
    }
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
