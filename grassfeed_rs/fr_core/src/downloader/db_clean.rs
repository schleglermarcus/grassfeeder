use crate::controller::guiprocessor::Job;
use crate::controller::sourcetree::SJob;
use crate::db::errorentry::ErrorEntry;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IIconRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::icon_row::IconRow;
use crate::db::message::MessageRow;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_DUMMY;
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
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Instant;

///  deadlock protection in case of recursion in database
pub const MAX_PATH_DEPTH: usize = 30;
pub const MAX_ERROR_LINES_PER_SUBSCRIPTION: usize = 100;
pub const MAX_ERROR_LINE_AGE_S: usize = 60 * 60 * 24 * 360;
pub const CLEAN_STEPS_MAX: u8 = 10;

pub struct CleanerInner {
    pub gp_job_sender: Sender<Job>,
    pub sourcetree_job_sender: Sender<SJob>,
    pub subscriptionrepo: SubscriptionRepo,
    pub messagesrepo: MessagesRepo,
    pub iconrepo: IconRepo,
    pub error_repo: ErrorRepo,
    pub fp_correct_subs_parent: Mutex<Vec<i32>>,
    pub subs_parents_active: Mutex<Vec<i32>>,
    pub need_update_subscriptions: bool,
    pub need_update_messages: bool,
    /// -1 : do not check
    pub max_messages_per_subscription: i32,
    pub starttime: Instant,
    pub stepmarker: u8,
}

impl CleanerInner {
    pub fn new(
        gp_se: Sender<Job>,
        s_se: Sender<SJob>,
        sub_re: SubscriptionRepo,
        msg_re: MessagesRepo,
        ico_re: IconRepo,
        max_msg: i32,
        err_re: ErrorRepo,
    ) -> Self {
        CleanerInner {
            gp_job_sender: gp_se,
            sourcetree_job_sender: s_se,
            subscriptionrepo: sub_re,
            messagesrepo: msg_re,
            iconrepo: ico_re,
            fp_correct_subs_parent: Mutex::new(Vec::default()),
            subs_parents_active: Mutex::new(Vec::default()),
            need_update_messages: false,
            need_update_subscriptions: false,
            max_messages_per_subscription: max_msg,
            error_repo: err_re,
            starttime: Instant::now(),
            stepmarker: 0,
        }
    }

    fn send_gp(&self, msg: Option<String>) {
        let el_ms = self.starttime.elapsed().as_millis() as u32;
        let mut msg_cut = match msg {
            Some(ref m) => m.clone(),
            None => String::default(),
        };
        msg_cut.truncate(80);
        trace!(
            "send_gp: {}ms \tmap {}=>{} \t{} ",
            el_ms,
            self.stepmarker,
            NS[self.stepmarker as usize],
            msg_cut
        );
        let _r =
            self.gp_job_sender
                .send(Job::NotifyDbClean(NS[self.stepmarker as usize], el_ms, msg));
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    fn advance_step(&mut self) {
        self.stepmarker += 1;
    }
    fn advance_to(&mut self, newvalue: u8) {
        self.stepmarker = newvalue;
        // debug!("ADVANCE-TO {} ", newvalue);
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

// Mapping DB-Check-States  to Gui-Level-Bars. This allows to shift the display levels according to time usage.
pub const NS: [u8; 25] = [
    0, 0, 0, 0, 0, // 0-4
    0, 0, 0, 1, 1, // 5-9
    1, 2, 3, // 10-12 ReduceTooManyMessages
    3, 4, 4, // 13-15: DeleteDoubleSame
    5, 6, // 16-17 PurgeMessages
    7, 8, // 18-19 :
    9, 10, 10, 10, 10, // 20-25
];

impl CleanerStart {
    pub fn new(i: CleanerInner) -> Self {
        CleanerStart(i)
    }
}
pub struct CleanerStart(pub CleanerInner);
impl Step<CleanerInner> for CleanerStart {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        self.0.send_gp(Some("Start".to_string()));
        StepResult::Continue(Box::new(RemoveNonConnectedSubscriptions(self.0)))
    }
}

pub struct RemoveNonConnectedSubscriptions(pub CleanerInner);
impl Step<CleanerInner> for RemoveNonConnectedSubscriptions {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None);
        let all_subs = inner.subscriptionrepo.get_all_entries();
        let mut connected_child_list: HashSet<isize> = HashSet::default();
        let mut folder_work: Vec<isize> = Vec::default();
        folder_work.push(0);
        while let Some(parent_subs_id) = folder_work.pop() {
            let childs = inner.subscriptionrepo.get_children(parent_subs_id);
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
                    inner.send_gp(Some(format!(" NotConnectedSubscription: {}", &se)));
                }
            }
        });
        if delete_list.len() > 3 {
            inner.send_gp(Some(format!(
                "Sanitize Subscriptions:  #connected: {}   #to_delete: {}",
                connected_child_list.len(),
                delete_list.len()
            )));
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
        inner.advance_step();
        inner.send_gp(None);
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

    fn take(self: Box<Self>) -> CleanerInner {
        self.0
    }
}

