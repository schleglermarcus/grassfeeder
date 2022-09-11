use crate::config::configmanager::ConfigManager;
use crate::controller::browserpane::BrowserPane;
use crate::controller::browserpane::IBrowserPane;
use crate::controller::sourcetree::ISourceTreeController;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::db::message::decompress;
use crate::db::message::MessageRow;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::timer::Timer;
use crate::ui_select::gui_context::GuiContext;
use crate::util::db_time_to_display;
use crate::util::remove_invalid_chars_from_input;
use chrono::DateTime;
use chrono::Local;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use feed_rs::model::Entry;
use flume::Receiver;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::gui_values::FontAttributes;
use gui_layer::gui_values::PropDef;
use resources::gen_icons;
use resources::id::LIST0_COL_MSG_ID;
use resources::id::TREEVIEW1;
use resources::names::FOCUS_POLICY_NAMES;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::rc::Weak;
use std::sync::RwLock;
use url::Url;
use webbrowser;

const JOBQUEUE_SIZE: usize = 100;

// #[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CJob {
    /// content_id, newtitle
    DbUpdateTitle(isize, String),
    /// content_id, new-post-id
    DbUpdatePostId(isize, String),
    /// content_id, entry-date
    DbUpdateEntryDate(isize, u64),
    ///  list_position, feed_content_id
    UpdateContentListSome(Vec<(u32, u32)>),
    /// feed_content_id
    SwitchBrowserTabContent(i32),
    ///
    ListSetCursorToPolicy,
    ///  db-id
    StartWebBrowser(i32),
    /// feed-source-id
    RequestUnreadAllCount(isize),
}

pub trait IFeedContents {
    fn addjob(&self, nj: CJob);
    fn process_jobs(&mut self);

    /// Sets those entries read, updates the  gui-store
    //  If the list entries are already read, don't update them
    ///  Map<  repo-id  ,   list-position >
    fn process_list_row_activated(&self, act_dbid_listpos: &HashMap<i32, i32>);

    /// Read from db and put into the list view,
    fn update_feed_list_contents(&self, feed_source_id: isize);

    ///  Vec < list_position,   feed_content_id >
    fn update_content_list_some(&self, vec_pos_dbid: &[(u32, u32)]);

    /// for clicking on the is-read icon
    fn toggle_feed_item_read(&self, content_repo_id: isize, list_position: i32);

    /// updates existing entries,  returns the new entries only,
    fn match_new_entries_to_db(
        &self,
        new_list: &[MessageRow],
        source_repo_id: isize,
    ) -> Vec<MessageRow>;

    fn get_job_receiver(&self) -> Receiver<CJob>;
    fn get_job_sender(&self) -> Sender<CJob>;

    fn set_read_all(&mut self, source_repo_id: isize);

    //  all content entries, unread content entries
    fn get_counts(&self, source_repo_id: isize) -> Option<(i32, i32)>;
    fn get_config(&self) -> Config;

    fn set_conf_focus_policy(&mut self, n: u8);
    fn set_conf_msg_keep_count(&mut self, n: i32);
    fn notify_config_update(&mut self);

    fn process_list_action(&self, action: String, repoid: Vec<(i32, i32)>);
    fn set_sort_order(&mut self, sort_column: u8, order_up: bool);
    fn start_web_browser(&self, db_ids: Vec<i32>);

    fn set_selected_content_ids(&self, list: Vec<i32>);
    fn get_selected_content_ids(&self) -> Vec<i32>;

    fn get_msg_state(&self, msg_id: isize, current_row: Option<&MessageRow>) -> FeedContentState;

    ///  decompressed
    fn get_msg_content_author_categories(
        &self,
        msg_id: isize,
        current_row: Option<&MessageRow>,
    ) -> (String, String, String);

    fn set_state_gui_listpos(
        &self,
        msg_id: isize,
        listpos: isize,
        current_row: Option<&MessageRow>,
    );
}

/// needs GuiContext  ConfigManager  BrowserPane  Downloader
pub struct FeedContents {
    timer_r: Rc<RefCell<Timer>>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_val_store: UIAdapterValueStoreType,
    feedsources_w: Weak<RefCell<SourceTreeController>>,
    configmanager_r: Rc<RefCell<ConfigManager>>,
    browserpane_r: Rc<RefCell<dyn IBrowserPane>>,
    job_queue_receiver: Receiver<CJob>,
    job_queue_sender: Sender<CJob>,
    last_activated_subscription_id: RefCell<isize>,

    /// source-repo-id  -> state,
    /// Later check:  shall contain only the current selected subscription's message states.
    fc_state_map: RwLock<HashMap<isize, FeedContentState>>,
    config: Config,
    list_fontsize: u32,
    list_selected_ids: RwLock<Vec<i32>>,
    messagesrepo_r: Rc<RefCell<dyn IMessagesRepo>>,
}

impl FeedContents {
    pub const CONF_FOCUS_POLICY: &'static str = "MessageSelectFocusPolicy";
    pub const CONF_MSG_KEEP_COUNT: &'static str = "MessagesKeepCount";

