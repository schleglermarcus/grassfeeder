use crate::config::configmanager::ConfigManager;
use crate::controller::contentdownloader::Downloader;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::contentlist::get_font_size_from_config;
use crate::controller::contentlist::CJob;
use crate::controller::contentlist::FeedContents;
use crate::controller::contentlist::IFeedContents;
use crate::db::icon_repo::IconRepo;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_entry::FeedSourceState;
use crate::db::subscription_entry::StatusMask;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_DELETED;
use crate::db::subscription_entry::SRC_REPO_ID_MOVING;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::opml::opmlreader::OpmlReader;
use crate::timer::ITimer;
use crate::timer::Timer;
use crate::ui_select::gui_context::GuiContext;
use crate::util::db_time_to_display_nonnull;
use crate::util::filter_by_iso8859_1;
use crate::util::remove_invalid_chars_from_input;
use crate::util::string_is_http_url;
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

const JOBQUEUE_SIZE: usize = 1000;
pub const TREE_STATUS_COLUMN: usize = 7;

/// seven days
const ICON_RELOAD_TIME_S: i64 = 60 * 60 * 24 * 7;

// #[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SJob {
    FillSourcesTree,
    GuiUpdateTreeAll,
    ScheduleFetchAllFeeds,
    CheckSpinnerActive,
    /// source_repo_id
    ScheduleUpdateFeed(isize),
    GuiUpdateTree(isize),
    /// source_repo_id
    SetFetchInProgress(isize),
    /// source_repo_id, error_happened
    SetFetchFinished(isize, bool),
    /// source_repo_id, new icon_repo_id
    SetIconId(isize, isize),
    SanitizeSources,
    /// source_repo_id, timestamp_feed_update,  timestamp_creation
    StoreFeedCreateUpdate(isize, i64, i64),
    ///  feed-url,  Display-Name, icon-id, Feed-Homepage
    NewFeedSourceEdit(String, String, isize, String),
    /// source_repo_id  - setting window title
    SetSelectedFeedSource(isize),
    /// source_repo_id, content_repo_id
    UpdateLastSelectedMessageId(isize, isize),
    UpdateTreePaths,
    /// subscription_id,  num_msg_all, num_msg_unread
    NotifyTreeReadCount(isize, isize, isize),
    ScanEmptyUnread,
}

// #[automock]
pub trait ISourceTreeController {
    fn on_fs_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool;
    fn mark_schedule_fetch(&self, src_repo_id: isize);
    fn set_tree_expanded(&self, source_repo_id: isize, new_expanded: bool);
    fn addjob(&self, nj: SJob);

    fn add_new_feedsource(&mut self, newsource: String, display: String) -> isize;
    fn add_new_feedsource_at_parent(
        &mut self,
        newsource: String,
        display: String,
        parent_id: isize,
        load_messages: bool,
    ) -> isize;

    /// using internal state for parent id
    fn add_new_folder(&mut self, folder_name: String) -> isize;
    fn add_new_folder_at_parent(&mut self, folder_name: String, parent_id: isize) -> isize;
    fn set_fetch_in_progress(&self, source_repo_id: isize);
    fn set_fetch_finished(&self, source_repo_id: isize, error_happened: bool);

    fn get_job_sender(&self) -> Sender<SJob>;
    fn set_fs_delete_id(&mut self, o_fs_id: Option<usize>);
    fn get_config(&self) -> Config;
    fn set_conf_load_on_start(&mut self, n: bool);
    fn set_conf_fetch_interval(&mut self, n: i32);
    fn set_conf_fetch_interval_unit(&mut self, n: i32);
    fn set_conf_display_feedcount_all(&mut self, a: bool);

    fn feedsource_delete(&mut self);
    fn feedsource_move_to_trash(&mut self);

    fn start_feedsource_edit_dialog(&mut self, source_repo_id: isize);
    fn end_feedsource_edit_dialog(&mut self, values: &[AValue]);
    fn start_new_fol_sub_dialog(&mut self, src_repo_id: isize, dialog_id: u8);
    fn start_delete_dialog(&mut self, src_repo_id: isize);
    fn newsource_dialog_edit(&mut self, edit_feed_url: String);

    fn notify_config_update(&mut self);
    fn set_selected_feedsource(&mut self, src_repo_id: isize);
    fn import_opml(&mut self, filename: String);
    fn mark_as_read(&self, src_repo_id: isize);
    fn get_current_selected_fse(&self) -> Option<SubscriptionEntry>;
}

