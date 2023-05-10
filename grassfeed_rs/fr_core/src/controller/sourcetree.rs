use crate::config::configmanager::ConfigManager;
use crate::controller::contentdownloader;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::contentlist::CJob;
use crate::controller::contentlist::FeedContents;
use crate::controller::contentlist::IContentList;
use crate::controller::isourcetree::ISourceTreeController;
use crate::controller::subscriptionmove::ISubscriptionMove;
use crate::controller::subscriptionmove::SubscriptionMove;
use crate::controller::timer::ITimer;
use crate::controller::timer::Timer;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::db::subscription_state::FeedSourceState;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::StatusMask;
use crate::db::subscription_state::SubsMapEntry;
use crate::db::subscription_state::SubscriptionState;
use crate::ui_select::gui_context::GuiContext;
use crate::util::db_time_to_display_nonnull;
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

const CHECK_MESSAGE_COUNTS_SET_SIZE: usize = 20;

// #[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SJob {
    /// only store into adapter
    FillSubscriptionsAdapter,

    /// subscription_id  CHECK IF NEEDED
    // FillSourcesTreeSingle(isize),

    /// only update from adapter into TreeView
    GuiUpdateTreeAll,
    /// subscription_id
    GuiUpdateTree(isize),

    /// path to be updated
    GuiUpdateTreePartial(Vec<u16>),

    ScheduleFetchAllFeeds,
    CheckSpinnerActive,
    /// subscription_id
    ScheduleUpdateFeed(isize),
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
    /// subscription_id, removed_some  num_msg_all, num_msg_unread
    NotifyMessagesCountsChecked(isize, bool, isize, isize),
    ScanEmptyUnread,
    EmptyTreeCreateDefaultSubscriptions,
    ///  Drag-String   Feed-Url   Error-Message,   Home-Page-Title
    DragUrlEvaluated(String, String, String, String),
    /// subs_id
    SetCursorToSubsID(isize),
    SetGuiTreeColumn1Width,
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
    pub(super) erro_repo_r: Rc<RefCell<ErrorRepo>>,
    pub(super) config: Rc<RefCell<Config>>,
    pub(super) subscriptionmove_w: Weak<RefCell<SubscriptionMove>>,
    pub(super) current_edit_fse: Option<SubscriptionEntry>,

    //  Subscription,  Non-Folder-Child-IDs
    pub(super) current_selected_subscription: RefCell<Option<(SubscriptionEntry, Vec<i32>)>>,
    pub(super) currently_minimized: bool,
    pub(super) job_queue_sender: Sender<SJob>,
    job_queue_receiver: Receiver<SJob>,
    timer_r: Rc<RefCell<dyn ITimer>>,
    any_spinner_visible: RefCell<bool>,
    pub(super) new_source: RefCell<NewSourceTempData>,
    pub(super) statemap: Rc<RefCell<SubscriptionState>>, // moved over
}

