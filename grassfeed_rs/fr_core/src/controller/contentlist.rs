use crate::config::configmanager::ConfigManager;
use crate::controller::browserpane::BrowserPane;
use crate::controller::browserpane::IBrowserPane;
use crate::controller::contentdownloader::Downloader;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::isourcetree::ISourceTreeController;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::controller::timer::Timer;
use crate::db::message::decompress;
use crate::db::message::MessageRow;
use crate::db::message_state::MessageStateMap;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::downloader::db_clean;
use crate::ui_select::gui_context::GuiContext;
use crate::util::db_time_to_display;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use flume::Receiver;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::gui_values::FontAttributes;
use gui_layer::gui_values::PropDef;
use regex::RegexBuilder;
use resources::gen_icons;
use resources::id::LIST0_COL_MSG_ID;
use resources::id::TREEVIEW1;
use resources::names::FOCUS_POLICY_NAMES;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;
use std::sync::RwLock;

const JOBQUEUE_SIZE: usize = 1000; // at least as many jobs as there might be subscriptions
const LIST_SCROLL_POS: i8 = 80; // to 70% of the upper list is visible, the cursor shall go to the lower 30%

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CJob {
    /// content_id, newtitle
    DbUpdateTitle(isize, String),
    /// content_id, new-post-id
    DbUpdatePostId(isize, String),
    /// content_id, entry-date
    DbUpdateEntryDate(isize, u64),
    ///  list_position, feed_content_id
    UpdateMessageListSome(Vec<(u32, u32)>),
    /// feed_content_id
    SwitchBrowserTabContent(i32),
    ListSetCursorToPolicy,
    /// feed-source-id
    RequestUnreadAllCount(isize),
    UpdateMessageList,
    ListSetCursorToMessage(isize),
    ///  list_position, msg_id, Favorite
    SetFavoriteSome(Vec<(u32, u32)>, bool),
    ///  db-id,   list-position
    LaunchBrowserSuccess(isize, u32),
    /// subs_id
    CheckMessageCounts(isize),
    // millseconds
    Pause(usize),
}

pub trait IContentList {
    /// returns queue size
    fn addjob(&self, nj: CJob) -> usize;

    fn process_jobs(&mut self);

    /// Sets those entries read, updates the  gui-store
    /// If the list entries are already read, don't update them
    /// Map<  repo-id  ,   list-position >
    fn process_list_row_activated(&self, act_dbid_listpos: &HashMap<i32, i32>);

    // check if the old subs_id has changed before
    fn update_message_list_(&self, subscription_id: isize);

    /// Read from db and put into the list view
    fn update_messagelist_only(&self /*, feed_source_id: isize*/);

    ///  Vec < list_position,   feed_content_id >
    fn update_content_list_some(&self, vec_pos_dbid: &[(u32, u32)]);

    /// for clicking on the is-read icon
    fn toggle_feed_item_read(&self, msg_id: isize, list_position: i32);
    ///  clicking on the favorite left
    fn toggle_favorite(&self, msg_id: isize, list_position: i32, new_fav: Option<bool>);
    fn set_favorite_multi(&self, msg_id: &[(i32, i32)], new_fav: bool);

    fn get_job_receiver(&self) -> Receiver<CJob>;
    fn get_job_sender(&self) -> Sender<CJob>;

    //  all content entries, unread content entries
    fn get_counts(&self, source_repo_id: isize) -> Option<(i32, i32)>;
    fn get_config(&self) -> Config;

    fn set_conf_focus_policy(&mut self, n: u8);
    fn set_conf_msg_keep_count(&mut self, n: i32);
    fn notify_config_update(&mut self);

    fn process_list_action(&self, action: String, repoid: Vec<(i32, i32)>);
    fn set_sort_order(&mut self, sort_column: u8, order_up: bool);

    fn set_selected_content_ids(&self, list: Vec<i32>);
    fn get_selected_content_ids(&self) -> Vec<i32>;

    ///  decompressed
    fn get_msg_content_author_categories(
        &self,
        msg_id: isize,
        current_row: Option<&MessageRow>,
    ) -> (String, String, String);
    fn move_list_cursor(&self, c: ListMoveCommand);
    fn set_messages_filter(&mut self, newtext: &str);
    fn launch_browser_single(&self, db_ids: Vec<i32>);
    fn launch_browser_selected(&self);

    /// does not update the message list
    fn set_read_complete_subscription(&mut self, source_repo_id: isize);

    fn memory_conserve(&mut self, act: bool);
    fn keyboard_delete(&self);
}

/// needs GuiContext  ConfigManager  BrowserPane  Downloader
pub struct FeedContents {
    timer_r: Rc<RefCell<Timer>>,
    messagesrepo_r: Rc<RefCell<dyn IMessagesRepo>>,
    feedsources_w: Weak<RefCell<SourceTreeController>>,
    configmanager_r: Rc<RefCell<ConfigManager>>,
    browserpane_r: Rc<RefCell<dyn IBrowserPane>>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    downloader_r: Rc<RefCell<dyn IDownloader>>,
    gui_val_store: UIAdapterValueStoreType,
    job_queue_receiver: Receiver<CJob>,
    job_queue_sender: Sender<CJob>,
    config: Config,
    list_selected_ids: RwLock<Vec<i32>>,
    msg_state: RwLock<MessageStateMap>,
    msg_filter: Option<String>,
    ///  subscription-id, number-of-lines, is_folder
    current_subscription: RefCell<(isize, isize, bool)>,
    window_minimized: bool,
}