/// needs  GuiContext SubscriptionRepo ConfigManager IconRepo
pub struct SourceTreeController {
    timer_r: Rc<RefCell<dyn ITimer>>,
    subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    iconrepo_r: Rc<RefCell<IconRepo>>,
    configmanager_r: Rc<RefCell<ConfigManager>>,
    downloader_r: Rc<RefCell<dyn IDownloader>>,
    gui_context_w: Weak<RefCell<GuiContext>>,
    feedcontents_w: Weak<RefCell<FeedContents>>, // YY
    pub messagesrepo_w: Weak<RefCell<MessagesRepo>>,
    job_queue_receiver: Receiver<SJob>,
    job_queue_sender: Sender<SJob>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_val_store: UIAdapterValueStoreType,
    config: Config,
    feedsource_delete_id: Option<usize>,
    current_edit_fse: Option<SubscriptionEntry>,
    current_selected_fse: Option<SubscriptionEntry>,
    current_new_folder_parent_id: Option<isize>,
    new_source: NewSourceTempData,
    tree_fontsize: u32,
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
        let dl_r = (*ac).get_rc::<Downloader>().unwrap();
        Self::new(
            (*ac).get_rc::<Timer>().unwrap(),
            (*ac).get_rc::<SubscriptionRepo>().unwrap(),
            (*ac).get_rc::<ConfigManager>().unwrap(),
            (*ac).get_rc::<IconRepo>().unwrap(),
            u_a,
            v_s_a,
            dl_r,
        )
    }

    pub fn new(
        timer_: Rc<RefCell<dyn ITimer>>,
        subscr_rr: Rc<RefCell<dyn ISubscriptionRepo>>,
        configmanage_: Rc<RefCell<ConfigManager>>,
        iconrepoo: Rc<RefCell<IconRepo>>,
        upd_ad: Rc<RefCell<dyn UIUpdaterAdapter>>,
        v_s_a: UIAdapterValueStoreType,
        downloader_: Rc<RefCell<dyn IDownloader>>,
    ) -> Self {
        let (q_s, q_r) = flume::bounded::<SJob>(JOBQUEUE_SIZE);
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
            config: Config::default(),
            new_source: NewSourceTempData::default(),
            tree_fontsize: 0,
            current_selected_fse: None,
            gui_context_w: Weak::new(),
            messagesrepo_w: Weak::new(),
            need_check_fs_paths: RefCell::new(true),
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
                    if let Some(subs_e) = (*self.subscriptionrepo_r)
                        .borrow()
                        .set_num_all_unread(subs_id, msg_all, msg_unread)
                    {
                        //  trace!(                            "NotifyTreeReadCount {} {}/{} {}",                            subs_id,                            msg_unread,                            msg_all,                            subs_e.display_name                        );
                        self.tree_update_one(&subs_e);
                    } else {
                        warn!("could not store readcount for id {}", subs_id);
                    }
                }
                SJob::UpdateTreePaths => {
                    (*self.subscriptionrepo_r).borrow().update_cached_paths();
                }
                SJob::FillSourcesTree => {
                    self.feedsources_into_store_adapter();
                    (*self.gui_updater).borrow().update_tree(TREEVIEW0);
                }
                SJob::GuiUpdateTree(feed_source_id) => {
                    if let Some(path) = self.get_path_for_src(feed_source_id) {
                        // trace!("GuiUpdateTree {} {:?}", feed_source_id, &path);
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
                    (*self.subscriptionrepo_r).borrow().set_schedule_fetch_all();
                }
                SJob::ScheduleUpdateFeed(fs_id) => {
                    self.mark_schedule_fetch(fs_id);
                }
                SJob::CheckSpinnerActive => {
                    let fetch_in_progress_ids = (*self.subscriptionrepo_r)
                        .borrow()
                        .get_ids_by_status(StatusMask::FetchInProgress, true, false);
                    self.set_any_spinner_visible(!fetch_in_progress_ids.is_empty());
                }
                SJob::SetFetchInProgress(fs_id) => {
                    self.set_fetch_in_progress(fs_id);
                }
                SJob::SetFetchFinished(fs_id, error_happened) => {
                    self.set_fetch_finished(fs_id, error_happened)
                }
                SJob::SetIconId(fs_id, icon_id) => {
                    (*self.subscriptionrepo_r).borrow().update_icon_id(
                        fs_id,
                        icon_id as usize,
                        timestamp_now(),
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
                    (*self.subscriptionrepo_r)
                        .borrow()
                        .update_last_selected(fs_id, fc_id);
                }
                SJob::ScanEmptyUnread => {
                    let o_work_id = (*self.subscriptionrepo_r).borrow().scan_num_all_unread();
                    if let Some(work_src_id) = o_work_id {
                        if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                            (*feedcontents)
                                .borrow()
                                .addjob(CJob::RequestUnreadAllCount(work_src_id));
                        }
                        self.addjob(SJob::ScanEmptyUnread);
                    }
                }
            }
            if self.config.mode_debug {
                let elapsed_m = now.elapsed().as_millis();
                if elapsed_m > 100 {
                    debug!("   SJOB: {:?} took {:?}", &job, elapsed_m);
                }
            }
        }
    }

    ///  Read all sources   from db and put into ModelValueAdapter
    pub fn feedsources_into_store_adapter(&mut self) {
        (*self.gui_val_store).write().unwrap().clear_tree(0);
        let _num_items = self.insert_tree_row(&Vec::<u16>::default(), 0);
        self.addjob(SJob::CheckSpinnerActive);
    }

    /// Creates the tree,  is recursive.
    pub fn insert_tree_row(&self, localpath: &[u16], parent_subs_id: i32) -> i32 {
        let entries = self
            .subscriptionrepo_r
            .borrow()
            .get_by_parent_repo_id(parent_subs_id as isize);
        entries.iter().enumerate().for_each(|(n, fse)| {
            let mut path: Vec<u16> = Vec::new();
            path.extend_from_slice(localpath);
            path.push(n as u16);
            let treevalues = self.tree_row_to_values(fse);
            (*self.gui_val_store)
                .write()
                .unwrap()
                .insert_tree_item(&path, treevalues.as_slice());
            self.insert_tree_row(&path, fse.subs_id as i32); // recurse
        });
        entries.len() as i32
    }

    /// We overlap the  in-mem Folder-expanded with DB-Folder-Expanded
    fn tree_row_to_values(&self, fse: &SubscriptionEntry) -> Vec<AValue> {
        let mut tv: Vec<AValue> = Vec::new(); // linked to ObjectTree
        let mut rightcol_text = String::default(); // later:  folder sum stats
        let mut num_msg_unread = 0;
        if !fse.is_folder {
            if let Some((num_all, num_unread)) = fse.num_msg_all_unread {
                if self.config.display_feedcount_all {
                    rightcol_text = format!("{}/{}", num_unread, num_all);
                } else {
                    rightcol_text = format!("{}", num_unread);
                }
                num_msg_unread = num_unread;
            }
        }
        let mut fs_iconstr: String = String::default();
        if let Some(ie) = self.iconrepo_r.borrow().get_by_index(fse.icon_id as isize) {
            fs_iconstr = ie.icon;
        }
        let mut show_status_icon = false;
        let mut status_icon = gen_icons::ICON_03_ICON_TRANSPARENT_48;
        if fse.is_fetch_scheduled() || fse.is_fetch_scheduled_jobcreated() {
            status_icon = gen_icons::ICON_14_ICON_DOWNLOAD_64;
            show_status_icon = true;
        } else if fse.is_err_on_fetch() {
            status_icon = gen_icons::ICON_32_FLAG_RED_32;
            show_status_icon = true;
        }
        let tp = match &fse.tree_path {
            Some(tp) => format!("{:?}", &tp),
            None => "".to_string(),
        };
        let tooltip = format!(
            "{} ST{} X{}  P{:?} I{} L{}",
            fse.subs_id,
            fse.status,
            match fse.expanded {
                true => 1,
                _ => 0,
            },
            tp,
            fse.icon_id,
            fse.last_selected_msg
        );
        let mut m_status = fse.status as u32;
        if fse.expanded {
            m_status |= TREE0_COL_STATUS_EXPANDED; //StatusMask::FolderExpanded as u32;
        }
        let displayname = if fse.display_name.is_empty() {
            String::from("--")
        } else {
            fse.display_name.clone()
        };
        tv.push(AValue::AIMG(fs_iconstr));
        tv.push(AValue::ASTR(displayname)); // 1:
        tv.push(AValue::ASTR(rightcol_text));
        tv.push(AValue::AIMG(status_icon.to_string()));
        tv.push(AValue::AU32(0)); // 4: is-folder
        tv.push(AValue::AU32(fse.subs_id as u32)); // 5: db-id
        tv.push(AValue::AU32(FontAttributes::to_activation_bits(
            self.tree_fontsize,
            num_msg_unread <= 0,
        ))); //  6: num_content_unread
        tv.push(AValue::AU32(m_status)); //	7 : status

        if self.config.mode_debug {
            tv.push(AValue::ASTR(tooltip)); //  : 8 tooltip
        } else {
            tv.push(AValue::None); //  : 8 tooltip
        }
        let show_spinner = fse.is_fetch_in_progress();
        tv.push(AValue::ABOOL(show_spinner)); //  : 9	spinner visible
        tv.push(AValue::ABOOL(!show_spinner)); //  : 10	StatusIcon Visible
        tv.push(AValue::ABOOL(!(show_status_icon | show_spinner))); //  11: unread-text visible
        tv
    }

    /// update one tree item  from db into treestore. Depends on the last tree path
    pub fn tree_store_update_one(&self, f_source_id: isize) {
        if let Some(fse) = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(f_source_id)
        {
            if !fse.isdeleted() {
                self.tree_update_one(&fse);
            }
        }
    }

    pub fn tree_update_one(&self, subscr: &SubscriptionEntry) {
        if subscr.isdeleted() {
            warn!("tree_update_one:  is_deleted ! {:?}", subscr);
            return;
        }
        match &subscr.tree_path {
            Some(t_path) => {
                let treevalues = self.tree_row_to_values(subscr);
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .replace_tree_item(t_path, &treevalues);
                (*self.gui_updater)
                    .borrow()
                    .update_tree_single(0, t_path.as_slice());
            }
            None => {
                warn!(
                    "tree_update_one: no path for id {} <= {:?}",
                    subscr.subs_id, subscr.tree_path
                );
                self.need_check_fs_paths.replace(true);
            }
        }
    }

    pub fn get_path_for_src(&self, feed_source_id: isize) -> Option<Vec<u16>> {
        let o_path = (*self.subscriptionrepo_r)
            .borrow()
            .get_tree_path(feed_source_id);
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
        let fetch_scheduled_list = (*self.subscriptionrepo_r).borrow().get_ids_by_status(
            StatusMask::FetchScheduled,
            true,
            false,
        );
        if fetch_scheduled_list.is_empty() {
            return;
        }
        let source_id = *fetch_scheduled_list.first().unwrap();
        let mut is_deleted: bool = false;
        let mut jobcreated: bool = false;
        if let Some(subs_e) = (*self.subscriptionrepo_r).borrow().get_by_index(source_id) {
            if subs_e.isdeleted() {
                debug!("process_fetch_scheduled:  deleted  {:?}", subs_e);
                is_deleted = true;
            }
            if subs_e.is_fetch_scheduled_jobcreated() {
                jobcreated = true;
            }
        }
        if !is_deleted && !jobcreated {
            (*self.downloader_r).borrow().add_update_source(source_id);
            (*self.subscriptionrepo_r).borrow().set_status(
                &[source_id],
                StatusMask::FetchScheduledJobCreated,
                true,
            );
            self.set_any_spinner_visible(true);
            self.tree_store_update_one(source_id);
            self.check_icon(source_id);
        }
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_id],
            StatusMask::FetchScheduled,
            false,
        );
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
        let o_from_entry = (*self.subscriptionrepo_r).borrow().get_by_path(from_path);
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
                o_to_entry_parent = (*self.subscriptionrepo_r)
                    .borrow()
                    .get_by_path(to_path_parent);
                // trace!(                    "split1: to_path_parent={:?}   to_parent_folderpos={}  to_path_prev={:?}  ",                    to_path_parent,                    to_parent_folderpos,                    to_path_prev                );
            }
        } else {
            warn!("drag_calc_positions: to_path too short: {:?}", &to_path);
        }
        if o_to_entry_parent.is_none() && !to_path_parent.is_empty() {
            if let Some((_last, elements)) = to_path_parent.split_last() {
                to_path_parent = elements;
            }
            // trace!(                "split2: to_path_parent={:?}   to_parent_folderpos={}    ",                to_path_parent,                to_parent_folderpos            );
            o_to_entry_parent = (*self.subscriptionrepo_r)
                .borrow()
                .get_by_path(to_path_parent);
        }

        let o_to_entry_direct = (*self.subscriptionrepo_r).borrow().get_by_path(to_path);
        let mut o_to_entry_prev: Option<SubscriptionEntry> = None;
        if o_to_entry_direct.is_none() && o_to_entry_parent.is_none() {
            o_to_entry_prev = (*self.subscriptionrepo_r)
                .borrow()
                .get_by_path(to_path_prev.as_slice());
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
            if to_entry_direct.is_folder {
                to_parent_id = to_entry_direct.parent_subs_id;
                to_parent_folderpos = 0;
            } else {
                to_parent_id = to_entry_direct.parent_subs_id;
                to_parent_folderpos = to_entry_direct.folder_position;
            }
            // trace!(                "Direct:  isFolder={}  to_parent_id={}  to_parent_folderpos={:?}",                to_entry_direct.is_folder,                to_parent_id,                to_parent_folderpos            );
            if from_entry.subs_id == to_parent_id {
                return Err(format!(
                    "drag on same element: {}:{:?} => {}:{:?}",
                    from_entry.subs_id, &from_path, to_parent_id, to_path_parent
                ));
            }
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
            // trace!(                "Parent:  to_parent_id={}  to_parent_folderpos={:?}",                to_parent_id,                to_parent_folderpos            );
            return Ok((from_entry, to_parent_id, to_parent_folderpos));
        }
        if let Some(to_entry_prev) = o_to_entry_prev {
            to_parent_id = to_entry_prev.parent_subs_id;
            to_parent_folderpos = to_entry_prev.folder_position + 1;
            // trace!(                "Previous:  to_parent_id={}  to_parent_folderpos={:?}",                to_parent_id,                to_parent_folderpos            );
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
    fn resort_parent_list(&self, parent_subs_id: isize) {
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
            debug!("process_newsource_edit:  {:?}  ", self.new_source);
            if self.new_source.edit_url.starts_with("http") {
                self.new_source.state = NewSourceState::Requesting;
                let dd: Vec<AValue> = vec![
                    AValue::None,        // 0:display
                    AValue::None,        // 1:homepage
                    AValue::None,        // 2: icon_str
                    AValue::ABOOL(true), // 3 :spinner
                ];
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_dialog_data(DIALOG_NEW_FEED_SOURCE, &dd);
                (*self.gui_updater)
                    .borrow()
                    .update_dialog(DIALOG_NEW_FEED_SOURCE);
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
        ];

        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_NEW_FEED_SOURCE, &dd);
        (*self.gui_updater)
            .borrow()
            .update_dialog(DIALOG_NEW_FEED_SOURCE);
    }

    fn check_feed_update_times(&mut self) {
        let interval_s = self.config.get_interval_seconds();
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
            .filter(|fse| !fse.is_fetch_scheduled() && !fse.is_fetch_in_progress())
            .for_each(|fse| {
                self.addjob(SJob::ScheduleUpdateFeed(fse.subs_id));
            });
    }

    fn store_default_db_entries(&self) {
        let mut fse = SubscriptionEntry {
            subs_id: SRC_REPO_ID_DELETED,
            display_name: "_deleted".to_string(),
            is_folder: true,
            parent_subs_id: -1,
            ..Default::default()
        };

        let _r = (*self.subscriptionrepo_r).borrow().store_entry(&fse);
        fse.subs_id = SRC_REPO_ID_MOVING;
        fse.display_name = "_moving".to_string();
        let _r = (*self.subscriptionrepo_r).borrow().store_entry(&fse);
    }

    fn startup_read_config(&mut self) {
        self.config.feeds_fetch_at_start = (*self.configmanager_r)
            .borrow()
            //            .get_section_key_bool(&Self::section_name(), Self::CONF_FETCH_ON_START);
            .get_val_bool(Self::CONF_FETCH_ON_START);
        self.config.display_feedcount_all = (*self.configmanager_r)
            .borrow()
            //            .get_section_key_bool(&Self::section_name(), Self::CONF_DISPLAY_FEECOUNT_ALL);
            .get_val_bool(Self::CONF_DISPLAY_FEECOUNT_ALL);
        self.config.feeds_fetch_interval = (*self.configmanager_r)
            .borrow()
            //		get_section_key_int(            &Self::section_name(),          Self::CONF_FETCH_INTERVAL,            0,        ) as u32;
            .get_val_int(Self::CONF_FETCH_INTERVAL)
            .unwrap_or(0) as u32;

        self.config.feeds_fetch_interval_unit = (*self.configmanager_r)
            .borrow()
            //            .get_section_key_int(&Self::section_name(), Self::CONF_FETCH_INTERVAL_UNIT, 0)
            .get_val_int(Self::CONF_FETCH_INTERVAL_UNIT)
            .unwrap_or(0) as u32;

        if self.config.feeds_fetch_interval == 0 {
            self.config.feeds_fetch_interval = 2;
        }
        if self.config.feeds_fetch_interval_unit == 0 {
            self.config.feeds_fetch_interval_unit = 2; // Hours
        }
        self.config.feeds_fetch_at_start = (*self.configmanager_r)
            .borrow()
            //.get_section_key_bool(&Self::section_name(), Self::CONF_FETCH_ON_START);
            .get_val_bool(Self::CONF_FETCH_ON_START);

        self.config.mode_debug = (*self.configmanager_r)
            .borrow()
            //		get_section_key_bool(            &ConfigManager::section_name(),           ConfigManager::CONF_MODE_DEBUG,        );
            .get_val_bool(ConfigManager::CONF_MODE_DEBUG);

        // debug!(            "sectionname={}    MODE_DEBUG={:?}",            ConfigManager::section_name(),            self.config.mode_debug        );
    }

    fn check_paths(&self) {
        if *self.need_check_fs_paths.borrow() {
            let now = Instant::now();
            (*self.subscriptionrepo_r).borrow().update_cached_paths();
            self.need_check_fs_paths.replace(false);
            let elapsed_ms = now.elapsed().as_millis();
            if elapsed_ms > 20 {
                debug!("check_paths took {} ms", elapsed_ms);
            }
        }
    }
    //	impl SourceTree
}