pub struct ReSortParentId(pub CleanerInner);
impl Step<CleanerInner> for ReSortParentId {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None);
        let mut parent_ids: Vec<i32> = inner.fp_correct_subs_parent.lock().unwrap().clone();
        parent_ids.sort();
        parent_ids.dedup();
        if !parent_ids.is_empty() {
            inner.send_gp(Some(format!(" resorting {:?}", parent_ids)));
            parent_ids.iter().for_each(|p| {
                resort_parent_list(*p as isize, &inner.subscriptionrepo);
            });
        }
        StepResult::Continue(Box::new(CorrectNames(inner)))
    }
    fn take(self: Box<Self>) -> CleanerInner {
        self.0
    }
}

///  Correct all Folder names that are empty
pub struct CorrectNames(pub CleanerInner);
impl Step<CleanerInner> for CorrectNames {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None);
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
        inner.advance_step();
        inner.send_gp(None);
        let collapse_ids = inner
            .subscriptionrepo
            .get_all_entries()
            .iter()
            .filter(|fse| !fse.is_folder && fse.expanded)
            .map(|fse| fse.subs_id)
            .collect::<Vec<isize>>();
        if !collapse_ids.is_empty() {
            inner.send_gp(Some(format!(" collapsing folders: {:?}", collapse_ids)));
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
        inner.advance_step();
        inner.send_gp(None);
        let all_folders: Vec<SubscriptionEntry> = inner
            .subscriptionrepo
            .get_all_entries()
            .into_iter()
            .filter(|fse| fse.is_folder)
            .collect();
        let mut reset_icon_subs_ids: Vec<i32> = all_folders
            .iter()
            .filter(|se| se.subs_id >= 0 && se.subs_id != SRC_REPO_ID_DUMMY)
            .filter(|se| se.icon_id != gen_icons::IDX_08_GNOME_FOLDER_48)
            .map(|se| se.subs_id as i32)
            .collect::<Vec<i32>>();
        reset_icon_subs_ids.sort(); //  -3,-2  is always in the list
        if !reset_icon_subs_ids.is_empty() {
            inner.send_gp(Some(format!(
                "CorrectIconsOfFolders:  IDS={:?}",
                reset_icon_subs_ids
            )));
            inner
                .subscriptionrepo
                .update_icon_id_many(reset_icon_subs_ids, gen_icons::IDX_08_GNOME_FOLDER_48);
            inner.need_update_subscriptions = true;
        }
        StepResult::Continue(Box::new(CorrectIconsDoublettes(inner)))
    }
}