impl FeedContents {
    pub const CONF_FOCUS_POLICY: &'static str = "MessageSelectFocusPolicy";
    pub const CONF_MSG_KEEP_COUNT: &'static str = "MessagesKeepCount";
    pub const CONF_MSG_KEEP_COUNT_DEFAULT: i32 = 1000;

    pub fn new(ac: &AppContext) -> Self {
        let (q_s, q_r) = flume::bounded::<CJob>(JOBQUEUE_SIZE);
        let gc_r = (*ac).get_rc::<GuiContext>().unwrap();
        let u_a = (*gc_r).borrow().get_updater_adapter();
        let v_s_a = (*gc_r).borrow().get_values_adapter();
        let cm_r = (*ac).get_rc::<ConfigManager>().unwrap();
        let bp_r = (*ac).get_rc::<BrowserPane>().unwrap();
        let msg_r = (*ac).get_rc::<MessagesRepo>().unwrap();
        let dl_r = (*ac).get_rc::<Downloader>().unwrap();
        FeedContents {
            timer_r: (*ac).get_rc::<Timer>().unwrap(),
            gui_updater: u_a,
            gui_val_store: v_s_a,
            configmanager_r: cm_r,
            browserpane_r: bp_r,
            job_queue_receiver: q_r,
            job_queue_sender: q_s,
            feedsources_w: Weak::new(),
            config: Config::default(),
            list_selected_ids: RwLock::new(Vec::default()),
            messagesrepo_r: msg_r,
            msg_state: Default::default(),
            msg_filter: None,
            current_subscription: RefCell::new((-1, -1, false)),
            window_minimized: false,
            downloader_r: dl_r,
        }
    }

    fn message_to_row(
        fc: &MessageRow,
        fontsize: u32,
        title_d: String,
        debug_mode: bool,
    ) -> Vec<AValue> {
        let mut newrow: Vec<AValue> = Vec::default();
        let nfav = if fc.is_favorite() {
            gen_icons::IDX_44_ICON_GREEN_D
        } else {
            gen_icons::IDX_03_ICON_TRANSPARENT_48
        };
        newrow.push(AValue::IIMG(nfav as i32)); // 0

        newrow.push(AValue::ASTR(title_d)); // 1: message title
        if fc.entry_src_date > 0 {
            let mut displaytime = db_time_to_display(fc.entry_src_date);
            if fc.entry_invalid_pubdate {
                displaytime = format!("! {displaytime}");
            }
            newrow.push(AValue::ASTR(displaytime));
        } else {
            newrow.push(AValue::None);
        }
        let n_icon = match fc.is_read {
            true => gen_icons::IDX_06_CENTER_POINT_GREEN,
            _ => gen_icons::IDX_16_DOCUMENT_PROPERTIES_48,
        };
        newrow.push(AValue::IIMG(n_icon as i32)); //  3
        newrow.push(AValue::AU32(FontAttributes::to_activation_bits(
            fontsize, fc.is_read, false, false,
        ))); // 4
        newrow.push(AValue::AU32(fc.message_id as u32)); // 5
        if debug_mode {
            let isdel = i32::from(fc.is_deleted); // if { 1 } else { 0 }
            newrow.push(AValue::ASTR(format!(
                "id{} src{}  D:{} F:{}",
                fc.message_id,
                fc.subscription_id,
                isdel,
                crate::util::db_time_to_display(fc.fetch_date)
            )));
        } else {
            newrow.push(AValue::None);
        } // 6 :  tooltip
        newrow.push(AValue::ABOOL(fc.is_favorite())); //  7 : is-fav
        newrow
    }

    fn set_read_many(&self, repoid_listpos: &Vec<(i32, i32)>, is_read: bool) {
        if repoid_listpos.is_empty() {
            return;
        }
        let repo_ids: Vec<i32> = repoid_listpos.iter().map(|(r, _p)| *r).collect();
        (*self.messagesrepo_r)
            .borrow_mut()
            .update_is_read_many(&repo_ids, is_read);
        self.msg_state
            .write()
            .unwrap()
            .set_read_many(&repo_ids, is_read);
        let (subs_id, _num_msg, isfolder) = *self.current_subscription.borrow();
        if isfolder {
            if let Some(feedsources) = self.feedsources_w.upgrade() {
                if let Some((_subs_e, children)) =
                    (*feedsources).borrow().get_current_selected_subscription()
                {
                    for c_id in children {
                        self.addjob(CJob::RequestUnreadAllCount(c_id as isize));
                    }
                }
            }
        }
        self.addjob(CJob::RequestUnreadAllCount(subs_id));
        let listpos_repoid: Vec<(u32, u32)> = repoid_listpos
            .iter()
            .map(|(r, p)| (*p as u32, *r as u32))
            .collect();
        self.addjob(CJob::UpdateMessageListSome(listpos_repoid));
    }

    fn set_cursor_to_policy(&self) {
        let fp: usize = self.config.focus_policy as usize;
        assert!(fp < FOCUS_POLICY_NAMES.len());
        match fp {
            1 => {
                self.set_cursor_to_message(-1); // None
            }
            2 => {
                let mut last_selected_msg_id: isize = -1; // Last Selected
                if let Some(feedsources) = self.feedsources_w.upgrade() {
                    if let Some(subs_e) =
                        (*feedsources).borrow().get_current_selected_subscription()
                    {
                        last_selected_msg_id = subs_e.0.last_selected_msg;
                    }
                }
                if last_selected_msg_id > 0 {
                    self.set_cursor_to_message(last_selected_msg_id);
                }
            }
            3 => {
                let (highest_ts_repo_id, _highest_created_timestamp, _earliest_id, _earliest_ts) =
                    self.msg_state
                        .read()
                        .unwrap()
                        .find_latest_earliest_created_timestamp();
                if highest_ts_repo_id > 0 {
                    self.set_cursor_to_message(highest_ts_repo_id);
                }
            }
            4 => {
                let o_before_earliest_unread_id =
                    self.msg_state.read().unwrap().find_before_earliest_unread();
                // trace!(                    "BeforeOldestUnread {}  earliest:{:?} ",                    fp,                    o_before_earliest_unread_id                );
                if let Some(id) = o_before_earliest_unread_id {
                    self.set_cursor_to_message(id);
                }
            }
            _ => (),
        }
    }

