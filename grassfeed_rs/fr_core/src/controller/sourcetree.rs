use crate::config::configmanager::ConfigManager;
use crate::controller::contentdownloader;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::contentlist::CJob;
use crate::controller::contentlist::FeedContents;
use crate::controller::contentlist::IFeedContents;
use crate::controller::isourcetree::ISourceTreeController;
use crate::controller::timer::ITimer;
use crate::controller::timer::Timer;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_MOVING;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::db::subscription_state::FeedSourceState;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::StatusMask;
use crate::db::subscription_state::SubsMapEntry;
use crate::db::subscription_state::SubscriptionState;
use crate::ui_select::gui_context::GuiContext;
use crate::util::timestamp_now;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use flume::Receiver;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::gui_values::FontAttributes;
use resources::gen_icons;
use resources::gen_icons::ICON_LIST;
use resources::id::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;
use std::time::Instant;

pub const JOBQUEUE_SIZE: usize = 1000;
pub const TREE_STATUS_COLUMN: usize = 7;

pub const DEFAULT_CONFIG_FETCH_FEED_INTERVAL: u8 = 2;
pub const DEFAULT_CONFIG_FETCH_FEED_UNIT: u8 = 2; // hours

/// seven days
const ICON_RELOAD_TIME_S: i64 = 60 * 60 * 24 * 7;

// #[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SJob {
    FillSourcesTree,
    // subscription_id
    FillSourcesTreeSingle(isize),
    GuiUpdateTreeAll,
    ScheduleFetchAllFeeds,
    CheckSpinnerActive,
    /// subscription_id
    ScheduleUpdateFeed(isize),
    GuiUpdateTree(isize),
    /// subscription_id
    SetFetchInProgress(isize),
    /// subscription_id, error_happened
    SetFetchFinished(isize, bool),
    /// subscription_id, new icon_repo_id
    SetIconId(isize, isize),
    SanitizeSources,
    /// subscription_id, timestamp_feed_update,  timestamp_creation
    StoreFeedCreateUpdate(isize, i64, i64),
    ///  feed-url,  Display-Name, icon-id, Feed-Homepage
    NewFeedSourceEdit(String, String, isize, String),
    /// subscription_id  - setting window title
    SetSelectedFeedSource(isize),
    /// subscription_id, content_repo_id
    UpdateLastSelectedMessageId(isize, isize),
    UpdateTreePaths,
    /// subscription_id,  num_msg_all, num_msg_unread
    NotifyTreeReadCount(isize, isize, isize),
    ScanEmptyUnread,
    EmptyTreeCreateDefaultSubscriptions,
    ///  Drag-String   Feed-Url   Error-Message,   Home-Page-Title
    DragUrlEvaluated(String, String, String, String),
}

/// needs  GuiContext SubscriptionRepo ConfigManager IconRepo
pub struct SourceTreeController {
    pub messagesrepo_w: Weak<RefCell<MessagesRepo>>,
    pub(super) subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    pub(super) iconrepo_r: Rc<RefCell<IconRepo>>,
    pub(super) configmanager_r: Rc<RefCell<ConfigManager>>,
    pub(super) downloader_r: Rc<RefCell<dyn IDownloader>>,
    pub(super) gui_context_w: Weak<RefCell<GuiContext>>,
    pub(super) feedcontents_w: Weak<RefCell<FeedContents>>, // YY
    pub(super) gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    pub(super) gui_val_store: UIAdapterValueStoreType,
    pub(super) config: Rc<RefCell<Config>>,
    pub(super) feedsource_delete_id: Option<usize>,
    pub(super) current_edit_fse: Option<SubscriptionEntry>,
    pub(super) current_new_folder_parent_id: Option<isize>,
    pub(super) new_source: NewSourceTempData,
    pub(super) statemap: Rc<RefCell<SubscriptionState>>,
    pub(super) erro_repo_r: Rc<RefCell<ErrorRepo>>,
    //  Subscription,  Non-Folder-Child-IDs
    pub(super) current_selected_subscription: Option<(SubscriptionEntry, Vec<i32>)>,
    pub(super) currently_minimized: bool,
    pub(super) job_queue_sender: Sender<SJob>,
    job_queue_receiver: Receiver<SJob>,
    timer_r: Rc<RefCell<dyn ITimer>>,
    any_spinner_visible: RefCell<bool>,
    need_check_fs_paths: RefCell<bool>,
}

impl SourceTreeController {
    pub const CONF_FETCH_ON_START: &'static str = "FetchFeedsOnStart";
    pub const CONF_FETCH_INTERVAL: &'static str = "FetchFeedsInterval";
    pub const CONF_FETCH_INTERVAL_UNIT: &'static str = "FetchFeedsIntervalUnit";
    pub const CONF_DISPLAY_FEECOUNT_ALL: &'static str = "DisplayFeedCountAll";