impl ISourceTreeController for SourceTreeController {
    fn on_fs_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool {
        trace!("START_DRAG {:?} => {:?}      ", &from_path, &to_path);

        let all1 = (*self.subscriptionrepo_r).borrow().get_all_entries();
        let length_before = all1.len();
        let mut success: bool = false;
        match self.drag_calc_positions(&from_path, &to_path) {
            Ok((from_entry, to_parent_id, to_folderpos)) => {
                self.drag_move(from_entry, to_parent_id, to_folderpos);

                let all2 = (*self.subscriptionrepo_r).borrow().get_all_entries();
                if all2.len() != length_before {
                    error!("Drag lost entries: {}->{}", length_before, all2.len());
                    success = false;
                } else {
                    (*self.subscriptionrepo_r).borrow().update_cached_paths();
                    success = true;
                }
            }
            Err(msg) => {
                warn!("DragFail: {:?}=>{:?} --> {} ", from_path, to_path, msg);
                // (*self.subscriptionrepo_r)                    .borrow()                    .debug_dump_tree("dragfail");
            }
        }
        self.addjob(SJob::FillSourcesTree);
        success
    }

    fn mark_schedule_fetch(&self, src_repo_id: isize) {
        let mut is_folder: bool = false;
        if let Some(entry) = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id)
        {
            is_folder = entry.is_folder;
            if entry.isdeleted() {
                return;
            }
            if entry.is_fetch_scheduled() {
                // debug!("fetch already scheduled for {} , skipping ", src_repo_id);
                return;
            }
        }
        if is_folder {
            let child_fse: Vec<SubscriptionEntry> = (*self.subscriptionrepo_r)
                .borrow()
                .get_by_parent_repo_id(src_repo_id);
            let child_repo_ids: Vec<isize> = child_fse
                .iter()
                .filter(|fse| !fse.is_folder)
                .map(|fse| fse.subs_id)
                .collect::<Vec<isize>>();
            trace!("mark_schedule_fetch child_feeds: {:?}   ", child_repo_ids);
            (*self.subscriptionrepo_r).borrow().set_status(
                &child_repo_ids,
                StatusMask::FetchScheduled,
                true,
            );
        } else {
            (*self.subscriptionrepo_r).borrow().set_status(
                &[src_repo_id],
                StatusMask::FetchScheduled,
                true,
            );
            self.tree_store_update_one(src_repo_id);
            self.addjob(SJob::GuiUpdateTree(src_repo_id));
        }
    }

    fn mark_as_read(&self, src_repo_id: isize) {
        let mut is_folder: bool = false;
        if let Some(entry) = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id)
        {
            is_folder = entry.is_folder;
        }
        if is_folder {
            let child_fse: Vec<SubscriptionEntry> = (*self.subscriptionrepo_r)
                .borrow()
                .get_by_parent_repo_id(src_repo_id);
            // debug!("mark_as_read: folder  ");
            child_fse
                .iter()
                .filter(|fse| !fse.is_folder)
                .for_each(|fse| {
                    if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                        (feedcontents)
                            .borrow_mut()
                            .set_read_all(fse.subs_id as isize);
                    }
                });
        } else if let Some(feedcontents) = self.feedcontents_w.upgrade() {
            (feedcontents)
                .borrow_mut()
                .set_read_all(src_repo_id as isize);
            (*self.gui_updater).borrow().update_list(TREEVIEW1);
        }
    }

    fn set_tree_expanded(&self, source_repo_id: isize, new_expanded: bool) {
        let src_vec = vec![source_repo_id];
        (*self.subscriptionrepo_r)
            .borrow_mut()
            .update_expanded(src_vec, new_expanded);
    }

    /// returns  source_repo_id
    fn add_new_folder(&mut self, folder_name: String) -> isize {
        let mut new_parent_id = 0;
        if self.current_new_folder_parent_id.is_some() {
            new_parent_id = self.current_new_folder_parent_id.take().unwrap();
        }
        self.add_new_folder_at_parent(folder_name, new_parent_id)
    }

    fn add_new_folder_at_parent(&mut self, folder_name: String, parent_id: isize) -> isize {
        let mut fse = SubscriptionEntry::from_new_foldername(folder_name, parent_id);
        fse.expanded = true;
        let max_folderpos: Option<isize> = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(parent_id)
            .iter()
            .map(|fse| fse.folder_position)
            .max();
        if let Some(mfp) = max_folderpos {
            fse.folder_position = (mfp + 1) as isize;
        }
        let r = (*self.subscriptionrepo_r).borrow().store_entry(&fse);
        match r {
            Ok(fse) => {
                self.addjob(SJob::UpdateTreePaths);
                self.addjob(SJob::FillSourcesTree);
                fse.subs_id
            }
            Err(e2) => {
                error!("add_new_folder: {:?}", e2);
                -1
            }
        }
    }

    fn addjob(&self, nj: SJob) {
        if self.job_queue_sender.is_full() {
            warn!(
                "FeedSource SJob queue full, size {}.  Skipping  {:?}",
                JOBQUEUE_SIZE, nj
            );
        } else {
            self.job_queue_sender.send(nj).unwrap();
        }
    }

    fn add_new_feedsource(&mut self, newsource: String, display: String) -> isize {
        let p_id = self.current_new_folder_parent_id.unwrap_or(0);
        self.add_new_feedsource_at_parent(newsource, display, p_id, false)
    }

    fn add_new_feedsource_at_parent(
        &mut self,
        newsource: String,
        display: String,
        parent_id: isize,
        load_messages: bool,
    ) -> isize {
        let san_source = remove_invalid_chars_from_input(newsource)
            .trim()
            .to_string();
        let mut san_display = remove_invalid_chars_from_input(display).trim().to_string();
        san_display = filter_by_iso8859_1(&san_display).0;
        let mut highest_src_id = 10;
        if let Some(messagesrepo) = self.messagesrepo_w.upgrade() {
            let h = (*messagesrepo).borrow().get_max_src_index();
            highest_src_id = std::cmp::max(h, highest_src_id);
        }
        let sub_repo_max = (*self.subscriptionrepo_r).borrow().get_highest_src_id();
        highest_src_id = std::cmp::max(sub_repo_max, highest_src_id);
        highest_src_id += 1;

        let mut fse = SubscriptionEntry::from_new_url(san_display, san_source.clone());
        fse.subs_id = highest_src_id;
        fse.parent_subs_id = parent_id;
        let max_folderpos: Option<isize> = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(parent_id)
            .iter()
            .map(|fse| fse.folder_position)
            .max();
        if let Some(mfp) = max_folderpos {
            fse.folder_position = (mfp + 1) as isize;
        }
        // trace!("add_new_feedsource_at_parent  INSERT_FSE={:?}", &fse);
        let mut new_id = -1;
        match (*self.subscriptionrepo_r).borrow().store_entry(&fse) {
            Ok(fse2) => {
                self.addjob(SJob::UpdateTreePaths);
                if load_messages {
                    self.addjob(SJob::FillSourcesTree);
                    self.addjob(SJob::ScheduleUpdateFeed(fse2.subs_id));
                    self.addjob(SJob::CheckSpinnerActive);
                }
                new_id = fse2.subs_id;
            }
            Err(e) => error!(" add_new_feedsource_at_parent >{}<  {:?}", &san_source, e),
        }

        new_id
    }

    fn set_fetch_in_progress(&self, source_repo_id: isize) {
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_repo_id],
            StatusMask::FetchInProgress,
            true,
        );
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_repo_id],
            StatusMask::FetchScheduled,
            false,
        );
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_repo_id],
            StatusMask::FetchScheduledJobCreated,
            false,
        );
        self.set_any_spinner_visible(true);
        self.tree_store_update_one(source_repo_id as isize);
    }

    fn set_fetch_finished(&self, source_repo_id: isize, error_happened: bool) {
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_repo_id],
            StatusMask::FetchInProgress,
            false,
        );
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_repo_id],
            StatusMask::FetchScheduled,
            false,
        );
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_repo_id],
            StatusMask::FetchScheduledJobCreated,
            false,
        );
        (*self.subscriptionrepo_r).borrow().set_status(
            &[source_repo_id],
            StatusMask::ErrFetchReq,
            error_happened,
        );
        self.addjob(SJob::CheckSpinnerActive);
        (*self.subscriptionrepo_r)
            .borrow()
            .clear_num_all_unread(source_repo_id);

        if let Some(fse) = &self.current_selected_fse {
            if fse.subs_id == source_repo_id {
                // trace!("set_fetch_finished {} {}", source_repo_id, fse.display_name);
                if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                    (*feedcontents)
                        .borrow()
                        .update_feed_list_contents(fse.subs_id);
                }
            }
        }
        self.addjob(SJob::ScanEmptyUnread);
        self.tree_store_update_one(source_repo_id as isize);
    }

    fn get_job_sender(&self) -> Sender<SJob> {
        self.job_queue_sender.clone()
    }

    fn set_fs_delete_id(&mut self, o_fs_id: Option<usize>) {
        self.feedsource_delete_id = o_fs_id;
    }

    fn feedsource_move_to_trash(&mut self) {
        if self.feedsource_delete_id.is_none() {
            return;
        }
        let fs_id = self.feedsource_delete_id.unwrap();
        let fse: SubscriptionEntry = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(fs_id as isize)
            .unwrap();
        // debug!(            "feedsource_move_to_trash {:?}   Parent: {}  ",            self.feedsource_delete_id, fse.parent_subs_id        );
        (*self.subscriptionrepo_r)
            .borrow()
            .update_parent_and_folder_position(fse.subs_id, SRC_REPO_ID_DELETED, 0);
        (*self.subscriptionrepo_r)
            .borrow()
            .set_deleted_rec(fse.subs_id);
        self.resort_parent_list(fse.parent_subs_id);
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSourcesTree);
        self.addjob(SJob::GuiUpdateTreeAll);
        self.feedsource_delete_id = None;
    }

    // later: delete only those from trash bin
    fn feedsource_delete(&mut self) {
        if self.feedsource_delete_id.is_none() {
            return;
        }
        let fs_id = self.feedsource_delete_id.unwrap();
        let fse: SubscriptionEntry = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(fs_id as isize)
            .unwrap();
        debug!(
            "feedsource_delete {:?}   Parent: {}  ",
            self.feedsource_delete_id, fse.parent_subs_id
        );
        (*self.subscriptionrepo_r)
            .borrow()
            .delete_by_index(fs_id as isize);
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSourcesTree);
        self.addjob(SJob::GuiUpdateTreeAll);

        self.feedsource_delete_id = None;
    }

    fn start_feedsource_edit_dialog(&mut self, src_repo_id: isize) {
        let mut dialog_id = DIALOG_FS_EDIT;
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id as isize);
        if o_fse.is_none() {
            return;
        }
        let fse = o_fse.unwrap();
        self.current_edit_fse.replace(fse.clone());
        let mut num_all: i32 = -1;
        let mut num_unread: i32 = -1;

        if let Some(feedcontents) = self.feedcontents_w.upgrade() {
            (num_all, num_unread) = (*feedcontents).borrow().get_counts(src_repo_id).unwrap();
        }
        let mut dd: Vec<AValue> = Vec::default();
        let mut fs_iconstr: String = String::default();
        if let Some(ie) = self.iconrepo_r.borrow().get_by_index(fse.icon_id as isize) {
            fs_iconstr = ie.icon;
        }
        dd.push(AValue::ASTR(fse.display_name.clone())); // 0
        if fse.is_folder {
            dialog_id = DIALOG_FOLDER_EDIT;
        } else {
            dd.push(AValue::ASTR(fse.url.clone())); // 1
            dd.push(AValue::AIMG(fs_iconstr)); // 2
            dd.push(AValue::AI32(num_all)); // 3
            dd.push(AValue::AI32(num_unread)); // 4
            dd.push(AValue::ASTR(fse.website_url)); // 5
            dd.push(AValue::ASTR(db_time_to_display_nonnull(fse.updated_int))); // 6
            dd.push(AValue::ASTR(db_time_to_display_nonnull(fse.updated_ext))); // 7
        }
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(dialog_id, &dd);
        (*self.gui_updater).borrow().update_dialog(dialog_id);
        (*self.gui_updater).borrow().show_dialog(dialog_id);
    }

    fn end_feedsource_edit_dialog(&mut self, values: &[AValue]) {
        if self.current_edit_fse.is_none() || values.is_empty() {
            return;
        }
        let fse: SubscriptionEntry = self.current_edit_fse.take().unwrap();
        let newname = values.get(0).unwrap().str().unwrap();
        let newname = (*newname).trim();
        if !newname.is_empty() && fse.display_name != newname {
            (*self.subscriptionrepo_r)
                .borrow()
                .update_displayname(fse.subs_id, newname.to_string());
            self.tree_store_update_one(fse.subs_id);
        }
        if !fse.is_folder {
            let new_url = values.get(1).unwrap().str().unwrap();
            let new_url = (*new_url).trim();
            if !new_url.is_empty() && fse.url != new_url {
                (*self.subscriptionrepo_r)
                    .borrow()
                    .update_url(fse.subs_id, new_url.to_string());
                self.addjob(SJob::ScheduleUpdateFeed(fse.subs_id));
            }
        }
    }

    fn start_new_fol_sub_dialog(&mut self, src_repo_id: isize, dialog_id: u8) {
        match (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id as isize)
        {
            None => {
                debug!("subscription {} not found ", src_repo_id);
                self.current_new_folder_parent_id = None;
            }
            Some(fse) => {
                if fse.is_folder {
                    self.current_new_folder_parent_id = Some(fse.subs_id);
                } else {
                    self.current_new_folder_parent_id = Some(fse.parent_subs_id);
                }
            }
        }
        debug!(
            "show_dialog {}   parent={:?}",
            dialog_id, self.current_new_folder_parent_id
        );
        (*self.gui_updater).borrow().update_dialog(dialog_id);
        (*self.gui_updater).borrow().show_dialog(dialog_id);
    }

    fn start_delete_dialog(&mut self, src_repo_id: isize) {
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id as isize);
        if o_fse.is_none() {
            return;
        }
        let fse = o_fse.unwrap();
        self.set_fs_delete_id(Some(src_repo_id as usize));
        let dd: Vec<AValue> = vec![
            AValue::ABOOL(fse.is_folder),           // 0
            AValue::ASTR(fse.display_name.clone()), // 1
            AValue::ASTR(fse.url),                  // 2
        ];

        debug!("start_delete_dialog  DDD={:?}", &dd);
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_FS_DELETE, &dd);
        (*self.gui_updater).borrow().update_dialog(DIALOG_FS_DELETE);
        (*self.gui_updater).borrow().show_dialog(DIALOG_FS_DELETE);
    }

    fn get_config(&self) -> Config {
        self.config.clone()
    }

    fn set_conf_load_on_start(&mut self, n: bool) {
        self.config.feeds_fetch_at_start = n;
/*
        (*self.configmanager_r).borrow_mut().set_section_key(
            &Self::section_name(),
            SourceTreeController::CONF_FETCH_ON_START,
            n.to_string().as_str(),
        );
*/
(*self.configmanager_r).borrow().set_val(SourceTreeController::CONF_FETCH_ON_START,n.to_string() )		;
    }

    fn set_conf_fetch_interval(&mut self, n: i32) {
        if n < 1 {
            error!("interval too low {}", n);
            return;
        }
        if n > 60 {
            error!("interval too high {}", n);
            return;
        }
        self.config.feeds_fetch_interval = n as u32;
        /*
        (*self.configmanager_r).borrow_mut().set_section_key(
            &Self::section_name(),
            SourceTreeController::CONF_FETCH_INTERVAL,
            n.to_string().as_str(),
        );
        */
        (*self.configmanager_r)
            .borrow()
            .set_val(SourceTreeController::CONF_FETCH_INTERVAL, n.to_string());
    }

    fn set_conf_fetch_interval_unit(&mut self, n: i32) {
        if !(1..=3).contains(&n) {
            error!("fetch_interval_unit wrong {}", n);
            return;
        }
        self.config.feeds_fetch_interval_unit = n as u32;
/*
        (*self.configmanager_r).borrow_mut().set_section_key(
            &Self::section_name(),
            SourceTreeController::CONF_FETCH_INTERVAL_UNIT,
            n.to_string().as_str(),
        );
*/
		(*self.configmanager_r).borrow().set_val(    SourceTreeController::CONF_FETCH_INTERVAL_UNIT, n.to_string() );

    }

    fn set_conf_display_feedcount_all(&mut self, a: bool) {
        self.config.display_feedcount_all = a;
/*
        (*self.configmanager_r).borrow_mut().set_section_key(
            &Self::section_name(),
            SourceTreeController::CONF_DISPLAY_FEECOUNT_ALL,
            a.to_string().as_str(),
        );
*/
		        (*self.configmanager_r).borrow().set_val(SourceTreeController::CONF_DISPLAY_FEECOUNT_ALL, a.to_string());

    }

    fn newsource_dialog_edit(&mut self, edit_feed_url: String) {
        if edit_feed_url != self.new_source.edit_url {
            // trace!("newsource_dialog_edit : {}", edit_feed_url);
            self.new_source.edit_url = edit_feed_url.trim().to_string();
            self.new_source.state = NewSourceState::UrlChanged;
            if string_is_http_url(&self.new_source.edit_url) {
                (*self.downloader_r)
                    .borrow()
                    .new_feedsource_request(&self.new_source.edit_url);
            }
        }
    }

    fn notify_config_update(&mut self) {
        self.tree_fontsize = get_font_size_from_config(self.configmanager_r.clone());
        self.addjob(SJob::FillSourcesTree);
    }

    fn set_selected_feedsource(&mut self, src_repo_id: isize) {
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id as isize);
        if let Some(fse) = o_fse {
            let display_name = fse.display_name.clone();
            if let Some(gui_context) = self.gui_context_w.upgrade() {
                (*gui_context).borrow_mut().set_window_title(display_name);
            }
            self.current_selected_fse = Some(fse);
        }
    }

    fn import_opml(&mut self, filename: String) {
        let new_folder_id = self.add_new_folder_at_parent("import".to_string(), 0);
        let mut opmlreader = OpmlReader::new(self.subscriptionrepo_r.clone());
        match opmlreader.read_from_file(filename) {
            Ok(_) => {
                debug!("import-opml read ok  -> {}", new_folder_id);
                opmlreader.transfer_to_db(new_folder_id);
                self.addjob(SJob::UpdateTreePaths);
            }
            Err(e) => {
                warn!("reading opml {:?}", e);
            }
        }
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSourcesTree);
    }

    fn get_current_selected_fse(&self) -> Option<SubscriptionEntry> {
        self.current_selected_fse.clone()
    }
} // impl ISourceTreeController