    pub fn new(ac: &AppContext) -> Self {
        let (q_s, q_r) = flume::bounded::<CJob>(JOBQUEUE_SIZE);
        let gc_r = (*ac).get_rc::<GuiContext>().unwrap();
        let u_a = (*gc_r).borrow().get_updater_adapter();
        let v_s_a = (*gc_r).borrow().get_values_adapter();
        let cm_r = (*ac).get_rc::<ConfigManager>().unwrap();
        let bp_r = (*ac).get_rc::<BrowserPane>().unwrap();
        let msg_r = (*ac).get_rc::<MessagesRepo>().unwrap();
        FeedContents {
            timer_r: (*ac).get_rc::<Timer>().unwrap(),
            gui_updater: u_a,
            gui_val_store: v_s_a,
            configmanager_r: cm_r,
            browserpane_r: bp_r,
            job_queue_receiver: q_r,
            job_queue_sender: q_s,
            feedsources_w: Weak::new(),
            last_activated_subscription_id: RefCell::new(-1),
            fc_state_map: Default::default(),
            config: Config::default(),
            list_fontsize: 0,
            list_selected_ids: RwLock::new(Vec::default()),
            messagesrepo_r: msg_r,
        }
    }

    // later:   remove  content-id  as tooltip
    fn message_to_row(
        fc: &MessageRow,
        fontsize: u32,
        title_d: String,
        debug_mode: bool,
    ) -> Vec<AValue> {
        let mut newrow: Vec<AValue> = Vec::default();
        newrow.push(AValue::AIMG(
            gen_icons::ICON_03_ICON_TRANSPARENT_48.to_string(),
        )); // 0
        newrow.push(AValue::ASTR(title_d)); // 1: message title
        if fc.entry_src_date > 0 {
            let mut displaytime = db_time_to_display(fc.entry_src_date);
            if fc.entry_invalid_pubdate {
                displaytime = format!("! {}", displaytime);
            }
            newrow.push(AValue::ASTR(displaytime));
        } else {
            newrow.push(AValue::None);
        }
        newrow.push(AValue::AIMG(match fc.is_read {
            true => gen_icons::ICON_06_CENTER_POINT_GREEN.to_string(),
            _ => gen_icons::ICON_16_DOCUMENT_PROPERTIES_48.to_string(),
        })); //  3
        newrow.push(AValue::AU32(FontAttributes::to_activation_bits(
            fontsize, fc.is_read,
        ))); // 4
        newrow.push(AValue::AU32(fc.message_id as u32)); // 5
        if debug_mode {
            newrow.push(AValue::ASTR(format!(
                "id{} src{}  postid:{}",
                fc.message_id, fc.subscription_id, fc.post_id
            )));
        } else {
            newrow.push(AValue::None);
        } // 6 :  tooltip
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
        debug!(
            "set_read_many: {}   repoids={:?}",
            *self.last_activated_subscription_id.borrow(),
            repo_ids
        );
        self.fc_state_map
            .write()
            .unwrap()
            .iter_mut()
            .filter(|(id, _st)| repo_ids.contains(&((**id) as i32)))
            .for_each(|(_id, st)| st.is_read_c_u = is_read);
        self.addjob(CJob::RequestUnreadAllCount(
            *self.last_activated_subscription_id.borrow(),
        ));
        let listpos_repoid: Vec<(u32, u32)> = repoid_listpos
            .iter()
            .map(|(r, p)| (*p as u32, *r as u32))
            .collect();
        self.addjob(CJob::UpdateContentListSome(listpos_repoid));
    }

    fn set_cursor_to_policy(&self) {
        let fp: usize = self.config.focus_policy as usize;
        assert!(fp < FOCUS_POLICY_NAMES.len());
        let fp_name = FOCUS_POLICY_NAMES[fp];
        match fp {
            1 => {
                (*self.gui_updater)
                    .borrow()
                    .list_set_cursor(TREEVIEW1, -1, LIST0_COL_MSG_ID); // None
            }
            2 => {
                let mut last_selected_msg_id: isize = -1; // Last Selected
                if let Some(feedsources) = self.feedsources_w.upgrade() {
                    if let Some(fse) = (*feedsources).borrow().get_current_selected_fse() {
                        last_selected_msg_id = fse.last_selected_msg;
                    }
                }
                if last_selected_msg_id > 0 {
                    (*self.gui_updater).borrow().list_set_cursor(
                        TREEVIEW1,
                        last_selected_msg_id,
                        LIST0_COL_MSG_ID,
                    );
                }
            }
            3 => {
                trace!(
                    "set_cursor  MostRecent {} {}  sort_col={}  sort_asc={}",
                    fp,
                    fp_name,
                    self.config.list_sort_column,
                    self.config.list_sort_order_up
                );
                let mut highest_created_timestamp: i64 = 0; // Most Recent
                let mut highest_ts_repo_id: isize = -1;
                self.fc_state_map
                    .read()
                    .unwrap()
                    .iter()
                    .for_each(|(fc_id, fc_state)| {
                        if fc_state.msg_created_timestamp > highest_created_timestamp {
                            highest_created_timestamp = fc_state.msg_created_timestamp;
                            highest_ts_repo_id = *fc_id;
                        }
                    });
                //debug!(                    "mostRecent={} {}",                    highest_ts_repo_id,                    highest_created_timestamp                );
                if highest_ts_repo_id > 0 {
                    (*self.gui_updater).borrow().list_set_cursor(
                        TREEVIEW1,
                        highest_ts_repo_id,
                        LIST0_COL_MSG_ID,
                    );
                }
            }
            4 => {
                info!("Later:  Before oldest unread {} {}", fp, fp_name);
            }
            _ => (),
        }
    }

    fn insert_state_from_row(&self, msg: &MessageRow, list_position: Option<isize>) {
        let mut st = FeedContentState {
            is_read_c_u: msg.is_read,
            gui_list_position: list_position.unwrap_or(-1),
            msg_created_timestamp: msg.entry_src_date,
            ..Default::default()
        };
        if !msg.title.is_empty() {
            st.title_d = decompress(&msg.title);
        }
        self.fc_state_map
            .write()
            .unwrap()
            .insert(msg.message_id, st);
    }
} // impl FeedContents