    pub fn new_ac(ac: &AppContext) -> Self {
        let gc_r = (*ac).get_rc::<GuiContext>().unwrap();
        let u_a = (*gc_r).borrow().get_updater_adapter();
        let v_s_a = (*gc_r).borrow().get_values_adapter();
        let dl_r = (*ac).get_rc::<contentdownloader::Downloader>().unwrap();
        let err_rep = (*ac).get_rc::<ErrorRepo>().unwrap();
        Self::new(
            (*ac).get_rc::<Timer>().unwrap(),
            (*ac).get_rc::<SubscriptionRepo>().unwrap(),
            (*ac).get_rc::<ConfigManager>().unwrap(),
            (*ac).get_rc::<IconRepo>().unwrap(),
            u_a,
            v_s_a,
            dl_r,
            err_rep,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        timer_: Rc<RefCell<dyn ITimer>>,
        subscr_rr: Rc<RefCell<dyn ISubscriptionRepo>>,
        configmanage_: Rc<RefCell<ConfigManager>>,
        iconrepoo: Rc<RefCell<IconRepo>>,
        upd_ad: Rc<RefCell<dyn UIUpdaterAdapter>>,
        v_s_a: UIAdapterValueStoreType,
        downloader_: Rc<RefCell<dyn IDownloader>>,
        err_rep: Rc<RefCell<ErrorRepo>>,
    ) -> Self {
        let (q_s, q_r) = flume::bounded::<SJob>(JOBQUEUE_SIZE);
        let statemap_ = Rc::new(RefCell::new(SubscriptionState::default()));
        let confi = Rc::new(RefCell::new(Config::default()));
        SourceTreeController {
            timer_r: timer_,
            subscriptionrepo_r: subscr_rr,
            iconrepo_r: iconrepoo,
            configmanager_r: configmanage_,
            job_queue_sender: q_s,
            job_queue_receiver: q_r,
            gui_updater: upd_ad,
            gui_val_store: v_s_a,
            any_spinner_visible: RefCell::new(false),
            feedcontents_w: Weak::new(),
            downloader_r: downloader_,
            feedsource_delete_id: None,
            current_edit_fse: None,
            current_new_folder_parent_id: None,
            config: confi,
            new_source: NewSourceTempData::default(),
            current_selected_subscription: None,
            gui_context_w: Weak::new(),
            messagesrepo_w: Weak::new(),
            need_check_fs_paths: RefCell::new(true),
            statemap: statemap_.clone(),
            erro_repo_r: err_rep,
            currently_minimized: false,
        }
    }

    /// is run by  the timer
    pub fn process_jobs(&mut self) {
        let mut job_list: Vec<SJob> = Vec::new();
        while let Ok(job) = self.job_queue_receiver.try_recv() {
            if !job_list.contains(&job) {
                job_list.push(job);
            }
        }
        for job in job_list {
            let now = Instant::now();
            match job {
                SJob::NotifyTreeReadCount(subs_id, msg_all, msg_unread) => {
                    let o_subs_state = self
                        .statemap
                        .borrow_mut()
                        .set_num_all_unread(subs_id, msg_all, msg_unread);
                    if let Some(su_st) = o_subs_state {
                        let subs_e = (*self.subscriptionrepo_r)
                            .borrow()
                            .get_by_index(subs_id)
                            .unwrap();
                        // trace!(                            "NotifyTreeReadCount {} {}/{} clearing parent {} ",                            subs_id,                            msg_unread,                            msg_all,                            subs_e.parent_subs_id                        );
                        if subs_e.parent_subs_id > 0 {
                            self.statemap
                                .borrow_mut()
                                .clear_num_all_unread(subs_e.parent_subs_id);
                            self.addjob(SJob::ScanEmptyUnread);
                        }
                        if !self.tree_update_one(&subs_e, &su_st) {
                            self.need_check_fs_paths.replace(true);
                        }
                    } else {
                        warn!("could not store readcount for id {}", subs_id);
                    }
                }
                SJob::UpdateTreePaths => {
                    self.update_cached_paths();
                }
                SJob::FillSourcesTree => {
                    self.feedsources_into_store_adapter();
                    (*self.gui_updater).borrow().update_tree(TREEVIEW0);
                }
                SJob::FillSourcesTreeSingle(subs_id) => {
                    self.insert_tree_row_single(subs_id);
                }
                SJob::GuiUpdateTree(feed_source_id) => {
                    if let Some(path) = self.get_path_for_src(feed_source_id) {
                        (*self.gui_updater)
                            .borrow()
                            .update_tree_single(0, path.as_slice());
                    } else {
                        warn!(" path not found for FS-ID {}", feed_source_id);
                    }
                }
                SJob::GuiUpdateTreeAll => {
                    (*self.gui_updater).borrow().update_tree(TREEVIEW0);
                }
                SJob::ScheduleFetchAllFeeds => {
                    self.statemap.borrow_mut().set_schedule_fetch_all();
                }
                SJob::ScheduleUpdateFeed(subs_id) => {
                    self.mark_schedule_fetch(subs_id);
                }
                SJob::CheckSpinnerActive => {
                    let fetch_in_progress_ids = self.statemap.borrow().get_ids_by_status(
                        StatusMask::FetchInProgress,
                        true,
                        false,
                    );
                    self.set_any_spinner_visible(!fetch_in_progress_ids.is_empty());
                }
                SJob::SetFetchInProgress(fs_id) => {
                    self.set_fetch_in_progress(fs_id);
                }
                SJob::SetFetchFinished(fs_id, error_happened) => {
                    self.set_fetch_finished(fs_id, error_happened)
                }
                SJob::SetIconId(fs_id, icon_id) => {
                    let ts_now = timestamp_now();
                    (*self.subscriptionrepo_r).borrow().update_icon_id(
                        fs_id,
                        icon_id as usize,
                        ts_now,
                    );
                    self.tree_store_update_one(fs_id);
                }
                SJob::SanitizeSources => {
                    (*self.downloader_r).borrow().cleanup_db();
                }
                SJob::StoreFeedCreateUpdate(src_id, update_now, creation) => {
                    let o_creation = if creation > 0 { Some(creation) } else { None };
                    (*self.subscriptionrepo_r)
                        .borrow()
                        .update_timestamps(src_id, update_now, o_creation);
                }
                SJob::NewFeedSourceEdit(ref feed_url, ref display, icon_id, ref homepage) => {
                    self.process_newsource_request_done(
                        feed_url.clone(),
                        display.clone(),
                        icon_id,
                        homepage.clone(),
                    );
                }
                SJob::SetSelectedFeedSource(src_repo_id) => {
                    self.set_selected_feedsource(src_repo_id)
                }
                SJob::UpdateLastSelectedMessageId(fs_id, fc_id) => {
                    // trace!(                        "processing UpdateLastSelectedMessageId {} {}  ",                        fs_id,                        fc_id                    );
                    (*self.subscriptionrepo_r)
                        .borrow()
                        .update_last_selected(fs_id, fc_id); // later: this  takes a long time sometimes
                    self.set_selected_message_id(fs_id, fc_id);
                }
                SJob::ScanEmptyUnread => {
                    let unread_ids = self.statemap.borrow().scan_num_all_unread();
                    if !unread_ids.is_empty() {
                        self.addjob(SJob::ScanEmptyUnread);
                        for (unread_id, is_folder) in unread_ids {
                            if is_folder {
                                let _r = self.sum_up_num_all_unread(unread_id);
                            } else if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                                (*feedcontents)
                                    .borrow()
                                    .addjob(CJob::RequestUnreadAllCount(unread_id));
                            }
                        }
                    }
                }
                SJob::EmptyTreeCreateDefaultSubscriptions => {
                    self.empty_create_default_subscriptions()
                }
                SJob::DragUrlEvaluated(
                    ref drag_string,
                    ref feed_url,
                    ref err_msg,
                    ref hp_title,
                ) => {
                    if !err_msg.is_empty() {
                        debug!(
                            "DragUrlEvaluated: {}  url:{}:   ERR  {} ",
                            drag_string, feed_url, err_msg,
                        );
                    }
                    let av_ti = if hp_title.is_empty() {
                        AValue::None
                    } else {
                        AValue::ASTR(hp_title.clone())
                    };
                    if !feed_url.is_empty() {
                        let dd: Vec<AValue> = vec![
                            AValue::None,                   // 0:display
                            av_ti,                          // 1: homepage
                            AValue::None,                   // 2: icon_str
                            AValue::ABOOL(true),            // 3 :spinner
                            AValue::ASTR(feed_url.clone()), // 4: feed url
                        ];
                        (*self.gui_val_store)
                            .write()
                            .unwrap()
                            .set_dialog_data(DIALOG_NEW_SUBSCRIPTION, &dd);
                        (*self.gui_updater)
                            .borrow()
                            .update_dialog(DIALOG_NEW_SUBSCRIPTION);
                        (*self.gui_updater)
                            .borrow()
                            .show_dialog(DIALOG_NEW_SUBSCRIPTION);
                    }
                }
            }
            if (*self.config).borrow().mode_debug {
                let elapsed_m = now.elapsed().as_millis();
                if elapsed_m > 100 {
                    debug!("   SJOB: {:?} took {:?}", &job, elapsed_m);
                }
            }
        }
    }