pub struct CorrectIconsDoublettes(pub CleanerInner);
impl Step<CleanerInner> for CorrectIconsDoublettes {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None);
        let all_icons: Vec<IconRow> = inner.iconrepo.get_all_entries();
        let mut ic_first: HashMap<String, isize> = HashMap::new();
        let mut replace_ids: HashMap<isize, isize> = HashMap::new(); // subsequent-icon-id =>  previous icon-id
        all_icons
            .into_iter()
            .for_each(|e| match ic_first.get(&e.icon) {
                None => {
                    ic_first.insert(e.icon, e.icon_id);
                }
                Some(id) => {
                    replace_ids.insert(e.icon_id, *id);
                }
            });
        let all_subs = inner.subscriptionrepo.get_all_nonfolder();
        let mut changes: Vec<String> = Vec::default();
        replace_ids.iter().for_each(|(repl, dest)| {
            all_subs
                .iter()
                .filter(|subs| subs.icon_id == *repl as usize)
                .for_each(|subs| {
                    let m = format!("subs{} icon-id{}=>{}  ", subs.subs_id, subs.icon_id, dest);
                    changes.push(m);
                    inner
                        .subscriptionrepo
                        .update_icon_id(subs.subs_id, *dest as usize)
                });
        });
        inner.advance_step();
        if !changes.is_empty() {
            inner.send_gp(Some(format!(
                "CorrectIconsDoublettes changes: {:?} ",
                changes
            )));
        }
        if !replace_ids.is_empty() {
            inner.send_gp(Some(format!(
                "IconsDoublettes:  removing double icons: {:?} ",
                replace_ids.keys()
            )));
            replace_ids.iter().for_each(|(repl, _dest)| {
                // inner.iconrepo.remove_icon(*repl);
                inner.iconrepo.delete_icon(*repl);
            });
            // inner.iconrepo.check_or_store();
        }
        StepResult::Continue(Box::new(CorrectIconsOnSubscriptions(inner)))
    }
}

pub struct CorrectIconsOnSubscriptions(pub CleanerInner);
impl Step<CleanerInner> for CorrectIconsOnSubscriptions {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None);
        let all_folders: Vec<SubscriptionEntry> = inner
            .subscriptionrepo
            .get_all_entries()
            .into_iter()
            .filter(|fse| !fse.is_folder)
            .collect();
        let mut reset_icon_subs_ids: Vec<i32> = Vec::default();
        let all_icon_ids: Vec<isize> = inner
            .iconrepo
            .get_all_entries()
            .iter()
            .map(|ie| ie.icon_id)
            .collect::<Vec<isize>>();
        if all_icon_ids.len() < 2 {
            inner.send_gp(Some(format!(
                "no icons found, skipping CorrectIconsOnSubscriptions: {}",
                all_icon_ids.len()
            )));
            return StepResult::Continue(Box::new(MarkUnconnectedMessages(inner)));
        }
        for se in all_folders {
            if se.icon_id < ICON_LIST.len() && se.icon_id != gen_icons::IDX_05_RSS_FEEDS_GREY_64_D {
                reset_icon_subs_ids.push(se.subs_id as i32);
                continue;
            }
            if !all_icon_ids.contains(&(se.icon_id as isize)) {
                inner.send_gp(Some(format!(
                    "CorrectIcons: subscr {}  not-in-icon-db: {:?}  ",
                    se.subs_id, se.icon_id
                )));
                reset_icon_subs_ids.push(se.subs_id as i32);
                continue;
            }
            if se.icon_id < gen_icons::IDX_05_RSS_FEEDS_GREY_64_D {
                inner.send_gp(Some(format!(
                    "CorrectIcons: subscr {}  icon id too low: {:?}  ",
                    se.subs_id, se.icon_id
                )));
                reset_icon_subs_ids.push(se.subs_id as i32);
                continue;
            }
        }
        reset_icon_subs_ids.sort();
        if !reset_icon_subs_ids.is_empty() {
            inner.send_gp(Some(format!(
                "CorrectIconsOnSubscriptions : {:?}   #icons={} ",
                reset_icon_subs_ids,
                all_icon_ids.len()
            )));
            inner
                .subscriptionrepo
                .update_icon_id_many(reset_icon_subs_ids, gen_icons::IDX_05_RSS_FEEDS_GREY_64_D);
            inner.need_update_subscriptions = true;
        }
        StepResult::Continue(Box::new(MarkUnconnectedMessages(inner)))
    }

    fn take(self: Box<Self>) -> CleanerInner {
        self.0
    }
}