    fn set_cursor_to_message(&self, msg_id: isize) {
        (*self.gui_updater).borrow().list_set_cursor(
            TREEVIEW1,
            msg_id,
            LIST0_COL_MSG_ID,
            LIST_SCROLL_POS,
        );
    }

    fn insert_state_from_row(&self, msg: &MessageRow, list_position: Option<isize>) {
        self.msg_state.write().unwrap().insert(
            msg.message_id,
            msg.is_read,
            list_position.unwrap_or(-1),
            msg.entry_src_date,
            msg.title.clone(),
            msg.subscription_id,
        );
    }

    fn filter_messages(&self, list_in: &[MessageRow]) -> Vec<MessageRow> {
        let matchtext: &str = self.msg_filter.as_ref().unwrap().as_str();
        let reg = RegexBuilder::new(&regex::escape(matchtext))
            .case_insensitive(true)
            .build()
            .unwrap();
        let out_list: Vec<MessageRow> = list_in
            .iter()
            .filter(|m| {
                let o_title = self.msg_state.read().unwrap().get_title(m.message_id);
                if o_title.is_none() {
                    return true;
                }
                let title = o_title.unwrap();

                if reg.is_match(&title) {
                    return true;
                }
                false
            })
            .cloned()
            .collect();
        out_list
    }

    /// Read from db and put into the list view,
    /// State Map shall contain only the current subscription's messages, for finding the cursor position for the focus policy
    fn update_feed_list_contents_int(&self) {
        let (subs_id, num_msg, isfolder) = *self.current_subscription.borrow();
        let mut messagelist: Vec<MessageRow> = Vec::default();
        let mut child_ids: Vec<i32> = Vec::default();
        if isfolder {
            if let Some(feedsources) = self.feedsources_w.upgrade() {
                if let Some((_subs_e, child_subs)) =
                    (*feedsources).borrow().get_current_selected_subscription()
                {
                    child_ids = child_subs;
                }
            }
            for subs_id in child_ids {
                (*(self.messagesrepo_r.borrow_mut()))
                    .get_by_src_id(subs_id as isize, false)
                    .into_iter()
                    .for_each(|m| messagelist.push(m));
            }
        } else {
            messagelist = (*(self.messagesrepo_r.borrow_mut())).get_by_src_id(subs_id, false);
        }
        if num_msg != messagelist.len() as isize {
            self.fill_state_map(&messagelist);
        }
        if self.msg_filter.is_some() {
            messagelist = self.filter_messages(&messagelist);
        }
        let mut valstore = (*self.gui_val_store).write().unwrap();
        valstore.clear_list(0);
        messagelist.iter().enumerate().for_each(|(i, fc)| {
            let title_string = self
                .msg_state
                .read()
                .unwrap()
                .get_title(fc.message_id)
                .unwrap_or_default();
            valstore.insert_list_item(
                0,
                i as i32,
                &Self::message_to_row(
                    fc,
                    self.config.list_fontsize as u32,
                    title_string,
                    self.config.mode_debug,
                ),
            );
        });
        (*self.gui_updater).borrow().update_list(TREEVIEW1);
        self.list_selected_ids.write().unwrap().clear();
    }

    fn fill_state_map(&self, r_messagelist: &Vec<MessageRow>) {
        let (subs_id, _num_msg, isfolder) = *self.current_subscription.borrow();
        let messagelist: Vec<MessageRow> = if r_messagelist.is_empty() {
            (*(self.messagesrepo_r.borrow_mut())).get_by_src_id(subs_id, false)
        } else {
            r_messagelist.clone()
        };
        self.current_subscription
            .replace((subs_id, messagelist.len() as isize, isfolder));
        self.msg_state.write().unwrap().clear();
        messagelist.iter().enumerate().for_each(|(i, fc)| {
            self.insert_state_from_row(fc, Some(i as isize));
        });
    }

    fn delete_messages(&self, del_ids: &[i32]) {
        (self.messagesrepo_r)
            .borrow()
            .update_is_deleted_many(del_ids, true);
        let o_neighbour = self
            .msg_state
            .read()
            .unwrap()
            .find_neighbour_message(del_ids);
        let (subs_id, _num_msg, _isfolder) = *self.current_subscription.borrow();
        trace!(
            "kb_delete: {:?} {:?}  next={:?}",
            del_ids,
            subs_id,
            o_neighbour
        );
        self.update_message_list_(subs_id);
        if let Some(feedsources) = self.feedsources_w.upgrade() {
            feedsources.borrow().clear_read_unread(subs_id);
        }
        self.addjob(CJob::RequestUnreadAllCount(subs_id));
        self.addjob(CJob::UpdateMessageList);
        if let Some((msg_id, _gui_list_pos)) = o_neighbour {
            self.addjob(CJob::ListSetCursorToMessage(msg_id));
        }
    }