    /// returns: true if we could sum up all children.  If one stat was missing, returns false
    fn sum_up_num_all_unread(&self, folder_subs_id: isize) -> bool {
        let children = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(folder_subs_id);
        let child_subs_ids: Vec<isize> = children.iter().map(|se| se.subs_id).collect();
        let mut sum_all = 0;
        let mut sum_unread = 0;
        let mut one_missing = false;
        child_subs_ids.iter().for_each(|id| {
            let num_all_unread = self.statemap.borrow().get_num_all_unread(*id);
            if let Some((n_a, n_u)) = num_all_unread {
                sum_all += n_a;
                sum_unread += n_u;
            } else {
                one_missing = true;
            }
        });
        if one_missing {
            return false;
        }
        // trace!(            "SUM UP {} {:?}  => {} / {} ",            folder_subs_id,            &child_subs_ids,            sum_all,            sum_unread        );
        self.statemap
            .borrow_mut()
            .set_num_all_unread(folder_subs_id, sum_all, sum_unread);
        self.addjob(SJob::NotifyTreeReadCount(
            folder_subs_id,
            sum_all,
            sum_unread,
        ));
        true
    }

    ///  Read all sources   from db and put into ModelValueAdapter
    pub fn feedsources_into_store_adapter(&mut self) {
        (*self.gui_val_store).write().unwrap().clear_tree(0);
        self.insert_tree_row(&Vec::<u16>::default(), 0);
        self.addjob(SJob::CheckSpinnerActive);
    }

    /// Creates the tree, fills the gui_val_store ,  is recursive.
    pub fn insert_tree_row_single(&self, parent_subscr_id: isize) {
        let entries = self
            .subscriptionrepo_r
            .borrow()
            .get_by_parent_repo_id(parent_subscr_id);
        entries.iter().enumerate().for_each(|(_n, fse)| {
            let o_subs_map = self.statemap.borrow().get_state(fse.subs_id);
            if o_subs_map.is_none() {
                warn!("insert_single : no state map entry for {}", fse.subs_id);
                return;
            }
            let subs_map = o_subs_map.unwrap();
            if subs_map.tree_path.is_none() {
                warn!("insert_single : path for {}", fse.subs_id);
                return;
            }
            let path = subs_map.tree_path.as_ref().unwrap();
            let treevalues = self.tree_row_to_values(fse, &subs_map);
            (*self.gui_val_store)
                .write()
                .unwrap()
                .insert_tree_item(path, treevalues.as_slice());
        });
    }