impl IFeedContents for FeedContents {
    fn addjob(&self, nj: CJob) {
        if self.job_queue_sender.is_full() {
            error!("FeedContents CJob queue full  Skipping  {:?}", nj);
        } else {
            self.job_queue_sender.send(nj).unwrap();
        }
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
                CJob::UpdateContentListSome(ref vec_pos_db) => {
                    self.update_content_list_some(vec_pos_db);
                    let list_pos: Vec<u32> =
                        vec_pos_db.iter().map(|(lp, _db)| *lp).collect::<Vec<u32>>();
                    (*self.gui_updater)
                        .borrow()
                        .update_list_some(TREEVIEW1, &list_pos);
                }

                CJob::SwitchBrowserTabContent(fc_id) => {
                    let mut st = self.get_msg_state(fc_id as isize, None);
                    if st.contents_author_categories_d.is_none() {
                        let msg = (*self.messagesrepo_r)
                            .borrow()
                            .get_by_index(fc_id as isize)
                            .unwrap();
                        let triplet =
                            self.get_msg_content_author_categories(fc_id as isize, Some(&msg));
                        st.contents_author_categories_d.replace(triplet);
                    }
                    // debug!("JOB {} SwitchBrowserTabContent  {:?}", fc_id, st);
                    (*self.browserpane_r)
                        .borrow()
                        .switch_browsertab_content(fc_id, st);
                }
                CJob::ListSetCursorToPolicy => self.set_cursor_to_policy(),
                CJob::StartWebBrowser(db_id) => {
                    if let Some(fce) = (*self.messagesrepo_r).borrow().get_by_index(db_id as isize)
                    {
                        let r = webbrowser::open(&fce.link);
                        if let Err(e) = r {
                            warn!("opening web page {} {}", &fce.link, e);
                        }
                    }
                }
                CJob::RequestUnreadAllCount(feed_source_id) => {
                    let msg_count = (*self.messagesrepo_r).borrow().get_src_sum(feed_source_id);
                    let read_count = (*self.messagesrepo_r).borrow().get_read_sum(feed_source_id);
                    let unread_count = msg_count - read_count;
                    if msg_count >= 0 {
                        // trace!(                        "RequestUnreadAllCount: {:?}  {}/{}",                        feed_source_id,                        unread_count,                        msg_count                    );
                        if let Some(feedsources) = self.feedsources_w.upgrade() {
                            (*feedsources).borrow().addjob(SJob::NotifyTreeReadCount(
                                feed_source_id,
                                msg_count,
                                unread_count,
                            ));
                        }
                    }
                }
            }
            let elapsed_m = now.elapsed().as_millis();
            if elapsed_m > 100 {
                debug!("CJOB: {:?} took {:?}", &job, elapsed_m);
            }
        }
    }

    /// Sets those entries read, updates the  gui-store
    ///  If the list entries are already read, don't update them
    ///  Map<  repo-id  ,   list-position >
    ///       list-position comes from treemodel.path
    fn process_list_row_activated(&self, act_dbid_listpos: &HashMap<i32, i32>) {
        let fc_repo_ids: Vec<i32> = act_dbid_listpos.keys().cloned().collect::<Vec<i32>>();
        let unread_repo_ids = fc_repo_ids
            .iter()
            .filter(|c_id| {
                if let Some(st) = self.fc_state_map.read().unwrap().get(&(**c_id as isize)) {
                    !st.is_read_c_u
                } else {
                    true
                }
            })
            .map(|c_id| *c_id as i32)
            .collect::<Vec<i32>>();
        self.fc_state_map
            .write()
            .unwrap()
            .iter_mut()
            .filter(|(id, _st)| fc_repo_ids.contains(&(**id as i32)))
            .for_each(|(_id, st)| {
                st.is_read_c_u = true;
            });
        let (last_content_id, _last_list_pos) = act_dbid_listpos.iter().last().unwrap();
        self.addjob(CJob::SwitchBrowserTabContent(*last_content_id));
        let list_pos_dbid = act_dbid_listpos
            .iter()
            .map(|(k, v)| (*v as u32, *k as u32))
            .collect::<Vec<(u32, u32)>>();
        if !unread_repo_ids.is_empty() {
            // trace!("process_list_row_activated:  list_pos_dbid={:?}   #unread={}  act_id={}",                list_pos_dbid,                unread_repo_ids.len(),                *self.last_activated_subscription_id.borrow()            );
            (*self.messagesrepo_r)
                .borrow_mut()
                .update_is_read_many(&unread_repo_ids, true);
            self.addjob(CJob::RequestUnreadAllCount(
                *self.last_activated_subscription_id.borrow(),
            ));
        }
        self.addjob(CJob::UpdateContentListSome(list_pos_dbid));
        if let Some(feedsources) = self.feedsources_w.upgrade() {
            (*feedsources)
                .borrow()
                .addjob(SJob::UpdateLastSelectedMessageId(
                    *self.last_activated_subscription_id.borrow(),
                    *last_content_id as isize,
                ));
        }
        self.set_selected_content_ids(vec![*last_content_id]);
    }

    // later:  check if update_feed_list_contents()    is a good option, or if cursor shall remain
    fn set_read_all(&mut self, src_repo_id: isize) {
        (*self.messagesrepo_r)
            .borrow_mut()
            .update_is_read_all(src_repo_id, true);
        self.update_feed_list_contents(src_repo_id);
        self.addjob(CJob::RequestUnreadAllCount(src_repo_id));
    }

    /// Read from db and put into the list view,
    /// State Map shall contain only the current subscription's messages, for finding the cursor position for the focus policy
    fn update_feed_list_contents(&self, feed_source_id: isize) {
        self.last_activated_subscription_id.replace(feed_source_id);
        let mut messagelist: Vec<MessageRow> =
            (*(self.messagesrepo_r.borrow_mut())).get_by_src_id(feed_source_id);

        self.fc_state_map.write().unwrap().clear();
        messagelist.iter_mut().enumerate().for_each(|(i, fc)| {
            self.insert_state_from_row(fc, Some(i as isize));
        });
        let mut valstore = (*self.gui_val_store).write().unwrap();
        valstore.clear_list(0);
        messagelist.iter_mut().enumerate().for_each(|(i, fc)| {
            let mut title_string = String::default();
            if let Some(st) = self.fc_state_map.read().unwrap().get(&(fc.message_id)) {
                title_string = st.title_d.clone();
            }
            valstore.insert_list_item(
                0,
                i as i32,
                &Self::message_to_row(
                    fc,
                    self.list_fontsize as u32,
                    title_string,
                    self.config.mode_debug,
                ),
            );
        });
        (*self.gui_updater).borrow().update_list(TREEVIEW1);
        self.addjob(CJob::ListSetCursorToPolicy);
        self.list_selected_ids.write().unwrap().clear();
    }

    ///  Vec < list_position,   feed_content_id >
    fn update_content_list_some(&self, vec_pos_dbid: &[(u32, u32)]) {
        for (list_position, feed_content_id) in vec_pos_dbid {
            let o_msg: Option<MessageRow> =
                (*(self.messagesrepo_r.borrow_mut())).get_by_index(*feed_content_id as isize);
            if o_msg.is_none() {
                warn!("update_single: no messsage for {}", feed_content_id);
                continue;
            }
            let msg: MessageRow = o_msg.unwrap();

            if let Some(state) = self.fc_state_map.read().unwrap().get(&msg.message_id) {
                // trace!(                "update_content_list_some {:?} {} ",                vec_pos_dbid,                title_text            );
                let av_list = Self::message_to_row(
                    &msg,
                    self.list_fontsize as u32,
                    state.title_d.clone(),
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
    fn toggle_feed_item_read(&self, content_repo_id: isize, list_position: i32) {
        let msg = (*(self.messagesrepo_r.borrow()))
            .get_by_index(content_repo_id)
            .unwrap();
        (*(self.messagesrepo_r.borrow_mut()))
            .update_is_read_many(&[content_repo_id as i32], !msg.is_read);
        let vec_pos_db: Vec<(u32, u32)> = vec![(list_position as u32, content_repo_id as u32)];
        self.update_content_list_some(&vec_pos_db);
        (*self.gui_updater)
            .borrow()
            .update_list_some(TREEVIEW1, &[list_position as u32]);
        debug!(
            "toggle_feed_item_read {}",
            *self.last_activated_subscription_id.borrow()
        );
        self.addjob(CJob::RequestUnreadAllCount(
            *self.last_activated_subscription_id.borrow(),
        ));
    }

    /// updates existing entries,
    /// returns the new entries only,
    fn match_new_entries_to_db(
        &self,
        new_list: &[MessageRow],
        source_repo_id: isize,
    ) -> Vec<MessageRow> {
        let existing_entries = (*self.messagesrepo_r)
            .borrow()
            .get_by_src_id(source_repo_id);
        match_new_entries_to_existing(
            &new_list.to_vec(),
            &existing_entries,
            self.job_queue_sender.clone(),
        )
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
            .get_by_src_id(source_repo_id);
        let num_is_read = all.iter().filter(|fce| fce.is_read).count() as i32;
        Some((all.len() as i32, (all.len() as i32 - num_is_read) as i32))
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
        /*
                (*self.configmanager_r).borrow_mut().set_section_key(
                    &Self::section_name(),
                    FeedContents::CONF_FOCUS_POLICY,
                    n.to_string().as_str(),
                );
        */
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
        // (*self.configmanager_r).borrow_mut().set_section_key(
        //     &Self::section_name(),
        //     FeedContents::CONF_MSG_KEEP_COUNT,
        //     n.to_string().as_str(),
        // );
        (*self.configmanager_r)
            .borrow_mut()
            .set_val(FeedContents::CONF_MSG_KEEP_COUNT, n.to_string());
    }

    fn notify_config_update(&mut self) {
        self.list_fontsize = get_font_size_from_config(self.configmanager_r.clone());
        // debug!("config_update: {}", self.list_fontsize);
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

    fn process_list_action(&self, action: String, repoid_listpos: Vec<(i32, i32)>) {
        // trace!("process_list_action   {} ,  {:?}", action, repoid_listpos);
        match action.as_str() {
            "mark-as-read" => {
                self.set_read_many(&repoid_listpos, true);
            }
            "mark-as-unread" => {
                self.set_read_many(&repoid_listpos, false);
            }
            "open-in-browser" => {
                let repoids: Vec<i32> = repoid_listpos.iter().map(|(db, _lp)| *db).collect();
                trace!("list ->  start external browser {:?}", repoids);
                self.start_web_browser(repoids);
            }
            "messages-delete" => {
                debug!("TODO  delete messages  ");
            }
            "copy-link" => {
                debug!("TODO  instrument the clipboard ");
            }
            _ => {
                warn!("contentlist_action unknown {}", &action);
            }
        }
    }

    fn set_sort_order(&mut self, sort_column: u8, ascending: bool) {
        self.config.list_sort_column = sort_column;
        self.config.list_sort_order_up = ascending;
        /*
                (self.configmanager_r).borrow_mut().set_section_key(
                    &Self::section_name(),
                    &PropDef::GuiList0SortColumn.to_string(),
                    &sort_column.to_string(),
                );
                (self.configmanager_r).borrow_mut().set_section_key(
                    &Self::section_name(),
                    &PropDef::GuiList0SortAscending.to_string(),
                    &ascending.to_string(),
                );
        */
        (self.configmanager_r).borrow().set_val(
            &PropDef::GuiList0SortColumn.to_string(),
            sort_column.to_string(),
        );
        (self.configmanager_r).borrow().set_val(
            &PropDef::GuiList0SortAscending.to_string(),
            ascending.to_string(),
        );
    }

    fn start_web_browser(&self, db_ids: Vec<i32>) {
        db_ids
            .iter()
            .for_each(|dbid| self.addjob(CJob::StartWebBrowser(*dbid)));
    }

    fn get_msg_state(&self, msg_id: isize, current_row: Option<&MessageRow>) -> FeedContentState {
        let sm_r = self.fc_state_map.read().unwrap();
        if sm_r.contains_key(&msg_id) {
            return sm_r.get(&msg_id).unwrap().clone();
        }
        if let Some(msg) = current_row {
            self.insert_state_from_row(msg, None);
        }
        if sm_r.contains_key(&msg_id) {
            return sm_r.get(&msg_id).unwrap().clone();
        }
        debug!("get_msg_state() fall through default ");
        FeedContentState::default()
    }

    fn get_msg_content_author_categories(
        &self,
        msg_id: isize,
        current_row: Option<&MessageRow>,
    ) -> (String, String, String) {
        let contains = self.fc_state_map.read().unwrap().contains_key(&msg_id);
        if !contains {
            if let Some(msg) = current_row {
                self.insert_state_from_row(msg, None);
            }
        }
        let mut needs_decompress: bool = false;
        if let Some(state) = self.fc_state_map.read().unwrap().get(&msg_id) {
            if state.contents_author_categories_d.is_none() {
                needs_decompress = true;
            } else {
                return state.contents_author_categories_d.as_ref().unwrap().clone();
            }
        }
        if needs_decompress {
            if let Some(msg) = current_row {
                let triplet = (
                    decompress(&msg.content_text),
                    decompress(&msg.author),
                    decompress(&msg.categories),
                );
                self.fc_state_map
                    .write()
                    .unwrap()
                    .get_mut(&msg_id)
                    .unwrap()
                    .contents_author_categories_d = Some(triplet.clone());
                return triplet;
            }
        }
        debug!("get_msg_content_author_categories() fall through default ");
        (String::default(), String::default(), String::default())
    }

    fn set_state_gui_listpos(
        &self,
        msg_id: isize,
        listpos: isize,
        current_row: Option<&MessageRow>,
    ) {
        let contains = self.fc_state_map.read().unwrap().contains_key(&msg_id);
        if !contains {
            if let Some(msg) = current_row {
                self.insert_state_from_row(msg, None);
            }
        }
        self.fc_state_map
            .write()
            .unwrap()
            .get_mut(&msg_id)
            .unwrap()
            .gui_list_position = listpos;
    }
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
            fc.config.message_keep_count = 100;
        }
        if let Some(i) = conf.get_int(&PropDef::GuiList0SortColumn.to_string()) {
            fc.config.list_sort_column = i as u8;
        } else {
            fc.config.list_sort_column = 0;
        }
        fc.config.list_sort_order_up = conf.get_bool(&PropDef::GuiList0SortAscending.to_string());
        fc
    }
    fn section_name() -> String {
        String::from("contentlist")
    }
}

impl StartupWithAppContext for FeedContents {
    fn startup(&mut self, ac: &AppContext) {
        self.feedsources_w = Rc::downgrade(&(*ac).get_rc::<SourceTreeController>().unwrap());
        let feedcontents_r = ac.get_rc::<FeedContents>().unwrap();
        {
            let mut t = (*self.timer_r).borrow_mut();
            t.register(&TimerEvent::Timer100ms, feedcontents_r);
        }
        self.config.mode_debug = (*self.configmanager_r)
            .borrow()
            .get_val_bool(ConfigManager::CONF_MODE_DEBUG);
    }
}

impl TimerReceiver for FeedContents {
    fn trigger(&mut self, event: &TimerEvent) {
        if event == &TimerEvent::Timer100ms {
            self.process_jobs();
        }
    }
}

///
///  takes the last of media[]  and brings it into enclosure_url
///
///   filter_by_iso8859_1().0;    // also removes umlauts
///  https://docs.rs/feed-rs/latest/feed_rs/model/struct.Entry.html#structfield.published
///         * RSS 2 (optional) "pubDate": Indicates when the item was published.
///
///  if title  contains invalid chars (for instance  & ), the Option<title>  is empty
pub fn message_from_modelentry(me: &Entry) -> MessageRow {
    let mut msg = MessageRow::default();
    let mut published_ts: i64 = 0;

    if let Some(publis) = me.published {
        published_ts = DateTime::<Local>::from(publis).timestamp();
    } else {
        if let Some(upd) = me.updated {
            published_ts = DateTime::<Local>::from(upd).timestamp();
        }
        msg.entry_invalid_pubdate = true;
    }
    msg.entry_src_date = published_ts;
    msg.fetch_date = crate::util::timestamp_now();
    msg.message_id = -1;
    if !me.links.is_empty() {
        msg.link = me.links.get(0).unwrap().href.clone();
    }
    if let Some(t) = me.title.clone() {
        let mut filtered = remove_invalid_chars_from_input(t.content);
        filtered = filtered.trim().to_string();
        msg.title = filtered; // not compressed yet
    } else {
        debug!("Message ID {} has no valid title. ", &me.id);
    }
    if let Some(summary) = me.summary.clone() {
        if !summary.content.is_empty() {
            msg.content_text = summary.content;
        }
    }
    msg.post_id = me.id.clone();
    if let Some(c) = me.content.clone() {
        if let Some(b) = c.body {
            msg.content_text = b
        }
        if let Some(enc) = c.src {
            msg.enclosure_url = enc.href
        }
    }
    for media in &me.media {
        // trace!("#content={}", media.content.len());
        for cont in &media.content {
            if let Some(m_url) = &cont.url {
                let u: Url = m_url.clone();
                if u.domain().is_some() {
                    msg.enclosure_url =
                        format!("{}://{}{}", u.scheme(), u.domain().unwrap(), u.path());
                }
            }
        }
        if msg.content_text.is_empty() {
            if let Some(descrip) = &media.description {
                if descrip.content_type.to_string().starts_with("text") {
                    // debug!(" content={:?}=", descrip.content);
                    msg.content_text = descrip.content.clone();
                }
            }
        }
    }
    let authorlist = me
        .authors
        .iter()
        .map(|author| author.name.clone())
        .filter(|a| a.as_str() != "author")
        .map(remove_invalid_chars_from_input)
        .collect::<Vec<String>>()
        .join(", ");
    let cate_list = me
        .categories
        .iter()
        .map(|cat| cat.term.clone())
        .map(remove_invalid_chars_from_input)
        .collect::<Vec<String>>()
        .join(", ");
    msg.author = authorlist;
    msg.categories = cate_list;
    msg
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
                exi_pos_match.insert(n, ones_count as u8);
            }
            if ones_count > max_ones_count {
                max_ones_count = ones_count
            }
        });
        let pos_with_max_ones = exi_pos_match
            .iter()
            .find(|(_pos, ones_count)| **ones_count >= max_ones_count)
            // .next()
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