    fn set_favorite_int(&self, listpos_msgid: &[(u32, u32)], new_fav: bool) {
        let mut mod_listpos_db: Vec<(u32, u32)> = Vec::default();
        listpos_msgid.iter().for_each(|(listpos, msg_id)| {
            let o_msg = (*(self.messagesrepo_r.borrow_mut())).get_by_index(*msg_id as isize);
            if o_msg.is_none() {
                warn!("FAV: msg not found: {}", msg_id);
                return;
            }
            let mut msg = o_msg.unwrap();
            if msg.is_favorite() != new_fav {
                msg.set_favorite(new_fav);
                mod_listpos_db.push((*listpos, *msg_id));
            }
            (*(self.messagesrepo_r.borrow_mut())).update_markers(*msg_id as isize, msg.markers);
        });
        self.update_content_list_some(&mod_listpos_db);
        let vec_listpos = mod_listpos_db
            .iter()
            .map(|(p, _d)| *p)
            .collect::<Vec<u32>>();
        (*self.gui_updater)
            .borrow()
            .update_list_some(TREEVIEW1, &vec_listpos);
    }

    fn check_message_counts(&self, subs_id: isize) {
        let msg_keep_count: isize = (*self.configmanager_r)
            .borrow()
            .get_val_int(FeedContents::CONF_MSG_KEEP_COUNT)
            .unwrap_or(-1);
        let msg_repo = MessagesRepo::new_by_connection(
            (*self.messagesrepo_r).borrow().get_ctx().get_connection(),
        );

        let (rm_some, _n_rm, num_all, num_unread) =
            db_clean::reduce_too_many_messages(&msg_repo, msg_keep_count as usize, subs_id);
        // if rm_some {            trace!(                "checkMessageCounts {} unread:{} removed:{} ",                subs_id,                num_unread,                n_rm            );        }
        if let Some(feedsources) = self.feedsources_w.upgrade() {
            (*feedsources)
                .borrow()
                .addjob(SJob::NotifyMessagesCountsChecked(
                    subs_id,
                    rm_some,
                    num_all as isize,
                    num_unread as isize,
                ));
        }
    }
} // impl FeedContents

impl IContentList for FeedContents {
    /// returns queue size
    fn addjob(&self, nj: CJob) -> usize {
        if self.job_queue_sender.is_full() {
            error!("FeedContents CJob queue full  Skipping  {:?}", nj);
        } else {
            self.job_queue_sender.send(nj).unwrap();
        }
        self.job_queue_sender.len()
    }

    fn process_jobs(&mut self) {
        let mut job_list: Vec<CJob> = Vec::new();
        while let Ok(job) = self.job_queue_receiver.try_recv() {
            job_list.push(job);
        }
        for job in job_list {
            let now = std::time::Instant::now();
            match job {
                CJob::DbUpdateTitle(content_id, ref title) => {
                    (*self.messagesrepo_r)
                        .borrow()
                        .update_title(content_id, title.clone());
                }
                CJob::DbUpdatePostId(content_id, ref post_id) => {
                    (*self.messagesrepo_r)
                        .borrow()
                        .update_post_id(content_id, post_id.clone());
                }
                CJob::DbUpdateEntryDate(content_id, newdate) => {
                    (*self.messagesrepo_r)
                        .borrow()
                        .update_entry_src_date(content_id, newdate as i64);
                }
                CJob::UpdateMessageListSome(ref vec_pos_db) => {
                    self.update_content_list_some(vec_pos_db);
                    let list_pos: Vec<u32> =
                        vec_pos_db.iter().map(|(lp, _db)| *lp).collect::<Vec<u32>>();
                    (*self.gui_updater)
                        .borrow()
                        .update_list_some(TREEVIEW1, &list_pos);
                }
                CJob::SwitchBrowserTabContent(msg_id) => {
                    if self
                        .msg_state
                        .read()
                        .unwrap()
                        .get_contents_author_categories(msg_id as isize)
                        .is_none()
                    {
                        let triplet = self.get_msg_content_author_categories(msg_id as isize, None);
                        self.msg_state
                            .write()
                            .unwrap()
                            .set_contents_author_categories(msg_id as isize, &triplet);
                    }
                    let o_co_au_ca = self
                        .msg_state
                        .read()
                        .unwrap()
                        .get_contents_author_categories(msg_id as isize);
                    let title = self
                        .msg_state
                        .read()
                        .unwrap()
                        .get_title(msg_id as isize)
                        .unwrap_or_default();
                    (*self.browserpane_r)
                        .borrow()
                        .switch_browsertab_content(msg_id, title, o_co_au_ca);
                }
                CJob::ListSetCursorToPolicy => self.set_cursor_to_policy(),
                CJob::RequestUnreadAllCount(feed_source_id) => {
                    let msg_count = (*self.messagesrepo_r).borrow().get_src_sum(feed_source_id);
                    let read_count = (*self.messagesrepo_r).borrow().get_read_sum(feed_source_id);
                    let unread_count = msg_count - read_count;
                    if msg_count >= 0 {
                        if let Some(feedsources) = self.feedsources_w.upgrade() {
                            (*feedsources).borrow().addjob(SJob::NotifyTreeReadCount(
                                feed_source_id,
                                msg_count,
                                unread_count,
                            ));
                        }
                    }
                }
                CJob::UpdateMessageList => {
                    self.update_feed_list_contents_int();
                }
                CJob::ListSetCursorToMessage(msg_id) => {
                    self.set_cursor_to_message(msg_id);
                }
                CJob::SetFavoriteSome(ref vec_listpos_msgid, new_fav) => {
                    self.set_favorite_int(vec_listpos_msgid, new_fav);
                }
                CJob::LaunchBrowserSuccess(msg_id, list_position) => {
                    self.set_read_many(&vec![(msg_id as i32, list_position as i32)], true);
                }
                CJob::CheckMessageCounts(subs_id) => {
                    self.check_message_counts(subs_id);
                }
                CJob::Pause(t_ms) => {
                    std::thread::sleep(std::time::Duration::from_millis(t_ms as u64));
                }
            }
            let elapsed_m = now.elapsed().as_millis();
            if elapsed_m > 200 {
                debug!("CJOB: {:?} took {:?}", &job, elapsed_m);
            }
        }
    }