pub struct MarkUnconnectedMessages(pub CleanerInner);
impl Step<CleanerInner> for MarkUnconnectedMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None); // Some("MarkUnconnectedMessages-0".to_string())
        let parent_ids_active: Vec<i32> = inner
            .subscriptionrepo
            .get_all_nonfolder()
            .iter()
            .filter(|se| !se.isdeleted())
            .map(|se| se.subs_id as i32)
            .collect();
        inner.send_gp(None);
        let noncon_ids = inner
            .messagesrepo
            .get_src_not_contained(&parent_ids_active) // need 0...5s  time
            .iter()
            .filter(|se| !se.is_deleted)
            .map(|fse| fse.message_id as i32)
            .collect::<Vec<i32>>();
        if !noncon_ids.is_empty() {
            let msg: String = if noncon_ids.len() < 100 && parent_ids_active.len() < 100 {
                format!(
                    "Cleanup: not connected messages={:?}   parent-ids={:?}",
                    &noncon_ids, &parent_ids_active
                )
            } else {
                format!(
                    "Cleanup: not connected messages:{}   parent-ids:{}",
                    &noncon_ids.len(),
                    &parent_ids_active.len()
                )
            };
            inner.send_gp(Some(msg));
            inner.need_update_messages = true;
            inner.messagesrepo.update_is_deleted_many(&noncon_ids, true); // needs long time
        }
        StepResult::Continue(Box::new(ReduceTooManyMessages(inner)))
    }
}

pub struct ReduceTooManyMessages(pub CleanerInner);
impl Step<CleanerInner> for ReduceTooManyMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None);
        if inner.max_messages_per_subscription > 1 {
            let subs_ids = inner
                .subscriptionrepo
                .get_all_entries()
                .iter()
                .filter(|fse| !fse.is_folder)
                .map(|fse| fse.subs_id)
                .collect::<Vec<isize>>();
            let markers = [
                subs_ids.len() / 4,
                subs_ids.len() / 2,
                subs_ids.len() * 3 / 4,
            ];
            subs_ids.iter().enumerate().for_each(|(num, su_id)| {
                let (need_u, _n_removed, _n_all, _n_unread) = reduce_too_many_messages(
                    &mut inner.messagesrepo,
                    inner.max_messages_per_subscription as usize,
                    *su_id,
                );
                if markers.contains(&num) {
                    inner.advance_step();
                    inner.send_gp(None); // Some("ReduceTooManyMessages-1".to_string())
                }
                inner.need_update_messages = need_u;
            });
        }
        inner.send_gp(None); // Some("ReduceTooManyMessages-2".to_string())
        StepResult::Continue(Box::new(DeleteDoubleSameMessages(inner)))
    }
}

// returns   need-update, #removed, num-all, num-unread
pub fn reduce_too_many_messages(
    msg_r: &mut MessagesRepo,
    max_messages: usize,
    subs_id: isize,
) -> (bool, usize, usize, usize) {
    let remove_list: Vec<i32>;
    let length_before: usize;
    let mut num_unread: usize;
    let num_all_stay: usize;
    {
        let mut all_messages = msg_r
            .get_by_subscriptions(&[subs_id], true)
            .collect::<Vec<&MessageRow>>();
        length_before = all_messages.len();

        num_unread = all_messages
            .iter()
            .filter(|msg| !msg.is_read && !msg.is_deleted)
            .count();
        if length_before <= max_messages {
            return (false, 0, length_before, num_unread);
        }
        all_messages.sort_by(|a, b| b.entry_src_date.cmp(&a.entry_src_date));
        let (stay, remove) = all_messages.split_at(max_messages);
        remove_list = remove
            .iter()
            .filter(|e| !e.is_favorite())
            .map(|e| e.message_id as i32)
            .collect();
        num_unread = stay
            .iter()
            .filter(|msg| !msg.is_read && !msg.is_deleted)
            .count();
        num_all_stay = stay.iter().filter(|msg| !msg.is_deleted).count();
    }
    if !remove_list.is_empty() {
        msg_r.update_is_deleted_many(&remove_list, true);

        return (true, remove_list.len(), num_all_stay, num_unread);
    }
    (false, 0, length_before, num_unread)
}