impl TimerReceiver for SourceTreeController {
    fn trigger(&mut self, event: &TimerEvent) {
        match event {
            TimerEvent::Timer100ms => {}
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

impl Buildable for SourceTreeController {
    type Output = SourceTreeController;
    fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        SourceTreeController::new_ac(_appcontext)
    }
    fn section_name() -> String {
        String::from("sourcetree")
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
            t.register(&TimerEvent::Timer200ms, f_so_r.clone());
            t.register(&TimerEvent::Timer100ms, f_so_r.clone());
            t.register(&TimerEvent::Timer1s, f_so_r.clone());
            t.register(&TimerEvent::Timer10s, f_so_r);
        }
        self.store_default_db_entries();
        self.startup_read_config();
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSourcesTree);
        if self.config.feeds_fetch_at_start {
            self.addjob(SJob::ScheduleFetchAllFeeds);
        }
        self.addjob(SJob::ScanEmptyUnread);
        self.addjob(SJob::GuiUpdateTreeAll);
        self.addjob(SJob::SanitizeSources);
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
        }
    }
}

#[derive(Default)]
struct NewSourceTempData {
    edit_url: String,
    display_name: String,
    icon_id: isize,
    icon_str: String,
    feed_homepage: String,
    state: NewSourceState,
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
enum NewSourceState {
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

#[cfg(test)]
pub mod feedsources_t {
    // use super::*;
}