    /// update one tree item  from db into treestore. Depends on the last tree path
    pub fn tree_store_update_one(&self, f_source_id: isize) {
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(f_source_id);
        if o_fse.is_none() {
            return;
        }
        let fse = o_fse.unwrap();
        if fse.isdeleted() {
            return;
        }
        let o_state = self.statemap.borrow().get_state(fse.subs_id);
        if o_state.is_none() {
            return;
        }
        let su_st = o_state.unwrap();
        if !self.tree_update_one(&fse, &su_st) {
            self.need_check_fs_paths.replace(true);
        }
    }

    pub fn get_path_for_src(&self, feed_source_id: isize) -> Option<Vec<u16>> {
        let o_path = self.statemap.borrow().get_tree_path(feed_source_id);
        if o_path.is_none() {
            debug!("get_path_for_src {} => {:?}", feed_source_id, o_path);
            self.need_check_fs_paths.replace(true);
            return None;
        }
        let path = o_path.unwrap();
        Some(path)
    }

    // if folder was scheduled, now create a downloader job
    fn process_fetch_scheduled(&self) {
        let fetch_scheduled_list =
            self.statemap
                .borrow()
                .get_ids_by_status(StatusMask::FetchScheduled, true, false);
        if fetch_scheduled_list.is_empty() {
            return;
        }
        let source_id = *fetch_scheduled_list.first().unwrap();
        let mut is_deleted: bool = false;
        let mut jobcreated: bool = false;
        let su_st = self
            .statemap
            .borrow()
            .get_state(source_id)
            .unwrap_or_default();
        if let Some(subs_e) = (*self.subscriptionrepo_r).borrow().get_by_index(source_id) {
            if subs_e.isdeleted() {
                debug!("process_fetch_scheduled:  deleted  {:?}", subs_e);
                is_deleted = true;
            }
            if su_st.is_fetch_scheduled_jobcreated() {
                jobcreated = true;
            }
        }
        if !is_deleted && !jobcreated {
            (*self.downloader_r).borrow().add_update_source(source_id);
            self.statemap.borrow_mut().set_status(
                &[source_id],
                StatusMask::FetchScheduledJobCreated,
                true,
            );
            self.set_any_spinner_visible(true);
            self.tree_store_update_one(source_id);
            self.check_icon(source_id);
        }
        self.statemap
            .borrow_mut()
            .set_status(&[source_id], StatusMask::FetchScheduled, false);
    }

    fn check_icon(&self, fs_id: isize) {
        if let Some(fse) = self.subscriptionrepo_r.borrow().get_by_index(fs_id) {
            let now_seconds = timestamp_now();
            let time_outdated = now_seconds - (fse.updated_icon + ICON_RELOAD_TIME_S);
            if time_outdated > 0 || fse.icon_id < ICON_LIST.len() {
                (*self.downloader_r)
                    .borrow()
                    .load_icon(fse.subs_id, fse.url, fse.icon_id);
            }
        }
    }

    pub fn set_any_spinner_visible(&self, v: bool) {
        self.any_spinner_visible.replace(v);
        (*self.gui_val_store).write().unwrap().set_spinner_active(v);
    }

    /// returns:   From-Entry,   To-Parent-ID,  to-folderpos
    ///
    /// When dragging on a folder, we get a sub-sub-Path  from gtk
    ///
    /// Mouse-Drag  to [0]  creates a drag-event  to [0, 0]
    /// Mouse-Drag  to [1]  creates a drag-event  to [1, 0]
    /// Mouse-Drag  under [0]  creates a drag-event  to [1]
    ///
    ///
    pub fn drag_calc_positions(
        &self,
        from_path: &[u16],
        to_path: &[u16],
    ) -> Result<(SubscriptionEntry, isize, isize), String> {
        let o_from_entry = self.get_by_path(from_path);
        if o_from_entry.is_none() {
            self.need_check_fs_paths.replace(true);
            let msg = format!("from_path={:?}  Missing, check statemap", from_path);
            return Err(msg);
        }
        let from_entry = o_from_entry.unwrap();
        let mut to_path_parent: &[u16] = &[];
        let mut to_path_prev: Vec<u16> = Vec::default();
        let mut o_to_entry_parent: Option<SubscriptionEntry> = None;
        if !to_path.is_empty() {
            if let Some((last, elements)) = to_path.split_last() {
                to_path_parent = elements;
                if *last > 0 {
                    to_path_prev = elements.to_vec();
                    to_path_prev.push(*last - 1);
                }
                o_to_entry_parent = self.get_by_path(to_path_parent);
            }
        } else {
            warn!("drag_calc_positions: to_path too short: {:?}", &to_path);
        }
        if o_to_entry_parent.is_none() && !to_path_parent.is_empty() {
            if let Some((_last, elements)) = to_path_parent.split_last() {
                to_path_parent = elements;
            }
            o_to_entry_parent = self.get_by_path(to_path_parent);
        }
        let o_to_entry_direct = self.get_by_path(to_path);
        let mut o_to_entry_prev: Option<SubscriptionEntry> = None;
        if o_to_entry_direct.is_none() && o_to_entry_parent.is_none() {
            o_to_entry_prev = self.get_by_path(to_path_prev.as_slice());
        }
        if o_to_entry_direct.is_none() && o_to_entry_parent.is_none() && o_to_entry_prev.is_none() {
            return Err(format!(
                "to_id not found for {:?} {:?}",
                &to_path, to_path_parent
            ));
        }
        let to_parent_folderpos: isize;
        let to_parent_id;
        if let Some(to_entry_direct) = o_to_entry_direct {
            to_parent_id = to_entry_direct.parent_subs_id;
            if from_entry.subs_id == to_parent_id {
                return Err(format!(
                    "drag on same element: {}:{:?} => {}:{:?}",
                    from_entry.subs_id, &from_path, to_parent_id, to_path_parent
                ));
            }
            to_parent_folderpos = to_entry_direct.folder_position; // dragging insidethe tree down
            return Ok((from_entry, to_parent_id, to_parent_folderpos));
        }
        if let Some(to_entry_parent) = o_to_entry_parent {
            if to_entry_parent.is_folder {
                to_parent_id = to_entry_parent.subs_id;
                to_parent_folderpos = 0;
            } else {
                return Err(format!(
                    "drag on entry: {}:{:?} => {:?}:{:?} no more",
                    from_entry.subs_id, &from_path, to_path_parent, to_entry_parent
                ));
            }
            return Ok((from_entry, to_parent_id, to_parent_folderpos));
        }
        if let Some(to_entry_prev) = o_to_entry_prev {
            to_parent_id = to_entry_prev.parent_subs_id;
            to_parent_folderpos = to_entry_prev.folder_position + 1;
            return Ok((from_entry, to_parent_id, to_parent_folderpos));
        }
        panic!();
    }