pub struct DeleteDoubleSameMessages(pub CleanerInner);
impl Step<CleanerInner> for DeleteDoubleSameMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_to(13);
        inner.send_gp(None);
        let subs_ids_active: Vec<i32> = inner
            .subscriptionrepo
            .get_all_nonfolder()
            .iter()
            .map(|se| se.subs_id as i32)
            .collect();
        let markers = [
            subs_ids_active.len() / 4,
            subs_ids_active.len() / 2,
            subs_ids_active.len() * 3 / 4,
        ];
        subs_ids_active
            .iter()
            .enumerate()
            .for_each(|(num, subs_id)| {
                let mut msglist = inner
                    .messagesrepo
                    .get_by_subscriptions(&[*subs_id as isize], true)
                    .collect::<Vec<&MessageRow>>();
                if !msglist.is_empty() {
                    msglist.sort_by(|a, b| a.fetch_date.cmp(&b.fetch_date));
                    let mut known: HashSet<(i64, String)> = HashSet::new();
                    let mut delete_list: Vec<i32> = Vec::default();
                    msglist.iter().for_each(|msg| {
                        if known.contains(&(msg.entry_src_date, msg.title.clone())) {
                            delete_list.push(msg.message_id as i32);
                        } else {
                            known.insert((msg.entry_src_date, msg.title.clone()));
                        };
                    });
                    if !delete_list.is_empty() {
                        //  let del_indices: Vec<i32> =                            delete_list.iter().map(|m| m.message_id as i32).collect();
                        inner
                            .messagesrepo
                            .update_is_deleted_many(delete_list.as_slice(), true);
                    }
                }
                if markers.contains(&num) {
                    inner.advance_step();
                    inner.send_gp(None); // Some("DeleteDoubleSameMessages-1".to_string())
                }
            });
        StepResult::Continue(Box::new(PurgeMessages(inner)))
    }

    fn take(self: Box<Self>) -> CleanerInner {
        self.0
    }
}

pub struct PurgeMessages(pub CleanerInner);
impl Step<CleanerInner> for PurgeMessages {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_to(16);
        inner.send_gp(None); // Some("PurgeMessages".to_string())
        let allmsg = inner.messagesrepo.get_all_deleted();
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
        inner.advance_step();
        let num_deleted = inner.messagesrepo.delete_by_index(&to_delete);
        if to_delete.len() != num_deleted {
            warn!(
                "PurgeMessages: could not delete completely:  TO_DELETE: {}  DELETED:{}",
                to_delete.len(),
                num_deleted,
            );
        }
        if num_deleted > 0 {
            inner.send_gp(Some(format!("Deleted {} messages", num_deleted)));
        }
        StepResult::Continue(Box::new(CheckErrorLog(inner)))
    }
    fn take(self: Box<Self>) -> CleanerInner {
        self.0
    }
}