#[derive(Default, Debug, Clone)]
pub struct FeedContentState {
    is_read_c_u: bool,
    /// remember  list position for setting the cursor
    gui_list_position: isize,
    msg_created_timestamp: i64,
    pub contents_author_categories_d: Option<(String, String, String)>,

    /// display text, decompressed
    pub title_d: String,
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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            focus_policy: 1,
            message_keep_count: 980,
            list_sort_column: 0,
            list_sort_order_up: false,
            mode_debug: false,
        }
    }
}

pub fn get_font_size_from_config(configmanager_r: Rc<RefCell<ConfigManager>>) -> u32 {
    /*
        if (*configmanager_r).borrow().get_section_key_bool(
            &GuiContext::section_name(),
            PropDef::GuiFontSizeManualEnable.to_string().as_str(),
        ) {
            if let Some(fs_man) = (*configmanager_r).borrow().
            get_section_key(
                &GuiContext::section_name(),
                PropDef::GuiFontSizeManual.to_string().as_str(),
            ) {
                if let Ok(s) = fs_man.parse::<u32>() {
                    return s;
                }
            }
        }
        0
    */
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

//------------------------------------------------------

#[cfg(test)]
mod feedcontents_test {

    use crate::controller::contentlist;
    use crate::db::message::MessageRow;
    use crate::util::db_time_to_display_nonnull;
    use chrono::DateTime;
    use feed_rs::parser;
    use std::fs;

    // TODO
    //cargo watch -s "cargo test controller::contentlist::feedcontents_test::parse_naturalnews_aug  --lib -- --exact --nocapture"
    #[ignore]
    #[test]
    fn parse_naturalnews_aug() {
        let rss_str = fs::read_to_string("../testing/tests/fr_htdocs/naturalnews_aug.xml").unwrap();
        let feeds = parser::parse(rss_str.as_bytes()).unwrap();
        let entry0 = feeds.entries.get(0).unwrap();
        let msg0: MessageRow = contentlist::message_from_modelentry(&entry0);
        println!("title={}=", msg0.title);
        //        assert!(msg2.title.starts_with("Borderlands-"));
    }

    //  Maybe later:
    //  The file contains an invalid  single  &  as title.   The parse does not like that and returns  no title.
    //cargo watch -s "cargo test controller::contentlist::feedcontents_test::parse_with_ampersand  --lib -- --exact --nocapture"
    // #[ignore]
    // #[test]
    #[allow(dead_code)]
    fn parse_with_ampersand() {
        let rss_str = fs::read_to_string("../testing/tests/fr_htdocs/dieneuewelle.xml").unwrap();
        let feeds = parser::parse(rss_str.as_bytes()).unwrap();
        let entry2 = feeds.entries.get(2).unwrap();
        let msg2: MessageRow = contentlist::message_from_modelentry(&entry2);
        // println!("entry2.title={}=", msg2.title);
        assert!(msg2.title.starts_with("Borderlands-"));
    }

    // #[ignore]
    #[test]
    fn parse_convert_entry_content_simple() {
        let rss_str = r#" <?xml version="1.0" encoding="UTF-8"?>
	        <rss   version="2.0"  xmlns:content="http://purl.org/rss/1.0/modules/content/" >
	        <channel>
	         <item>
	            <title>Rama Dama</title>
	              <description>Bereits sein Regie-Erstling war ein Hit</description>
	              <content:encoded>Lorem1</content:encoded>
	         </item>
	        </channel>
	        </rss>"#;
        let feeds = parser::parse(rss_str.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = contentlist::message_from_modelentry(&first_entry);
        assert_eq!(fce.content_text, "Lorem1");
    }

    /*
    id: String

    A unique identifier for this item with a feed. If not supplied it is initialised to a hash of the first link or a UUID if not available.

        Atom (required): Identifies the entry using a universally unique and permanent URI.
        RSS 2 (optional) “guid”: A string that uniquely identifies the item.
        RSS 1: does not specify a unique ID as a separate item, but does suggest the URI should be “the same as the link” so we use a hash of the link if found
        JSON Feed: is unique for that item for that feed over time.

    */
    // #[ignore]
    #[test]
    fn parse_feed_with_namespaces() {
        let rss_str = r#" <?xml version="1.0" encoding="UTF-8"?>
	        <rss xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:content="http://purl.org/rss/1.0/modules/content/" version="2.0">
	        <channel>
	            <title>Neu im Kino</title>
	            <item>
	              <title>Rama Dama</title>
	              <dc:creator>Kino.de Redaktion</dc:creator>
	              <content:encoded>Lorem2</content:encoded>
				  <guid>1234</guid>
	            </item>
	        </channel>
	        </rss>"#;
        let feeds = parser::parse(rss_str.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        assert!(!first_entry.authors.is_empty());
        assert_eq!(first_entry.authors[0].name, "Kino.de Redaktion");
        let fce: MessageRow = contentlist::message_from_modelentry(&first_entry);
        assert_eq!(fce.content_text, "Lorem2");
        assert_eq!(fce.post_id, "1234");
    }

    // #[ignore]
    #[test]
    fn message_from_modelentry_3() {
        let rsstext = r#" <?xml version="1.0" encoding="UTF-8"?>
	<rss xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:content="http://purl.org/rss/1.0/modules/content/" version="2.0">
	  <channel>
	    <description>Alle neuen Filme in den deutschen Kinos</description>
	    <language>de</language>
	    <copyright>Copyright 2021 Kino.de</copyright>
	    <title>Neu im Kino</title>
	    <lastBuildDate>Wed, 10 Nov 2021 00:12:03 +0100</lastBuildDate>
	    <link>https://www.kino.de/rss/stars</link>
	    <item>
	      <dc:creator>Kino.de Redaktion</dc:creator>
	      <description>Bereits sein Regie-Erstling war ein Hit</description>
	      <content:encoded>Felix Zeiler verbringt</content:encoded>
	      <enclosure url="https://static.kino.de/rama-dama-1990-film-rcm1200x0u.jpg" type="image/jpeg" length="153553"/>
	      <pubDate>Wed, 13 Oct 2021 12:00:00 +0200</pubDate>
	      <title>Rama Dama</title>
	      <link>https://www.kino.de/film/rama-dama-1990/</link>
	      <guid isPermaLink="true">https://www.kino.de/film/rama-dama-1990/</guid>
	    </item>
	  </channel>
	</rss>"#;
        let feeds = parser::parse(rsstext.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = contentlist::message_from_modelentry(&first_entry);
        assert_eq!(fce.content_text, "Felix Zeiler verbringt");
        assert_eq!(
            fce.enclosure_url,
            "https://static.kino.de/rama-dama-1990-film-rcm1200x0u.jpg"
        );
    }

    // #[ignore]
    #[test]
    fn parse_convert_entry_file1() {
        let rss_str = fs::read_to_string("tests/data/gui_proc_rss2_v1.rss").unwrap();
        let feeds = parser::parse(rss_str.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = contentlist::message_from_modelentry(&first_entry);
        assert_eq!(fce.content_text, "Today: Lorem ipsum dolor sit amet");
    }

    #[test]
    fn message_from_modelentry_4() {
        let rsstext = r#" <?xml version="1.0" encoding="UTF-8"?>
		<?xml-stylesheet type="text/xsl" media="screen" href="/~d/styles/rss2enclosuresfull.xsl"?>
		<?xml-stylesheet type="text/css" media="screen" href="http://feeds.feedburner.com/~d/styles/itemcontent.css"?>
		<rss xmlns:media="http://search.yahoo.com/mrss/" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:feedburner="http://rssnamespace.org/feedburner/ext/1.0" version="2.0">
		  <channel>
		    <title>THE FINANCIAL ARMAGEDDON BLOG</title>
		    <link>http://financearmageddon.blogspot.com/</link>
		    <description>&lt;i&gt;&lt;small&gt;THE | ECONOMIC COLLAPSE  | FINANCIAL ARMAGEDDON |  MELTDOWN | BLOG  is digging for the Truth Deep Down the Rabbit Hole , in Order to Prepare to Survive &amp;amp; Thrive the coming &lt;b&gt;Financial Apocalypse&lt;/b&gt; &amp;amp; &lt;b&gt;Economic Collapse&lt;/b&gt; &amp;amp;  be Ready for The Resistance to Tyranny and The NWO ,  Minds are like parachutes.......They only function when they are Open so Free Your Mind and come on join the ride&lt;/small&gt;&lt;/i&gt;</description>
		    <language>en</language>
		    <lastBuildDate>Wed, 10 Nov 2021 14:51:28 PST</lastBuildDate>
		<item>
	      <title>Warning : A 2 Quadrillions Debt Bubble by 2030     https://youtu.be/x6lmb992L0Q</title>
	      <link>http://feedproxy.google.com/~r/blogspot/cwWR/~3/wFtNHz9TStU/warning-2-quadrillions-debt-bubble-by.html</link>
	      <author>noreply@blogger.com (Politico Cafe)</author>
	      <pubDate>Mon, 01 Nov 2021 07:50:19 PDT</pubDate>
	      <guid isPermaLink="false">tag:blogger.com,1999:blog-8964382413486690048.post-7263323075085527050</guid>
	      <media:thumbnail url="https://img.youtube.com/vi/x6lmb992L0Q/default.jpg" height="72" width="72"/>
	      <thr:total xmlns:thr="http://purl.org/syndication/thread/1.0">0</thr:total>
	      <description>Warning : A 2 Quadrillions Debt Bubble by 2030     https://youtu.be/x6lmb992L0Q
	Central Banks are the new  Feudalism.
	All property is being concentrated into a few hands via Fiat and zero interest.
	Serfdom is the endgame.
	Central bankers were handed the Midas curse half a century...&lt;br/&gt;
	&lt;br/&gt;
	[[ This is a content summary only. Visit http://FinanceArmageddon.blogspot.com or  http://lindseywilliams101.blogspot.com  for full links, other content, and more! ]]&lt;img src="http://feeds.feedburner.com/~r/blogspot/cwWR/~4/wFtNHz9TStU" height="1" width="1" alt=""/&gt;</description>
	      <feedburner:origLink>http://financearmageddon.blogspot.com/2021/11/warning-2-quadrillions-debt-bubble-by.html</feedburner:origLink>
	    </item>
	  </channel>
	</rss>"#;
        let feeds = parser::parse(rsstext.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = contentlist::message_from_modelentry(&first_entry);
        assert!(fce.content_text.len() > 10);
    }

    //

    //cargo watch -s "cargo test controller::contentlist::feedcontents_test::from_modelentry_naturalnews  --lib -- --exact --nocapture"
    // TODO:  check condition
    #[ignore]
    #[test]
    fn from_modelentry_naturalnews() {
        let wrongdate = "Wed, 22 Jun 2022  15:59:0 CST";

        match DateTime::parse_from_rfc2822(&wrongdate) {
            Ok(_) => (),
            Err(e) => {
                println!("Problem: {:?} {:?} ", &wrongdate, &e);
                //				return Some(format!("Error parse_from_rfc2822(`{}`) {:?} ", &inner, &e));
            }
        }

        if false {
            let rsstext = r#"<?xml version="1.0" encoding="ISO-8859-1"?>
<rss xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" version="2.0">
  <channel>
    <title>NaturalNews.com</title>
    <lastBuildDate>Wed, 22 Jun 2022 00:00:00 CST</lastBuildDate>
    <item>
      <title>chemical</title>
      <description>hello</description>
      <author>mike</author>
      <pubDate>Wed, 22 Jun 2022  15:59:0 CST</pubDate>
    </item>
  </channel>
</rss>     "#;

            let feeds = parser::parse(rsstext.as_bytes()).unwrap();
            let first_entry = feeds.entries.get(0).unwrap();
            let fce: MessageRow = contentlist::message_from_modelentry(&first_entry);
            println!(
                "    entry_src_date={:?}",
                db_time_to_display_nonnull(fce.entry_src_date),
            );
        }
    }

    #[allow(dead_code)]
    fn from_modelentry_naturalnews_copy() {
        let rsstext = r#"<?xml version="1.0" encoding="ISO-8859-1"?>
<rss xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" version="2.0">
  <channel>
    <title>NaturalNews.com</title>
    <lastBuildDate>Wed, 22 Jun 2022 00:00:00 CST</lastBuildDate>
    <item>
      <title><![CDATA[RED ALERT: Entire U.S. supply of diesel engine oil may be wiped out in 8 weeks&#8230; no more oil until 2023 due to &#8220;Force Majeure&#8221; additive chemical shortages]]></title>
      <description><![CDATA[<table><tr><td><img src='wp-content/uploads/sites/91/2022/06/HRR-2022-06-22-Situation-Update_thumbnail.jpg' width='140' height='76' /></td><td valign='top'>(NaturalNews) <p> (Natural News)&#10; As if we all needed something else to add to our worries, a potentially catastrophic situation is emerging that threatens to wipe out the entire supply of diesel engine oil across the United States, leaving the country with no diesel engine oil until 2023.This isn't merely a rumor: We've confirmed this is &#x02026; [Read More...]</p></td></tr></table>]]></description>
      <author><![CDATA[Mike Adams]]></author>
      <pubDate>Wed, 22 Jun 2022  15:59:0 CST</pubDate>
      <link><![CDATA[https://www.naturalnews.com/2022-06-22-red-alert-entire-us-supply-of-diesel-engine-oil-wiped-out.html]]></link>
      <guid><![CDATA[https://www.naturalnews.com/2022-06-22-red-alert-entire-us-supply-of-diesel-engine-oil-wiped-out.html]]></guid>
    </item>
  </channel>
</rss>     "#;

        let feeds = parser::parse(rsstext.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = contentlist::message_from_modelentry(&first_entry);
        println!(
            "entry_src_date={:?}   ",
            db_time_to_display_nonnull(fce.entry_src_date),
        );
        assert!(fce.content_text.len() > 10);
    }
}