    pub fn drag_move(
        &self,
        from_entry: SubscriptionEntry,
        to_parent_id: isize,
        to_folderpos: isize,
    ) {
        let mut to_folderpos_lim = to_folderpos;
        if from_entry.parent_subs_id == to_parent_id && to_folderpos > from_entry.folder_position {
            to_folderpos_lim -= 1;
        }
        // remove the from-entry, re-write the folder-positions
        (*self.subscriptionrepo_r)
            .borrow()
            .update_parent_and_folder_position(
                from_entry.subs_id,
                SRC_REPO_ID_MOVING,
                to_folderpos,
            );
        // rewrite the folder positions
        self.resort_parent_list(from_entry.parent_subs_id);
        // insert element into destination list
        let mut to_list = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(to_parent_id);
        if to_folderpos_lim > to_list.len() as isize {
            to_folderpos_lim = to_list.len() as isize;
        }
        to_list.insert(to_folderpos_lim as usize, from_entry.clone());
        to_list.iter().enumerate().for_each(|(n, fse)| {
            if fse.subs_id == from_entry.subs_id {
                (*self.subscriptionrepo_r)
                    .borrow()
                    .update_parent_and_folder_position(fse.subs_id, to_parent_id, n as isize);
            } else if n != fse.folder_position as usize {
                (*self.subscriptionrepo_r)
                    .borrow()
                    .update_folder_position(fse.subs_id, n as isize);
            }
        });
    }

    /// straightens the folder_pos
    pub fn resort_parent_list(&self, parent_subs_id: isize) {
        let mod_list = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(parent_subs_id);
        mod_list.iter().enumerate().for_each(|(n, fse)| {
            if fse.folder_position != n as isize {
                (*self.subscriptionrepo_r)
                    .borrow()
                    .update_folder_position(fse.subs_id, n as isize);
            }
        });
    }

    pub fn process_newsource_edit(&mut self) {
        if self.new_source.state == NewSourceState::UrlChanged {
            if self.new_source.edit_url.starts_with("http") {
                self.new_source.state = NewSourceState::Requesting;
                let dd: Vec<AValue> = vec![
                    AValue::None,        // 0:display
                    AValue::None,        // 1:homepage
                    AValue::None,        // 2: icon_str
                    AValue::ABOOL(true), // 3 :spinner
                    AValue::None,        // 4: feed url
                ];
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_dialog_data(DIALOG_NEW_SUBSCRIPTION, &dd);
                (*self.gui_updater)
                    .borrow()
                    .update_dialog(DIALOG_NEW_SUBSCRIPTION);
                (*self.downloader_r)
                    .borrow()
                    .new_feedsource_request(&self.new_source.edit_url);
            }
            self.new_source.state = NewSourceState::Completed;
        }
    }