    /// Sets those entries read, updates the  gui-store
    ///  If the list entries are already read, don't update them
    ///  Map<  repo-id  ,   list-position >
    ///       list-position comes from treemodel.path
    fn process_list_row_activated(&self, act_dbid_listpos: &HashMap<i32, i32>) {
        let mut is_unread_ids: Vec<i32> = Vec::default();
        let mut is_read_ids: Vec<i32> = Vec::default();
        let msg_ids: Vec<i32> = act_dbid_listpos.keys().cloned().collect();
        for msg_id in &msg_ids {
            if self.msg_state.read().unwrap().get_isread(*msg_id as isize) {
                is_read_ids.push(*msg_id);
            } else {
                is_unread_ids.push(*msg_id);
            }
        }
        self.msg_state
            .write()
            .unwrap()
            .set_read_many(&is_unread_ids, true);
        let (last_content_id, _last_list_pos) = act_dbid_listpos.iter().last().unwrap();
        self.addjob(CJob::SwitchBrowserTabContent(*last_content_id));
        let list_pos_dbid = act_dbid_listpos
            .iter()
            .map(|(k, v)| (*v as u32, *k as u32))
            .collect::<Vec<(u32, u32)>>();
        let (subs_id, _num_msg, _isfolder) = *self.current_subscription.borrow();
        let subscr_ids = self
            .msg_state
            .read()
            .unwrap()
            .get_subscription_ids(&msg_ids);
        subscr_ids.iter().for_each(|subs_id| {
            self.addjob(CJob::RequestUnreadAllCount(*subs_id));
        });
        if !is_unread_ids.is_empty() {
            (*self.messagesrepo_r)
                .borrow_mut()
                .update_is_read_many(&is_unread_ids, true);
            self.addjob(CJob::RequestUnreadAllCount(subs_id));
        }
        self.addjob(CJob::UpdateMessageListSome(list_pos_dbid));
        if let Some(feedsources) = self.feedsources_w.upgrade() {
            (*feedsources)
                .borrow()
                .addjob(SJob::UpdateLastSelectedMessageId(
                    subs_id,
                    *last_content_id as isize,
                ));
            if !subscr_ids.is_empty() {
                (*feedsources).borrow().addjob(SJob::ScanEmptyUnread);
            }
        }
    }

    fn set_read_complete_subscription(&mut self, src_repo_id: isize) {
        (*self.messagesrepo_r)
            .borrow_mut()
            .update_is_read_all(src_repo_id, true);
        let (current_subs_id, _numlines, _isfolder) = *self.current_subscription.borrow();
        if current_subs_id == src_repo_id {
            self.update_message_list_(src_repo_id);
            self.addjob(CJob::RequestUnreadAllCount(src_repo_id));
            (*self.gui_updater).borrow().update_list(TREEVIEW1);
        } else {
            warn!(
                "set_read_complete_subscription: {} != {}",
                current_subs_id, src_repo_id
            );
        }
    }

    fn update_message_list_(&self, subscription_id: isize) {
        let (old_subs_id, _num_msg, mut isfolder) = *self.current_subscription.borrow();
        if subscription_id != old_subs_id {
            if let Some(feedsources) = self.feedsources_w.upgrade() {
                if let Some(subs_e) = (*feedsources).borrow().get_current_selected_subscription() {
                    isfolder = subs_e.0.is_folder;
                }
            }
            self.current_subscription
                .replace((subscription_id, -1, isfolder));
            self.update_messagelist_only();
        }
    }

    fn update_messagelist_only(&self) {
        self.fill_state_map(&Vec::default());
        self.addjob(CJob::UpdateMessageList);
        self.addjob(CJob::ListSetCursorToPolicy);
    }

    fn update_content_list_some(&self, vec_pos_dbid: &[(u32, u32)]) {
        for (list_position, feed_content_id) in vec_pos_dbid {
            let o_msg: Option<MessageRow> =
                (*(self.messagesrepo_r.borrow_mut())).get_by_index(*feed_content_id as isize);
            if o_msg.is_none() {
                warn!("update_single: no messsage for {}", feed_content_id);
                continue;
            }
            let msg: MessageRow = o_msg.unwrap();
            if msg.is_deleted {
                debug!("update_content_list_some  isdeleted: {}", &msg);
                continue;
            }
            if let Some(titl) = self.msg_state.read().unwrap().get_title(msg.message_id) {
                let av_list = Self::message_to_row(
                    &msg,
                    self.config.list_fontsize as u32,
                    titl,
                    self.config.mode_debug,
                );
                (*self.gui_val_store).write().unwrap().insert_list_item(
                    0,
                    *list_position as i32,
                    &av_list,
                );
            }
        }
    }

    /// for clicking on the is-read icon
    fn toggle_feed_item_read(&self, msg_id: isize, list_position: i32) {
        let is_read = self.msg_state.read().unwrap().get_isread(msg_id);
        self.msg_state
            .write()
            .unwrap()
            .set_read_many(&[msg_id as i32], !is_read);
        (*(self.messagesrepo_r.borrow_mut())).update_is_read_many(&[msg_id as i32], !is_read);
        let vec_pos_db: Vec<(u32, u32)> = vec![(list_position as u32, msg_id as u32)];
        self.update_content_list_some(&vec_pos_db);
        (*self.gui_updater)
            .borrow()
            .update_list_some(TREEVIEW1, &[list_position as u32]);
        let (subs_id, _num_msg, _isfolder) = *self.current_subscription.borrow();
        self.addjob(CJob::RequestUnreadAllCount(subs_id));
    }