impl SourceTreeController {
    pub const CONF_FETCH_ON_START: &'static str = "FetchFeedsOnStart";
    pub const CONF_FETCH_INTERVAL: &'static str = "FetchFeedsInterval";
    pub const CONF_FETCH_INTERVAL_UNIT: &'static str = "FetchFeedsIntervalUnit";
    pub const CONF_DISPLAY_FEEDCOUNT_ALL: &'static str = "DisplayFeedCountAll";

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
            current_edit_fse: None,
            config: confi,
            new_source: RefCell::new(NewSourceTempData::default()),
            current_selected_subscription: RefCell::new(None),
            gui_context_w: Weak::new(),
            messagesrepo_w: Weak::new(),
            subscriptionmove_w: Weak::new(),
            statemap: Default::default(),
            erro_repo_r: err_rep,
            currently_minimized: false,
        }
    }

    /// is run by  the timer
    pub fn process_jobs(&self) {
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
                    self.process_tree_read_count(subs_id, msg_all, msg_unread);
                }
                SJob::UpdateTreePaths => {
                    if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
                        subs_mov.borrow_mut().update_cached_paths();
                    }
                }
                SJob::FillSubscriptionsAdapter => {
                    self.feedsources_into_store_adapter();
                }
                SJob::GuiUpdateTree(subs_id) => {
                    if let Some(path) = self.get_path_for_src(subs_id) {
                        (*self.gui_updater)
                            .borrow()
                            .update_tree_single(0, path.as_slice());
                    } else {
                        warn!("GuiUpdateTree: No Path for id:{}", subs_id);
                    }
                }
                SJob::GuiUpdateTreeAll => {
                    (*self.gui_updater).borrow().update_tree(TREEVIEW0);
                }
                SJob::GuiUpdateTreePartial(ref path) => {
                    (*self.gui_updater).borrow().update_tree_partial(0, path);
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
                SJob::SetIconId(subs_id, icon_id) => {
                    let ts_now = timestamp_now();
                    (*self.subscriptionrepo_r).borrow().update_icon_id_time(
                        subs_id,
                        icon_id as usize,
                        ts_now,
                    );
                    self.tree_store_update_one(subs_id);
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
                    if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
                        subs_mov.borrow_mut().empty_create_default_subscriptions();
                    }
                }
                SJob::DragUrlEvaluated(ref dragged, ref feed_url, ref err_msg, ref hp_title) => {
                    self.process_drag_url_eval(
                        dragged.to_string(),
                        feed_url.to_string(),
                        err_msg.to_string(),
                        hp_title.to_string(),
                    );
                }
                SJob::SetCursorToSubsID(subs_id) => {
                    if let Some(path) = self.get_path_for_src(subs_id) {
                        self.set_cursor_to_subs_id(path);
                    } else {
                        warn!("SetCursorToSubsID: No Path for id:{}", subs_id);
                    }
                }
                SJob::NotifyMessagesCountsChecked(subs_id, removed_some, a, u) => {
                    self.statemap.borrow_mut().set_num_all_unread(subs_id, a, u);
                    self.statemap.borrow_mut().set_status(
                        &[subs_id],
                        StatusMask::MessageCountsChecked,
                        true,
                    );
                    if removed_some {
                        if let Some(sta) = self.get_state(subs_id) {
                            trace!(
                                "MessagesCountsChecked: {}   {} ST={:?} ",
                                subs_id,
                                removed_some,
                                sta
                            );
                            let subs_e = (*self.subscriptionrepo_r)
                                .borrow()
                                .get_by_index(subs_id)
                                .unwrap();
                            self.tree_update_one(&subs_e, &sta);
                            if subs_e.parent_subs_id > 0 {
                                self.statemap
                                    .borrow_mut()
                                    .clear_num_all_unread(subs_e.parent_subs_id);
                                self.addjob(SJob::ScanEmptyUnread);
                            }
                        }
                    }
                }
                SJob::SetGuiTreeColumn1Width => {
                    let fc_all = (*self.configmanager_r)
                        .borrow()
                        .get_val_bool(Self::CONF_DISPLAY_FEEDCOUNT_ALL);
                    let dd: Vec<AValue> = vec![AValue::ABOOL(fc_all)];
                    (*self.gui_val_store)
                        .write()
                        .unwrap()
                        .set_dialog_data(DIALOG_TREE0COL1, &dd);
                    (*self.gui_updater).borrow().update_dialog(DIALOG_TREE0COL1);
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

    fn process_drag_url_eval(
        &self,
        drag_text: String,
        feed_url: String,
        err_msg: String,
        hp_title: String,
    ) {
        if !err_msg.is_empty() {
            debug!(
                "DragUrlEvaluated: {}  url:{}:   ERR  {} ",
                drag_text, feed_url, err_msg,
            );
        }
        let av_ti = if hp_title.is_empty() {
            AValue::None
        } else {
            AValue::ASTR(hp_title)
        };
        if !feed_url.is_empty() {
            let dd: Vec<AValue> = vec![
                AValue::None,           // 0:display
                av_ti,                  // 1: homepage
                AValue::None,           // 2: icon_str
                AValue::ABOOL(true),    // 3 :spinner
                AValue::ASTR(feed_url), // 4: feed url
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

    fn process_tree_read_count(&self, subs_id: isize, msg_all: isize, msg_unread: isize) {
        let o_subs_state = self
            .statemap
            .borrow_mut()
            .set_num_all_unread(subs_id, msg_all, msg_unread);
        if let Some(su_st) = o_subs_state {
            let subs_e = (*self.subscriptionrepo_r)
                .borrow()
                .get_by_index(subs_id)
                .unwrap();
            // trace!(                "process_tree_read_count {} {}/{}  parent: {} ",                subs_id,                msg_unread,                msg_all,                subs_e.parent_subs_id            );
            if subs_e.parent_subs_id > 0 {
                self.statemap
                    .borrow_mut()
                    .clear_num_all_unread(subs_e.parent_subs_id);
                self.addjob(SJob::ScanEmptyUnread);
            }
            if !self.tree_update_one(&subs_e, &su_st) {
                if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
                    subs_mov.borrow_mut().request_check_paths(true);
                }
            }
        } else {
            warn!("could not store readcount for id {}", subs_id);
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
    pub fn feedsources_into_store_adapter(&self) {
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
            if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
                subs_mov.borrow_mut().request_check_paths(true);
            }
        }
    }

    pub fn get_path_for_src(&self, subs_id: isize) -> Option<Vec<u16>> {
        let o_path = self.statemap.borrow().get_tree_path(subs_id);
        if o_path.is_none() {
            if subs_id == 0 {
                return Some([0].to_vec());
            }
            if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
                subs_mov.borrow_mut().request_check_paths(true);
            }
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

    fn check_icon(&self, subs_id: isize) {
        let o_subs = self.subscriptionrepo_r.borrow().get_by_index(subs_id);
        if o_subs.is_none() {
            return;
        }
        let subs = o_subs.unwrap();
        let now_seconds = timestamp_now();
        let time_outdated = now_seconds - (subs.updated_icon + ICON_RELOAD_TIME_S);
        if time_outdated > 0 || subs.icon_id < ICON_LIST.len() {
            trace!(
                "check_icon:  ID:{}  icon-id:{} icontime:{} time_outdated={}h   now:{}  icontime:{} ",
                subs_id,
                subs.icon_id,
                subs.updated_icon,
                time_outdated / 3600,
                db_time_to_display_nonnull(now_seconds),
                db_time_to_display_nonnull(subs.updated_icon),
            );
            (*self.downloader_r)
                .borrow()
                .load_icon(subs.subs_id, subs.url, subs.icon_id);
        }
    }

    pub fn set_any_spinner_visible(&self, v: bool) {
        self.any_spinner_visible.replace(v);
        (*self.gui_val_store).write().unwrap().set_spinner_active(v);
    }

    pub fn process_newsource_edit(&self) {
        if self.new_source.borrow().state == NewSourceState::UrlChanged {
            if self.new_source.borrow().edit_url.starts_with("http") {
                self.new_source.borrow_mut().state = NewSourceState::Requesting;
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
                    .new_feedsource_request(&self.new_source.borrow().edit_url);
            }
            self.new_source.borrow_mut().state = NewSourceState::Completed;
        }
    }

    pub fn process_newsource_request_done(
        &self,
        feed_url_edit: String,
        display_name: String,
        icon_id: isize,
        feed_homepage: String,
    ) {
        self.new_source.borrow_mut().state = NewSourceState::Completed;
        self.new_source.borrow_mut().edit_url = feed_url_edit;
        self.new_source.borrow_mut().display_name = display_name;
        self.new_source.borrow_mut().icon_id = icon_id;
        self.new_source.borrow_mut().feed_homepage = feed_homepage;
        let mut icon_str = String::default();
        if icon_id > 0 {
            if let Some(ie) = self.iconrepo_r.borrow().get_by_index(icon_id) {
                icon_str = ie.icon;
            }
        };
        self.new_source.borrow_mut().icon_str = icon_str.clone();
        let dd: Vec<AValue> = vec![
            AValue::ASTR(self.new_source.borrow().display_name.clone()),
            AValue::ASTR(self.new_source.borrow().feed_homepage.clone()),
            AValue::ASTR(icon_str), // 2: icon_str
            AValue::ABOOL(false),   // 3: spinner
            AValue::None,           // 4: feed-url
        ];
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_NEW_SUBSCRIPTION, &dd);
        (*self.gui_updater)
            .borrow()
            .update_dialog(DIALOG_NEW_SUBSCRIPTION);
    }

    fn check_feed_update_times(&self) {
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
        let stm_b = self.statemap.borrow();
        let check_feed_ids = entries
            .iter()
            .filter(|fse| !fse.is_folder)
            .map(|fse| {
                let mut fetch_sch = false;
                let mut fetch_pro = false;
                let mut counts_ch = false;
                if let Some(st) = stm_b.get_state(fse.subs_id) {
                    fetch_sch = st.is_fetch_scheduled();
                    fetch_pro = st.is_fetch_in_progress();
                    counts_ch = st.is_messagecounts_checked();
                }
                (fse.subs_id, fetch_sch, fetch_pro, counts_ch)
            })
            .collect::<Vec<(isize, bool, bool, bool)>>();
        let update_ids = check_feed_ids
            .iter()
            .filter(|(_subs_id, fetch_sch, fetch_pro, _)| !fetch_sch && !fetch_pro)
            .map(|(subs_id, _, _, _)| *subs_id)
            .collect::<Vec<isize>>();
        update_ids
            .iter()
            .for_each(|id| self.addjob(SJob::ScheduleUpdateFeed(*id)));
        let mut check_count_ids =
            stm_b.get_ids_by_status(StatusMask::MessageCountsChecked, false, false);
        if !check_count_ids.is_empty() {
            check_count_ids.truncate(CHECK_MESSAGE_COUNTS_SET_SIZE);
            if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                check_count_ids.iter().for_each(|id| {
                    (*feedcontents)
                        .borrow()
                        .addjob(CJob::CheckMessageCounts(*id));
                });
            }
        }
    }

    fn startup_read_config(&mut self) {
        (*self.config).borrow_mut().feeds_fetch_at_start = (*self.configmanager_r)
            .borrow()
            .get_val_bool(Self::CONF_FETCH_ON_START);

        let fc_all = (*self.configmanager_r)
            .borrow()
            .get_val_bool(Self::CONF_DISPLAY_FEEDCOUNT_ALL);
        (*self.config).borrow_mut().display_feedcount_all = fc_all;
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
                    rightcol_text = format!("{num_unread}/{num_all}");
                } else {
                    rightcol_text = format!("{num_all}");
                }
            } else {
                rightcol_text = format!("{num_unread}");
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

    fn set_cursor_to_subs_id(&self, path: Vec<u16>) {
        (*self.gui_updater)
            .borrow()
            .tree_set_cursor(TREEVIEW0, path);
    }
}

impl TimerReceiver for SourceTreeController {
    fn trigger(&self, event: &TimerEvent) {
        if self.currently_minimized {
            if event == &TimerEvent::Timer10s {
                self.process_jobs();
                self.process_fetch_scheduled();
                self.process_newsource_edit();

                if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
                    subs_mov.borrow_mut().check_paths();
                }
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
                    if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
                        subs_mov.borrow_mut().check_paths();
                    }
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

        self.subscriptionmove_w = Rc::downgrade(&(*ac).get_rc::<SubscriptionMove>().unwrap());

        let sm_c_r: Rc<RefCell<dyn ISubscriptionMove>> =
            (*ac).get_rc::<SubscriptionMove>().unwrap();
        self.statemap = (*sm_c_r).borrow().get_state_map();

        let f_so_r = ac.get_rc::<SourceTreeController>().unwrap();
        {
            let mut t = (*self.timer_r).borrow_mut();
            t.register(&TimerEvent::Timer100ms, f_so_r.clone(), false);
            t.register(&TimerEvent::Timer200ms, f_so_r.clone(), false);
            t.register(&TimerEvent::Timer1s, f_so_r.clone(), false);
            t.register(&TimerEvent::Timer10s, f_so_r, false);
        }
        (*self.subscriptionrepo_r)
            .borrow()
            .store_default_db_entries();
        self.startup_read_config();
        self.addjob(SJob::EmptyTreeCreateDefaultSubscriptions);
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSubscriptionsAdapter);
        self.addjob(SJob::GuiUpdateTreeAll);
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
        self.addjob(SJob::ScanEmptyUnread);
        self.addjob(SJob::SetGuiTreeColumn1Width);
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

#[derive(Debug, PartialEq, Eq, Default)]
pub enum NewSourceState {
    #[default]
    None,
    UrlChanged,
    Requesting,
    Completed,
}