    pub fn process_newsource_request_done(
        &mut self,
        feed_url_edit: String,
        display_name: String,
        icon_id: isize,
        feed_homepage: String,
    ) {
        self.new_source.state = NewSourceState::Completed;
        self.new_source.edit_url = feed_url_edit;
        self.new_source.display_name = display_name;
        self.new_source.icon_id = icon_id;
        self.new_source.feed_homepage = feed_homepage;
        let mut icon_str = String::default();
        if icon_id > 0 {
            if let Some(ie) = self.iconrepo_r.borrow().get_by_index(icon_id as isize) {
                icon_str = ie.icon;
            }
        };
        self.new_source.icon_str = icon_str.clone();
        let dd: Vec<AValue> = vec![
            AValue::ASTR(self.new_source.display_name.clone()),
            AValue::ASTR(self.new_source.feed_homepage.clone()),
            AValue::ASTR(icon_str), // 2: icon_str
            AValue::ABOOL(false),   // 3: spinner
            AValue::None,           // 4: feed-url
        ];
        // trace!(            "process_newsource_request_done  {}",            self.new_source.feed_homepage.clone()        );
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_NEW_SUBSCRIPTION, &dd);
        (*self.gui_updater)
            .borrow()
            .update_dialog(DIALOG_NEW_SUBSCRIPTION);
    }

    fn check_feed_update_times(&mut self) {
        let interval_s = (*self.config).borrow().get_interval_seconds();
        if interval_s <= 0 {
            warn!(
                "skipping feed update, no interval ! {}  config={:?}",
                interval_s, self.config
            );
            return;
        }
        let now = timestamp_now();
        let time_limit_s = now - interval_s;
        let entries: Vec<SubscriptionEntry> = self
            .subscriptionrepo_r
            .borrow()
            .get_by_fetch_time(time_limit_s);
        entries
            .iter()
            .filter(|fse| !fse.is_folder)
            .filter(|fse| {
                let su_st = self
                    .statemap
                    .borrow()
                    .get_state(fse.subs_id)
                    .unwrap_or_default();
                !su_st.is_fetch_scheduled() && !su_st.is_fetch_in_progress()
            })
            .for_each(|fse| {
                self.addjob(SJob::ScheduleUpdateFeed(fse.subs_id));
            });
    }

    fn startup_read_config(&mut self) {
        (*self.config).borrow_mut().feeds_fetch_at_start = (*self.configmanager_r)
            .borrow()
            .get_val_bool(Self::CONF_FETCH_ON_START);
        (*self.config).borrow_mut().display_feedcount_all = (*self.configmanager_r)
            .borrow()
            .get_val_bool(Self::CONF_DISPLAY_FEECOUNT_ALL);
        (*self.config).borrow_mut().feeds_fetch_interval = (*self.configmanager_r)
            .borrow()
            .get_val_int(Self::CONF_FETCH_INTERVAL)
            .unwrap_or(0) as u32;
        (*self.config).borrow_mut().feeds_fetch_interval_unit = (*self.configmanager_r)
            .borrow()
            .get_val_int(Self::CONF_FETCH_INTERVAL_UNIT)
            .unwrap_or(0) as u32;
        if (*self.config).borrow().feeds_fetch_interval == 0 {
            (*self.config).borrow_mut().feeds_fetch_interval =
                DEFAULT_CONFIG_FETCH_FEED_INTERVAL as u32;
        }
        if (*self.config).borrow().feeds_fetch_interval_unit == 0 {
            (*self.config).borrow_mut().feeds_fetch_interval_unit =
                DEFAULT_CONFIG_FETCH_FEED_UNIT as u32;
        } // Hours

        (*self.config).borrow_mut().feeds_fetch_at_start = (*self.configmanager_r)
            .borrow()
            .get_val_bool(Self::CONF_FETCH_ON_START);
        if let Some(s) = (*self.configmanager_r)
            .borrow()
            .get_sys_val(ConfigManager::CONF_MODE_DEBUG)
        {
            if let Ok(b) = s.parse::<bool>() {
                (*self.config).borrow_mut().mode_debug = b;
            }
        }
    }

    fn check_paths(&self) {
        if *self.need_check_fs_paths.borrow() {
            self.update_cached_paths();
            self.need_check_fs_paths.replace(false);
        }
    }

    pub fn get_by_path(&self, path: &[u16]) -> Option<SubscriptionEntry> {
        let o_subs_id = self.statemap.borrow().get_id_by_path(path);
        if let Some(subs_id) = o_subs_id {
            return (*self.subscriptionrepo_r).borrow().get_by_index(subs_id);
        } else if !path.is_empty() {
            debug!(
                "no subscr_id for {:?}   #statemap={}",
                &path,
                self.statemap.borrow().get_length()
            );
        }
        None
    }

    pub fn update_paths_rec(
        &self,
        localpath: &[u16],
        parent_subs_id: i32,
        mut is_deleted: bool,
    ) -> bool {
        if parent_subs_id < 0 {
            is_deleted = true;
        }
        let entries: Vec<SubscriptionEntry> = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(parent_subs_id as isize);
        entries.iter().enumerate().for_each(|(num, entry)| {
            let mut path: Vec<u16> = Vec::new();
            path.extend_from_slice(localpath);
            path.push(num as u16);
            self.update_paths_rec(&path, entry.subs_id as i32, is_deleted);
            let mut smm = self.statemap.borrow_mut();
            smm.set_tree_path(entry.subs_id, path, entry.is_folder);
            smm.set_deleted(entry.subs_id, is_deleted);
        });
        false
    }

    /// scans the messages for highest subscription id, if there is a higher one, use next higher subscription id
    /// returns 0     to use   autoincrement
    pub fn get_next_available_subscription_id(&self) -> isize {
        let subs_repo_highest = (*self.subscriptionrepo_r).borrow().get_highest_src_id();
        let mut next_subs_id = std::cmp::max(subs_repo_highest + 1, 10);
        if let Some(messagesrepo) = self.messagesrepo_w.upgrade() {
            let h = (*messagesrepo).borrow().get_max_src_index();
            if h >= next_subs_id {
                next_subs_id = h + 1;
            } else {
                next_subs_id = 0; // default auto increment
            }
        }
        next_subs_id
    }

    fn empty_create_default_subscriptions(&mut self) {
        let before = (*self.subscriptionrepo_r).borrow().db_existed_before();
        if before {
            return;
        }
        {
            let folder1 = self.add_new_folder_at_parent(t!("SUBSC_DEFAULT_FOLDER1"), 0);
            self.add_new_subscription_at_parent(
                "https://rss.slashdot.org/Slashdot/slashdot".to_string(),
                "Slashdot".to_string(),
                folder1,
                true,
            );
            self.add_new_subscription_at_parent(
                "https://www.reddit.com/r/aww.rss".to_string(),
                "Reddit - Aww".to_string(),
                folder1,
                true,
            );
            self.add_new_subscription_at_parent(
                "https://xkcd.com/atom.xml".to_string(),
                "XKCD".to_string(),
                folder1,
                true,
            );
        }
        {
            let folder2 = self.add_new_folder_at_parent(t!("SUBSC_DEFAULT_FOLDER2"), 0);
            self.add_new_subscription_at_parent(
                "https://github.com/schleglermarcus/grassfeeder/releases.atom".to_string(),
                "Grassfeeder Releases".to_string(),
                folder2,
                true,
            );
            self.add_new_subscription_at_parent(
                "https://blog.linuxmint.com/?feed=rss2".to_string(),
                "Linux Mint".to_string(),
                folder2,
                true,
            );
            self.add_new_subscription_at_parent(
                "http://blog.rust-lang.org/feed.xml".to_string(),
                "Rust Language".to_string(),
                folder2,
                true,
            );
            self.add_new_subscription_at_parent(
                "https://www.heise.de/rss/heise-atom.xml".to_string(),
                "Heise.de".to_string(),
                folder2,
                true,
            );
            self.add_new_subscription_at_parent(
                "https://rss.golem.de/rss.php?feed=ATOM1.0".to_string(),
                "Golem.de".to_string(),
                folder2,
                true,
            );
        }
    }

    /// Creates the tree, fills the gui_val_store ,  is recursive.
    pub fn insert_tree_row(&self, localpath: &[u16], parent_subs_id: i32) -> i32 {
        let entries = self
            .subscriptionrepo_r
            .borrow()
            .get_by_parent_repo_id(parent_subs_id as isize);
        entries.iter().enumerate().for_each(|(n, fse)| {
            let mut path: Vec<u16> = Vec::new();
            path.extend_from_slice(localpath);
            path.push(n as u16);
            let subs_map = match self.statemap.borrow().get_state(fse.subs_id) {
                Some(m) => m,
                None => {
                    warn!("no subs_map for id {} {:?}", fse.subs_id, &path);
                    SubsMapEntry::default()
                }
            };
            let treevalues = self.tree_row_to_values(fse, &subs_map);
            (*self.gui_val_store)
                .write()
                .unwrap()
                .insert_tree_item(&path, treevalues.as_slice());
            self.insert_tree_row(&path, fse.subs_id as i32); // recurse
        });
        entries.len() as i32
    }

    /// We overlap the  in-mem Folder-expanded with DB-Folder-Expanded
    pub fn tree_row_to_values(&self, fse: &SubscriptionEntry, su_st: &SubsMapEntry) -> Vec<AValue> {
        let mut tv: Vec<AValue> = Vec::new(); // linked to ObjectTree
        let mut rightcol_text = String::default(); // later:  folder sum stats
        let mut num_msg_unread = 0;
        if let Some((num_all, num_unread)) = su_st.num_msg_all_unread {
            if (*self.config).borrow().display_feedcount_all {
                if num_unread > 0 {
                    rightcol_text = format!("{}/{}", num_unread, num_all);
                } else {
                    rightcol_text = format!("{}", num_all);
                }
            } else {
                rightcol_text = format!("{}", num_unread);
            }
            num_msg_unread = num_unread;
        }
        let mut fs_iconstr: String = String::default();
        if let Some(ie) = self.iconrepo_r.borrow().get_by_index(fse.icon_id as isize) {
            fs_iconstr = ie.icon;
        }
        let mut show_status_icon = false;
        let mut status_icon = gen_icons::ICON_03_ICON_TRANSPARENT_48;

        if su_st.is_fetch_scheduled() || su_st.is_fetch_scheduled_jobcreated() {
            status_icon = gen_icons::ICON_14_ICON_DOWNLOAD_64;
            show_status_icon = true;
        } else if su_st.is_err_on_fetch() {
            status_icon = gen_icons::ICON_32_FLAG_RED_32;
            show_status_icon = true;
        }
        let tp = match &su_st.tree_path {
            Some(tp) => format!("{:?}", &tp),
            None => "".to_string(),
        };
        let mut m_status = su_st.status as u32;
        if fse.expanded {
            m_status |= TREE0_COL_STATUS_EXPANDED;
        }
        let displayname = if fse.display_name.is_empty() {
            String::from("--")
        } else {
            fse.display_name.clone()
        };
        let mut tooltip_a = AValue::None;
        if su_st.is_err_on_fetch() {
            if let Some(last_e) = (*self.erro_repo_r).borrow().get_last_entry(fse.subs_id) {
                // debug!("err-list {}  => {:?}", fse.subs_id, errorlist);
                let mut e_part = last_e.text;
                e_part.truncate(100);
                tooltip_a = AValue::ASTR(e_part);
            }
        }
        if (*self.config).borrow().mode_debug && tooltip_a == AValue::None {
            tooltip_a = AValue::ASTR(format!(
                "{} ST{} X{}  P{:?} I{} L{}",
                fse.subs_id,
                su_st.status,
                match fse.expanded {
                    true => 1,
                    _ => 0,
                },
                tp,
                fse.icon_id,
                fse.last_selected_msg
            ));
        }
        let show_spinner = su_st.is_fetch_in_progress();
        let mut rightcol_visible = !(show_status_icon | show_spinner);
        if !(*self.config).borrow().display_feedcount_all && num_msg_unread == 0 {
            rightcol_visible = false;
        }

        tv.push(AValue::AIMG(fs_iconstr)); // 0
        tv.push(AValue::ASTR(displayname)); // 1:
        tv.push(AValue::ASTR(rightcol_text));
        tv.push(AValue::AIMG(status_icon.to_string()));
        tv.push(AValue::AU32(0)); // 4: is-folder
        tv.push(AValue::AU32(fse.subs_id as u32)); // 5: db-id
        tv.push(AValue::AU32(FontAttributes::to_activation_bits(
            (*self.config).borrow().tree_fontsize as u32,
            num_msg_unread <= 0,
            fse.is_folder,
            false,
        ))); //  6: num_content_unread
        tv.push(AValue::AU32(m_status)); //	7 : status
        tv.push(tooltip_a); //  : 8 tooltip
        tv.push(AValue::ABOOL(show_spinner)); //  : 9	spinner visible
        tv.push(AValue::ABOOL(!show_spinner)); //  : 10	StatusIcon Visible
        tv.push(AValue::ABOOL(rightcol_visible)); //  11: unread-text visible
        tv
    }

    // return: true on success,   false on fail / path check needed
    pub fn tree_update_one(&self, subscr: &SubscriptionEntry, su_st: &SubsMapEntry) -> bool {
        if subscr.isdeleted() {
            warn!("tree_update_one:  is_deleted ! {:?}", subscr);
            return false;
        }
        match &su_st.tree_path {
            Some(t_path) => {
                let treevalues = self.tree_row_to_values(subscr, su_st);
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .replace_tree_item(t_path, &treevalues);
                (*self.gui_updater)
                    .borrow()
                    .update_tree_single(0, t_path.as_slice());
                true
            }
            None => {
                warn!(
                    "tree_update_one: no path for id {} <= {:?}",
                    subscr.subs_id, su_st.tree_path
                );
                false
            }
        }
    }
}