    /// for clicking on Favorite Icon
    fn toggle_favorite(&self, msg_id: isize, list_position: i32, new_fav: Option<bool>) {
        let o_msg = (*(self.messagesrepo_r.borrow_mut())).get_by_index(msg_id);
        if o_msg.is_none() {
            warn!("FAV: msg not found: {}", msg_id);
            return;
        }
        let mut msg = o_msg.unwrap();
        // trace!(            "TOGGLE_FAV  {}   col{}  isFav:{} ",            msg_id,            list_position,            msg.is_favorite()        );
        if let Some(f) = new_fav {
            msg.set_favorite(f);
        } else {
            msg.set_favorite(!msg.is_favorite());
        }
        (*(self.messagesrepo_r.borrow_mut())).update_markers(msg_id, msg.markers);
        let vec_pos_db: Vec<(u32, u32)> = vec![(list_position as u32, msg_id as u32)];
        self.update_content_list_some(&vec_pos_db);
        (*self.gui_updater)
            .borrow()
            .update_list_some(TREEVIEW1, &[list_position as u32]);
    }

    /// [  ( msg-id , list-pos ) ]
    fn set_favorite_multi(&self, msg_id_listpos: &[(i32, i32)], new_fav: bool) {
        let chunk_size = 7;
        if msg_id_listpos.len() <= chunk_size {
            let mut mod_listpos_db: Vec<(u32, u32)> = Vec::default();
            msg_id_listpos.iter().for_each(|(msg_id, listpos)| {
                mod_listpos_db.push((*listpos as u32, *msg_id as u32));
            });
            self.addjob(CJob::SetFavoriteSome(mod_listpos_db, new_fav));
            return;
        }
        let num_chunks = (msg_id_listpos.len() + chunk_size - 1) / chunk_size;
        let num_lines = (msg_id_listpos.len() + 1) / num_chunks;
        for c in 0..num_chunks {
            let mut mod_listpos_db: Vec<(u32, u32)> = Vec::default();
            msg_id_listpos
                .iter()
                .skip(c * num_lines)
                .take(num_lines)
                .for_each(|(msg_id, listpos)| {
                    mod_listpos_db.push((*listpos as u32, *msg_id as u32));
                });
            self.addjob(CJob::SetFavoriteSome(mod_listpos_db, new_fav));
        }
    }

    fn get_job_receiver(&self) -> Receiver<CJob> {
        self.job_queue_receiver.clone()
    }

    fn get_job_sender(&self) -> Sender<CJob> {
        self.job_queue_sender.clone()
    }

    //  all content entries, unread content entries
    fn get_counts(&self, source_repo_id: isize) -> Option<(i32, i32)> {
        let all = (*self.messagesrepo_r)
            .borrow()
            .get_by_src_id(source_repo_id, false);
        let num_is_read = all.iter().filter(|fce| fce.is_read).count() as i32;
        Some((all.len() as i32, (all.len() as i32 - num_is_read)))
    }

    fn get_config(&self) -> Config {
        self.config.clone()
    }

    fn set_conf_focus_policy(&mut self, n: u8) {
        if n < 1 || n > FOCUS_POLICY_NAMES.len() as u8 {
            error!("_focus_policy wrong {}", n);
            return;
        }
        self.config.focus_policy = n;
        (*self.configmanager_r)
            .borrow()
            .set_val(FeedContents::CONF_FOCUS_POLICY, n.to_string());
    }

    fn set_conf_msg_keep_count(&mut self, n: i32) {
        if n < 1 {
            error!("msg_keep_count wrong {}", n);
            return;
        }
        self.config.message_keep_count = n;
        (*self.configmanager_r)
            .borrow_mut()
            .set_val(FeedContents::CONF_MSG_KEEP_COUNT, n.to_string());
    }

    fn notify_config_update(&mut self) {
        self.config.list_fontsize = get_font_size_from_config(self.configmanager_r.clone()) as u8;
    }

    fn set_selected_content_ids(&self, list: Vec<i32>) {
        let mut l = self.list_selected_ids.write().unwrap();
        l.clear();
        let mut mutable = list;
        l.append(&mut mutable);
    }

    fn get_selected_content_ids(&self) -> Vec<i32> {
        self.list_selected_ids.read().unwrap().clone()
    }

    fn process_list_action(&self, action: String, msgid_listpos: Vec<(i32, i32)>) {
        match action.as_str() {
            "mark-as-read" => {
                self.set_read_many(&msgid_listpos, true);
            }
            "mark-as-unread" => {
                self.set_read_many(&msgid_listpos, false);
            }
            "open-in-browser" => {
                let db_ids: Vec<i32> = msgid_listpos.iter().map(|(db, _lp)| *db).collect();
                self.launch_browser_single(db_ids);
            }
            "messages-delete" => {
                let db_ids: Vec<i32> = msgid_listpos.iter().map(|(db, _lp)| *db).collect();
                self.delete_messages(&db_ids);
            }
            "message-copy-link" => {
                if let Some((subs_id, _lispos)) = msgid_listpos.first() {
                    if let Some(e_msg) = (*self.messagesrepo_r)
                        .borrow()
                        .get_by_index(*subs_id as isize)
                    {
                        (*self.gui_updater).borrow().clipboard_set_text(e_msg.link);
                    }
                } else {
                    debug!("copy-link : no subs-id !!");
                }
            }
            "mark-as-favorite" => {
                self.set_favorite_multi(&msgid_listpos, true);
            }
            "unmark-favorite" => {
                self.set_favorite_multi(&msgid_listpos, false);
            }

            _ => {
                warn!("contentlist_action unknown {}", &action);
            }
        }
    }

