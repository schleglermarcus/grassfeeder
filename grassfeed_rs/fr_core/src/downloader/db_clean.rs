use crate::controller::contentlist::CJob;
use crate::controller::sourcetree::SJob;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::util::Step;
use crate::util::StepResult;
use crate::util::filter_by_iso8859_1;
use flume::Sender;
use std::borrow::Borrow;
use std::sync::Mutex;


pub struct CleanerInner {
    pub cjob_sender: Sender<CJob>,
    pub sourcetree_job_sender: Sender<SJob>,
    pub subscriptionrepo: SubscriptionRepo,
    pub messgesrepo: MessagesRepo,
    // pub fp_correct_subs_parent: RefCell<Vec<i32>>,
    // pub subs_parents_active: RefCell<Vec<i32>>,
    pub fp_correct_subs_parent: Mutex<Vec<i32>>,
    pub subs_parents_active: Mutex<Vec<i32>>,
    pub need_update_subscriptions: bool,
    pub need_update_messages: bool,
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
        let mut inner = self.0;
        check_layer(
            &Vec::<u16>::default(),
            0,
            &inner.subscriptionrepo,
            &inner.fp_correct_subs_parent,
            &inner.subs_parents_active,
        );
        // trace!("correct_p: {:?}", &inner.fp_correct_subs_parent.borrow());
        if inner.fp_correct_subs_parent.lock().unwrap().is_empty() {
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
        parent_ids.iter().for_each(|p| {
            debug!("restoring parent_ids {}", p);
            resort_parent_list(*p as isize, &inner.subscriptionrepo);
        });
        StepResult::Continue(Box::new(CorrectNames(inner)))
    }
}

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
                    // trace!("SO:  no display name for : {:?} ", fse);
                    inner
                        .subscriptionrepo
                        .update_displayname(fse.subs_id, format!("unnamed-{}", fse.subs_id));
                    inner.need_update_subscriptions = true;
                } else {
                    let (filtered, truncated) = filter_by_iso8859_1(&fse.display_name);
                    if truncated {
                        // trace!("SO: name change to iso8859 : {:?} ", fse);
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
            // debug!("sanitizing collapse: {:?}", collapse_ids);
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
        let parent_ids_active: Vec<i32> = inner.subs_parents_active.lock().unwrap().clone();
        let noncon_ids = inner
            .messgesrepo
            .get_src_not_contained(&parent_ids_active)
            .iter()
            .map(|fse| fse.message_id as i32)
            .collect::<Vec<i32>>();
        // trace!("nonconnected: {:?}", &noncon_ids);
        if !noncon_ids.is_empty() {
            inner.need_update_messages = true;
            inner.messgesrepo.update_is_deleted_many(&noncon_ids, true);
        }

        StepResult::Continue(Box::new(Notify(inner)))
    }
}

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
//  Correct all Folder names that are empty
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
    entries.iter().enumerate().for_each(|(n, fse)| {
        let mut path: Vec<u16> = Vec::new();
        path.extend_from_slice(localpath);
        if fse.folder_position != (n as isize) {
            fp_correct_subs_parent
                .lock()
                .unwrap()
                .push(fse.parent_subs_id as i32);
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