pub struct CheckErrorLog(pub CleanerInner);
impl Step<CleanerInner> for CheckErrorLog {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None); // Some("CheckErrorLog".to_string())
        let parent_ids_active: HashSet<isize> = inner
            .subscriptionrepo
            .get_all_nonfolder()
            .iter()
            .filter(|se| !se.isdeleted())
            .map(|se| se.subs_id)
            .collect::<HashSet<isize>>();
        let subs_ids: Vec<isize> = parent_ids_active.into_iter().collect();
        let list: Vec<ErrorEntry> = inner.error_repo.get_all_stored_entries();
        let mut delete_list: Vec<isize> = Vec::default();
        list.iter()
            .filter(|e| !subs_ids.contains(&e.subs_id))
            .for_each(|e| delete_list.push(e.err_id));
        let timestamp_earliest = timestamp_now() - (MAX_ERROR_LINE_AGE_S as i64);
        for subs_id in subs_ids {
            let mut all_per_subs_id: Vec<&ErrorEntry> = list
                .iter()
                .filter(|e| e.subs_id == subs_id)
                .collect::<Vec<&ErrorEntry>>();
            all_per_subs_id.sort_by(|a, b| a.date.cmp(&b.date));
            if all_per_subs_id.len() > MAX_ERROR_LINES_PER_SUBSCRIPTION {
                let (left, right) = all_per_subs_id.split_at(MAX_ERROR_LINES_PER_SUBSCRIPTION);
                left.iter().for_each(|el| {
                    delete_list.push(el.err_id);
                });
                all_per_subs_id = right.to_vec();
            }
            all_per_subs_id
                .iter()
                .filter(|el| el.date < timestamp_earliest)
                .for_each(|el| {
                    delete_list.push(el.err_id);
                });
        }
        if !delete_list.is_empty() {
            inner.send_gp(Some(format!(
                "CheckErrorLog: deleting  {:?} error log entries ",
                delete_list.len()
            )));
        }
        inner.error_repo.delete_by_index(&delete_list);
        StepResult::Continue(Box::new(ShortenDatabases(inner)))
    }
}

pub struct ShortenDatabases(pub CleanerInner);
impl Step<CleanerInner> for ShortenDatabases {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None); // Some("ShortenDatabases-M".to_string())
        inner.messagesrepo.db_vacuum();
        inner.advance_step();
        inner.send_gp(None); //  Some("ShortenDatabases-S".to_string())
        inner.subscriptionrepo.db_vacuum();
        inner.error_repo.db_vacuum();
        StepResult::Continue(Box::new(Notify(inner)))
    }
    fn take(self: Box<Self>) -> CleanerInner {
        self.0
    }
}

pub struct Notify(pub CleanerInner);
impl Step<CleanerInner> for Notify {
    fn step(self: Box<Self>) -> StepResult<CleanerInner> {
        let mut inner = self.0;
        inner.advance_step();
        inner.send_gp(None); // Some("Notify".to_string())
        if inner.need_update_subscriptions {
            let _r = inner.sourcetree_job_sender.send(SJob::UpdateTreePaths);
            let _r = inner
                .sourcetree_job_sender
                .send(SJob::FillSubscriptionsAdapter);
            let _r = inner.sourcetree_job_sender.send(SJob::GuiUpdateTreeAll);
        }
        inner.advance_step();
        inner.send_gp(Some("Stopped".to_string()));
        StepResult::Stop(inner)
    }
    fn take(self: Box<Self>) -> CleanerInner {
        self.0
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
    let entries = (*subs_repo).borrow().get_children(parent_subs_id as isize);
    if !entries.is_empty() {
        subs_active_parents.lock().unwrap().push(parent_subs_id);
    }
    entries.iter().enumerate().for_each(|(folderpos, fse)| {
        let mut path: Vec<u16> = Vec::new();
        path.extend_from_slice(localpath);
        // trace!(            "check_layer  PA{}  FP{}  FSE:  {:?} ",            parent_subs_id,            folderpos,            fse        );
        if fse.folder_position != (folderpos as isize) {
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

/// straightens the folder_position
pub fn resort_parent_list(parent_subs_id: isize, subscriptionrepo: &SubscriptionRepo) {
    let mod_list = subscriptionrepo.get_children(parent_subs_id);
    mod_list.iter().enumerate().for_each(|(n, fse)| {
        if fse.folder_position != n as isize {
            subscriptionrepo.update_folder_position(fse.subs_id, n as isize);
        }
    });
}