    fn set_sort_order(&mut self, sort_column: u8, ascending: bool) {
        self.config.list_sort_column = sort_column;
        self.config.list_sort_order_up = ascending;
        (self.configmanager_r).borrow().set_val(
            &PropDef::GuiList0SortColumn.to_string(),
            sort_column.to_string(),
        );
        (self.configmanager_r).borrow().set_val(
            &PropDef::GuiList0SortAscending.to_string(),
            ascending.to_string(),
        );
    }

    fn launch_browser_single(&self, db_ids: Vec<i32>) {
        db_ids
            .iter()
            .filter_map(|msg_id| {
                let o_msg = (*self.messagesrepo_r)
                    .borrow()
                    .get_by_index(*msg_id as isize);

                let list_pos = self.msg_state.read().unwrap().get_gui_pos(*msg_id as isize);
                o_msg.as_ref()?;
                Some((*msg_id as isize, o_msg.unwrap().link, list_pos))
            })
            .for_each(|(db_id, url, list_pos)| {
                (self.downloader_r)
                    .borrow()
                    .launch_webbrowser(url, db_id, list_pos);
            });
    }

    fn launch_browser_selected(&self) {
        let id_list: Vec<i32> = self.list_selected_ids.read().unwrap().clone();
        self.launch_browser_single(id_list);
    }

    fn get_msg_content_author_categories(
        &self,
        msg_id: isize,
        current_row: Option<&MessageRow>,
    ) -> (String, String, String) {
        let contains = self.msg_state.read().unwrap().contains(msg_id);
        if !contains {
            if let Some(msg) = current_row {
                self.insert_state_from_row(msg, None);
            }
        }
        let o_co_au_ca = self
            .msg_state
            .read()
            .unwrap()
            .get_contents_author_categories(msg_id);
        if let Some((co, au, ca)) = o_co_au_ca {
            return (co, au, ca);
        }
        let msg = (*self.messagesrepo_r)
            .borrow()
            .get_by_index(msg_id)
            .unwrap();

        let triplet = (
            decompress(&msg.content_text),
            decompress(&msg.author),
            decompress(&msg.categories),
        );
        self.msg_state
            .write()
            .unwrap()
            .set_contents_author_categories(msg_id, &triplet);
        triplet
    }

    fn move_list_cursor(&self, c: ListMoveCommand) {
        if ListMoveCommand::None == c {
            return;
        };
        let (last_subs_id, _num_msg, _isfolder) = *self.current_subscription.borrow();
        if last_subs_id <= 0 {
            return;
        }
        let selected = self.list_selected_ids.read().unwrap().clone();
        if selected.is_empty() {
            return;
        }
        let first_selected_msg: isize = selected[0] as isize;
        let select_later: bool = ListMoveCommand::LaterUnreadMessage == c;
        let o_dest_subs_id = self
            .msg_state
            .read()
            .unwrap()
            .find_unread_message(first_selected_msg, select_later);
        // trace!("move_list_cursor: dest_ids= {:?} ", o_dest_subs_id);
        if let Some((dest_id, next_dest_id)) = o_dest_subs_id {
            (*self.gui_updater).borrow().list_set_cursor(
                TREEVIEW1,
                dest_id,
                LIST0_COL_MSG_ID,
                LIST_SCROLL_POS,
            );
            let o_co_au_ca = self.get_msg_content_author_categories(next_dest_id, None);
            (*self.browserpane_r)
                .borrow()
                .browser_pre_load(next_dest_id as i32, Some(o_co_au_ca));
        }
    }

    fn set_messages_filter(&mut self, newtext: &str) {
        let trimmed = newtext.trim();
        if trimmed.is_empty() {
            self.msg_filter = None;
        } else {
            self.msg_filter.replace(trimmed.to_string());
        }
        self.addjob(CJob::UpdateMessageList);
    }

    fn keyboard_delete(&self) {
        let del_ids = self.list_selected_ids.read().unwrap();
        self.delete_messages(&del_ids);
    }

    fn memory_conserve(&mut self, act: bool) {
        self.window_minimized = act;
        if act {
            self.msg_state.write().unwrap().clear();
        } else {
            self.fill_state_map(&Vec::default());
            let (_, _, isfolder) = *self.current_subscription.borrow();
            if isfolder {
                self.addjob(CJob::UpdateMessageList); // when folder is selected, we would have no messages synced else
            }
            self.addjob(CJob::ListSetCursorToPolicy);
        }
    }

    // impl IContentList
}

impl Buildable for FeedContents {
    type Output = FeedContents;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let mut fc = FeedContents::new(_appcontext);
        if let Some(i) = conf.get_int(FeedContents::CONF_FOCUS_POLICY) {
            fc.config.focus_policy = i as u8;
        } else {
            fc.config.focus_policy = 1;
        }
        if let Some(i) = conf.get_int(FeedContents::CONF_MSG_KEEP_COUNT) {
            fc.config.message_keep_count = i as i32;
        } else {
            fc.config.message_keep_count = Self::CONF_MSG_KEEP_COUNT_DEFAULT;
        }
        if let Some(i) = conf.get_int(&PropDef::GuiList0SortColumn.to_string()) {
            fc.config.list_sort_column = i as u8;
        } else {
            fc.config.list_sort_column = 0;
        }
        fc.config.list_sort_order_up = conf.get_bool(&PropDef::GuiList0SortAscending.to_string());
        fc
    }
}