impl TimerReceiver for SourceTreeController {
    fn trigger(&mut self, event: &TimerEvent) {
        if self.currently_minimized {
            if event == &TimerEvent::Timer10s {
                self.process_jobs();
                self.process_fetch_scheduled();
                self.process_newsource_edit();
                self.check_paths();
                self.check_feed_update_times();
            }
        } else {
            match event {
                TimerEvent::Timer200ms => {
                    self.process_jobs();
                    self.process_fetch_scheduled();
                }
                TimerEvent::Timer1s => {
                    self.process_newsource_edit();
                    self.check_paths();
                }
                TimerEvent::Timer10s => {
                    self.check_feed_update_times();
                }
                _ => (),
            }
        }
    }
}

impl Buildable for SourceTreeController {
    type Output = SourceTreeController;
    fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        SourceTreeController::new_ac(_appcontext)
    }
}

impl StartupWithAppContext for SourceTreeController {
    fn startup(&mut self, ac: &AppContext) {
        self.feedcontents_w = Rc::downgrade(&(*ac).get_rc::<FeedContents>().unwrap());
        self.gui_context_w = Rc::downgrade(&(*ac).get_rc::<GuiContext>().unwrap());
        self.messagesrepo_w = Rc::downgrade(&(*ac).get_rc::<MessagesRepo>().unwrap());
        let f_so_r = ac.get_rc::<SourceTreeController>().unwrap();
        {
            let mut t = (*self.timer_r).borrow_mut();
            t.register(&TimerEvent::Timer100ms, f_so_r.clone());
            t.register(&TimerEvent::Timer200ms, f_so_r.clone());
            t.register(&TimerEvent::Timer1s, f_so_r.clone());
            t.register(&TimerEvent::Timer10s, f_so_r);
        }
        (*self.subscriptionrepo_r)
            .borrow()
            .store_default_db_entries();
        self.startup_read_config();
        self.addjob(SJob::EmptyTreeCreateDefaultSubscriptions);
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSourcesTree);
        if (*self.config).borrow().feeds_fetch_at_start {
            self.addjob(SJob::ScheduleFetchAllFeeds);
        }
        self.addjob(SJob::ScanEmptyUnread);
        self.addjob(SJob::GuiUpdateTreeAll);
        if (self.configmanager_r)
            .borrow()
            .get_val_bool(contentdownloader::CONF_DATABASES_CLEANUP)
        {
            self.addjob(SJob::SanitizeSources);
        }
        self.addjob(SJob::UpdateTreePaths);
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub feeds_fetch_at_start: bool,
    pub feeds_fetch_interval: u32,
    ///  1:Minute    2:Hour    3:Day
    pub feeds_fetch_interval_unit: u32,
    pub display_feedcount_all: bool,
    pub mode_debug: bool,
    pub tree_fontsize: u8,
}

impl Config {
    pub fn get_interval_seconds(&self) -> i64 {
        const M_MINUTE: u32 = 60;
        const M_HOUR: u32 = 60 * 60;
        const M_DAY: u32 = 60 * 60 * 24;
        let multiplicator = match self.feeds_fetch_interval_unit {
            1 => M_MINUTE,
            2 => M_HOUR,
            3 => M_DAY,
            _ => 0,
        };
        (self.feeds_fetch_interval * multiplicator) as i64
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            feeds_fetch_at_start: false,
            feeds_fetch_interval: 1,
            ///  1:Minute    2:Hour    3:Day
            feeds_fetch_interval_unit: 32,
            display_feedcount_all: false,
            mode_debug: false,
            tree_fontsize: 1,
        }
    }
}

#[derive(Default)]
pub(super) struct NewSourceTempData {
    pub(super) edit_url: String,
    pub(super) display_name: String,
    pub(super) icon_id: isize,
    pub(super) icon_str: String,
    pub(super) feed_homepage: String,
    pub(super) state: NewSourceState,
}

impl std::fmt::Debug for NewSourceTempData {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("FeedSourceEntry")
            .field("edit_url", &self.edit_url)
            .field("state", &self.state)
            .field("display", &self.display_name)
            .field("icon_id", &self.icon_id)
            .field("homepage", &self.feed_homepage)
            .field("#icon", &self.icon_str.len())
            .finish()
    }
}

#[derive(Debug, PartialEq)]
pub enum NewSourceState {
    None,
    UrlChanged,
    Requesting,
    Completed,
}

impl Default for NewSourceState {
    fn default() -> Self {
        NewSourceState::None
    }
}