impl StartupWithAppContext for FeedContents {
    fn startup(&mut self, ac: &AppContext) {
        self.feedsources_w = Rc::downgrade(&(*ac).get_rc::<SourceTreeController>().unwrap());
        let feedcontents_r = ac.get_rc::<FeedContents>().unwrap();
        {
            let mut t = (*self.timer_r).borrow_mut();
            t.register(&TimerEvent::Timer100ms, feedcontents_r.clone(), true);
            t.register(&TimerEvent::Timer2s, feedcontents_r, true);
        }

        if let Some(s) = (*self.configmanager_r)
            .borrow()
            .get_sys_val(ConfigManager::CONF_MODE_DEBUG)
        {
            if let Ok(b) = s.parse::<bool>() {
                self.config.mode_debug = b;
            }
        }
    }
}

impl TimerReceiver for FeedContents {
    fn trigger_mut(&mut self, event: &TimerEvent) {
        if self.window_minimized {
            if event == &TimerEvent::Timer2s {
                self.process_jobs();
            }
        } else if event == &TimerEvent::Timer100ms {
            self.process_jobs();
        }
    }
}

enum ContentMatchMask {
    EntrySrcDate = 1,
    PostId = 2,
    Title = 4,
}

///  returns bitfield:
///     1: entry_src_date
///     2: post_id
///     4: title
pub fn match_fce(existing: &MessageRow, new_fce: &MessageRow) -> u8 {
    let mut match_bits: u8 = 0;
    if existing.entry_src_date == new_fce.entry_src_date {
        match_bits |= ContentMatchMask::EntrySrcDate as u8;
    };
    if !existing.post_id.is_empty() && (existing.post_id == new_fce.post_id) {
        match_bits |= ContentMatchMask::PostId as u8;
    };
    if existing.title == new_fce.title {
        match_bits |= ContentMatchMask::Title as u8;
    };
    match_bits
}

pub fn match_new_entries_to_existing(
    new_list: &Vec<MessageRow>,
    existing_entries: &[MessageRow],
    job_sender: Sender<CJob>,
) -> Vec<MessageRow> {
    let mut new_list_delete_indices: Vec<usize> = Vec::default();
    for idx_new in 0..new_list.len() {
        let n_fce: &MessageRow = new_list.get(idx_new).unwrap();
        let mut exi_pos_match: HashMap<usize, u8> = HashMap::new();
        let mut max_ones_count: u8 = 0;
        existing_entries.iter().enumerate().for_each(|(n, ee)| {
            let matchfield: u8 = match_fce(ee, n_fce);
            let ones_count: u8 = matchfield.count_ones() as u8;
            if ones_count > 1 {
                exi_pos_match.insert(n, ones_count);
            }
            if ones_count > max_ones_count {
                max_ones_count = ones_count
            }
        });
        let pos_with_max_ones = exi_pos_match
            .iter()
            .find(|(_pos, ones_count)| **ones_count >= max_ones_count)
            .map(|(pos, _ones_count_)| pos);
        if let Some(pos) = pos_with_max_ones {
            let exi_fce = existing_entries.get(*pos).unwrap();
            let matchfield: u8 = match_fce(exi_fce, n_fce);
            if matchfield.count_ones() >= 3 {
                new_list_delete_indices.push(idx_new); // full match
            }
            if matchfield.count_ones() == 2 {
                let inv_match = !matchfield & 7;
                if inv_match & ContentMatchMask::EntrySrcDate as u8 > 0 {
                    let _r = job_sender.send(CJob::DbUpdateEntryDate(
                        exi_fce.message_id,
                        n_fce.entry_src_date as u64,
                    ));
                }
                if inv_match & ContentMatchMask::PostId as u8 > 0 {
                    let _r = job_sender.send(CJob::DbUpdatePostId(
                        exi_fce.message_id,
                        n_fce.post_id.clone(),
                    ));
                }
                if inv_match & ContentMatchMask::Title as u8 > 0 {
                    let _r = job_sender.send(CJob::DbUpdateTitle(
                        exi_fce.message_id,
                        n_fce.title.to_string(),
                    ));
                }
                new_list_delete_indices.push(idx_new); // entry corrected
            }
        }
    }
    let ret_list: Vec<MessageRow> = new_list
        .iter()
        .enumerate()
        .filter(|(i, _fce)| !new_list_delete_indices.contains(i))
        .map(|(_i, fce)| fce.clone())
        .collect::<Vec<MessageRow>>();
    ret_list
}

#[derive(Clone, Debug)]
pub struct Config {
    /// None,    LastSelected,    MostRecent,    BeforeUnread
    pub focus_policy: u8,
    pub message_keep_count: i32,
    /// 1: display, 2: timestamp  3: isread
    pub list_sort_column: u8,
    pub list_sort_order_up: bool,
    pub mode_debug: bool,
    pub list_fontsize: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            focus_policy: 1,
            message_keep_count: 980,
            list_sort_column: 0,
            list_sort_order_up: false,
            mode_debug: false,
            list_fontsize: 10,
        }
    }
}

pub fn get_font_size_from_config(configmanager_r: Rc<RefCell<ConfigManager>>) -> u32 {
    if (*configmanager_r)
        .borrow()
        .get_val_bool(&PropDef::GuiFontSizeManualEnable.to_string())
    {
        return (*configmanager_r)
            .borrow()
            .get_val_int(&PropDef::GuiFontSizeManual.to_string())
            .unwrap_or(0) as u32;
    }
    0
}

#[derive(Debug, PartialEq, Eq)]
pub enum ListMoveCommand {
    None,
    LaterUnreadMessage,
    PreviousUnreadMessage,
}
