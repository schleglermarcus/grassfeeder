// use dd::flume;
use crate::controller::contentlist::get_font_size_from_config;
use crate::controller::contentlist::CJob;
use crate::controller::contentlist::IFeedContents;
use crate::controller::sourcetree::Config;
use crate::controller::sourcetree::NewSourceState;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::controller::sourcetree::JOBQUEUE_SIZE;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_DELETED;
use crate::db::subscription_state::FeedSourceState;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::StatusMask;
use crate::db::subscription_state::SubsMapEntry;
use crate::opml::opmlreader::OpmlReader;
use crate::util::db_time_to_display_nonnull;
use crate::util::filter_by_iso8859_1;
use crate::util::remove_invalid_chars_from_input;
use crate::util::string_is_http_url;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use resources::id::*;
use std::cell::RefCell;
use std::rc::Rc;

pub trait ISourceTreeController {
    fn on_fs_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool;
    fn mark_schedule_fetch(&self, subscription_id: isize);
    fn set_tree_expanded(&self, subscription_id: isize, new_expanded: bool);
    fn addjob(&self, nj: SJob);

    fn add_new_subscription(&mut self, newsource: String, display: String) -> isize;
    fn add_new_subscription_at_parent(
        &mut self,
        newsource: String,
        display: String,
        parent_id: isize,
        load_messages: bool,
    ) -> isize;

    /// using internal state for parent id
    fn add_new_folder(&mut self, folder_name: String) -> isize;
    fn add_new_folder_at_parent(&mut self, folder_name: String, parent_id: isize) -> isize;
    fn set_fetch_in_progress(&self, subscription_id: isize);
    fn set_fetch_finished(&self, subscription_id: isize, error_happened: bool);

    fn get_job_sender(&self) -> Sender<SJob>;
    fn set_fs_delete_id(&mut self, o_fs_id: Option<usize>);
    fn get_config(&self) -> Rc<RefCell<Config>>;
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

    fn get_current_selected_subscription(&self) -> Option<(SubscriptionEntry, Vec<i32>)>;
    fn get_state(&self, search_id: isize) -> Option<SubsMapEntry>;
    /// writes the path array into the cached subscription list
    fn update_cached_paths(&self);

    fn clear_read_unread(&self, subs_id: isize);
    fn memory_conserve(&mut self, act: bool);

    fn set_selected_message_id(&mut self, subs_id: isize, msg_id: isize);
}

impl ISourceTreeController for SourceTreeController {
    fn on_fs_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool {
        trace!("START_DRAG {:?} => {:?}      ", &from_path, &to_path);
        let all1 = (*self.subscriptionrepo_r).borrow().get_all_entries();
        let length_before = all1.len();
        let mut success: bool = false;
        let mut from_parent_id: isize = -1;
        let mut to_parent_subs_id: isize = -1;
        match self.drag_calc_positions(&from_path, &to_path) {
            Ok((from_entry, to_parent_id, to_folderpos)) => {
                from_parent_id = from_entry.parent_subs_id;
                to_parent_subs_id = to_parent_id;
                self.drag_move(from_entry, to_parent_id, to_folderpos);
                let all2 = (*self.subscriptionrepo_r).borrow().get_all_entries();
                if all2.len() != length_before {
                    error!("Drag lost entries: {}->{}", length_before, all2.len());
                    success = false;
                } else {
                    success = true;
                }
            }
            Err(msg) => {
                warn!("DragFail: {:?}=>{:?} --> {} ", from_path, to_path, msg);
                (*self.subscriptionrepo_r)
                    .borrow()
                    .debug_dump_tree("dragfail");
            }
        }
        self.addjob(SJob::UpdateTreePaths);
        //  debug!("on-drag subs-id: from_p:{}  to_p:{}", from_parent_id, to_parent_subs_id        );
        self.addjob(SJob::FillSubscriptionsAdapter);
        if from_parent_id >= 0 {
            self.addjob(SJob::GuiUpdateTree(from_parent_id));
        }
        if to_parent_subs_id >= 0 && to_parent_subs_id != from_parent_id {
            self.addjob(SJob::GuiUpdateTree(to_parent_subs_id));
        }
        success
    }

    fn mark_schedule_fetch(&self, subs_id: isize) {
        let mut is_folder: bool = false;
        let su_st = self
            .statemap
            .borrow()
            .get_state(subs_id)
            .unwrap_or_default();
        if let Some(entry) = (*self.subscriptionrepo_r).borrow().get_by_index(subs_id) {
            is_folder = entry.is_folder;
            if entry.isdeleted() {
                return;
            }
            if su_st.is_fetch_scheduled() {
                return;
            }
        }
        if is_folder {
            let child_fse: Vec<SubscriptionEntry> = (*self.subscriptionrepo_r)
                .borrow()
                .get_by_parent_repo_id(subs_id);
            let child_repo_ids: Vec<isize> = child_fse
                .iter()
                .filter(|fse| !fse.is_folder)
                .map(|fse| fse.subs_id)
                .collect::<Vec<isize>>();
            self.statemap.borrow_mut().set_status(
                &child_repo_ids,
                StatusMask::FetchScheduled,
                true,
            );
        } else {
            self.statemap
                .borrow_mut()
                .set_status(&[subs_id], StatusMask::FetchScheduled, true);
            self.tree_store_update_one(subs_id);
            self.addjob(SJob::GuiUpdateTree(subs_id));
        }
    }

    fn mark_as_read(&self, src_repo_id: isize) {
        let mut is_folder: bool = false;
        if let Some(st) = self.statemap.borrow().get_state(src_repo_id) {
            is_folder = st.is_folder();
        }
        if is_folder {
            let child_fse: Vec<SubscriptionEntry> = (*self.subscriptionrepo_r)
                .borrow()
                .get_by_parent_repo_id(src_repo_id);
            child_fse
                .iter()
                .filter(|fse| !fse.is_folder)
                .for_each(|fse| {
                    if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                        (feedcontents)
                            .borrow_mut()
                            .set_read_complete_subscription(fse.subs_id);
                    }
                    self.statemap.borrow_mut().clear_num_all_unread(fse.subs_id);
                });
            self.addjob(SJob::ScanEmptyUnread);
            if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                feedcontents.borrow().addjob(CJob::UpdateMessageList);
            }
        } else if let Some(feedcontents) = self.feedcontents_w.upgrade() {
            (feedcontents)
                .borrow_mut()
                .set_read_complete_subscription(src_repo_id);
            feedcontents.borrow().addjob(CJob::UpdateMessageList);
        }
    }

    fn set_tree_expanded(&self, subs_id: isize, new_expanded: bool) {
        let src_vec = vec![subs_id];
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
            fse.folder_position = mfp + 1;
        }
        fse.subs_id = self.get_next_available_subscription_id();
        let r = (*self.subscriptionrepo_r).borrow().store_entry(&fse);
        match r {
            Ok(fse) => {
                self.addjob(SJob::UpdateTreePaths);
                self.addjob(SJob::FillSubscriptionsAdapter);
                self.addjob(SJob::FillSubscriptionsTreeUpdate);
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

    fn add_new_subscription(&mut self, newsource: String, display: String) -> isize {
        let p_id = self.current_new_folder_parent_id.unwrap_or(0);
        self.add_new_subscription_at_parent(newsource, display, p_id, false)
    }

    fn add_new_subscription_at_parent(
        &mut self,
        newsource: String,
        display: String,
        parent_id: isize,
        load_messages: bool,
    ) -> isize {
        let san_source = remove_invalid_chars_from_input(newsource.clone())
            .trim()
            .to_string();
        let mut san_display = remove_invalid_chars_from_input(display.clone())
            .trim()
            .to_string();
        let (filtered, was_truncated) = filter_by_iso8859_1(&san_display);
        if !was_truncated {
            san_display = filtered; // later see how to filter  https://www.ksta.de/feed/index.rss
        }
        let mut fse = SubscriptionEntry::from_new_url(san_display, san_source.clone());
        fse.subs_id = self.get_next_available_subscription_id();
        fse.parent_subs_id = parent_id;
        if was_truncated {
            let msg = format!("Found non-ISO chars in Subscription Title: {}", &display);
            (*self.erro_repo_r)
                .borrow()
                .add_error(fse.subs_id, 0, newsource, msg);
        }
        let max_folderpos: Option<isize> = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(parent_id)
            .iter()
            .map(|fse| fse.folder_position)
            .max();
        if let Some(mfp) = max_folderpos {
            fse.folder_position = mfp + 1;
        }
        let mut new_id = -1;
        match (*self.subscriptionrepo_r).borrow().store_entry(&fse) {
            Ok(fse2) => {
                self.addjob(SJob::UpdateTreePaths);
                if load_messages {
                    self.addjob(SJob::FillSubscriptionsAdapter);
                    self.addjob(SJob::FillSubscriptionsTreeUpdate);
                    self.addjob(SJob::ScheduleUpdateFeed(fse2.subs_id));
                    self.addjob(SJob::CheckSpinnerActive);
                }
                new_id = fse2.subs_id;
            }
            Err(e) => {
                error!(" add_new_subscription_at_parent >{}<  {:?}", &san_source, e);
            }
        }
        new_id
    }

    fn set_fetch_in_progress(&self, source_repo_id: isize) {
        self.statemap
            .borrow_mut()
            .set_status(&[source_repo_id], StatusMask::FetchInProgress, true);
        self.statemap
            .borrow_mut()
            .set_status(&[source_repo_id], StatusMask::FetchScheduled, false);
        self.statemap.borrow_mut().set_status(
            &[source_repo_id],
            StatusMask::FetchScheduledJobCreated,
            false,
        );
        self.set_any_spinner_visible(true);
        self.tree_store_update_one(source_repo_id);
    }

    fn set_fetch_finished(&self, source_repo_id: isize, error_happened: bool) {
        self.statemap.borrow_mut().set_status(
            &[source_repo_id],
            StatusMask::FetchInProgress,
            false,
        );
        self.statemap
            .borrow_mut()
            .set_status(&[source_repo_id], StatusMask::FetchScheduled, false);
        self.statemap.borrow_mut().set_status(
            &[source_repo_id],
            StatusMask::FetchScheduledJobCreated,
            false,
        );
        self.statemap.borrow_mut().set_status(
            &[source_repo_id],
            StatusMask::ErrFetchReq,
            error_happened,
        );
        self.addjob(SJob::CheckSpinnerActive);
        self.statemap
            .borrow_mut()
            .clear_num_all_unread(source_repo_id);
        if let Some((fse, _list)) = &self.get_current_selected_subscription() {
            if let Some(feedcontents) = self.feedcontents_w.upgrade() {
                if fse.subs_id == source_repo_id {
                    (*feedcontents).borrow().update_messagelist_only();
                } else {
                    (*feedcontents).borrow().update_message_list_(fse.subs_id);
                }
            }
        }
        self.addjob(SJob::ScanEmptyUnread);
        self.tree_store_update_one(source_repo_id);
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
        self.addjob(SJob::FillSubscriptionsAdapter);
        self.addjob(SJob::FillSubscriptionsTreeUpdate);
        self.addjob(SJob::GuiUpdateTreeAll);
        self.feedsource_delete_id = None;
    }

    // later: delete only those from trash bin
    fn feedsource_delete(&mut self) {
        if self.feedsource_delete_id.is_none() {
            return;
        }
        let fs_id = self.feedsource_delete_id.unwrap();
        (*self.subscriptionrepo_r)
            .borrow()
            .delete_by_index(fs_id as isize);
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSubscriptionsAdapter);
        self.addjob(SJob::FillSubscriptionsTreeUpdate);
        self.addjob(SJob::GuiUpdateTreeAll);
        self.feedsource_delete_id = None;
    }

    fn start_feedsource_edit_dialog(&mut self, src_repo_id: isize) {
        let mut dialog_id = DIALOG_FS_EDIT;
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id);
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
            let lines: Vec<String> = (*self.erro_repo_r)
                .borrow()
                .get_by_subscription(src_repo_id)
                .iter()
                .map(|ee| ee.to_line(fse.display_name.clone()))
                .collect();
            let joined = lines.join("\n");
            dd.push(AValue::ASTR(joined)); // 8
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
        assert!(!values.is_empty());
        let mut newname = String::default();
        if let Some(s) = values.get(0) {
            if let Some(t) = s.str() {
                newname = t;
            }
        }
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
            .get_by_index(src_repo_id)
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
        (*self.gui_updater).borrow().update_dialog(dialog_id);
        (*self.gui_updater).borrow().show_dialog(dialog_id);
    }

    fn start_delete_dialog(&mut self, src_repo_id: isize) {
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id);
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
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_FS_DELETE, &dd);
        (*self.gui_updater).borrow().update_dialog(DIALOG_FS_DELETE);
        (*self.gui_updater).borrow().show_dialog(DIALOG_FS_DELETE);
    }

    fn get_config(&self) -> Rc<RefCell<Config>> {
        self.config.clone()
    }

    fn set_conf_load_on_start(&mut self, n: bool) {
        (*self.config).borrow_mut().feeds_fetch_at_start = n;
        (*self.configmanager_r)
            .borrow()
            .set_val(SourceTreeController::CONF_FETCH_ON_START, n.to_string());
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
        (*self.config).borrow_mut().feeds_fetch_interval = n as u32;
        (*self.configmanager_r)
            .borrow()
            .set_val(SourceTreeController::CONF_FETCH_INTERVAL, n.to_string());
    }

    fn set_conf_fetch_interval_unit(&mut self, n: i32) {
        if !(1..=3).contains(&n) {
            error!("fetch_interval_unit wrong {}", n);
            return;
        }
        (*self.config).borrow_mut().feeds_fetch_interval_unit = n as u32;
        (*self.configmanager_r).borrow().set_val(
            SourceTreeController::CONF_FETCH_INTERVAL_UNIT,
            n.to_string(),
        );
    }

    fn set_conf_display_feedcount_all(&mut self, a: bool) {
        (*self.config).borrow_mut().display_feedcount_all = a;
        (*self.configmanager_r).borrow().set_val(
            SourceTreeController::CONF_DISPLAY_FEECOUNT_ALL,
            a.to_string(),
        );
    }

    fn newsource_dialog_edit(&mut self, edit_feed_url: String) {
        if edit_feed_url != self.new_source.edit_url {
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
        (*self.config).borrow_mut().tree_fontsize =
            get_font_size_from_config(self.configmanager_r.clone()) as u8;
        self.addjob(SJob::FillSubscriptionsAdapter);
        self.addjob(SJob::FillSubscriptionsTreeUpdate);
    }

    fn set_selected_feedsource(&mut self, src_repo_id: isize) {
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id);
        if let Some(fse) = o_fse {
            let display_name = fse.display_name.clone();
            if let Some(gui_context) = self.gui_context_w.upgrade() {
                (*gui_context).borrow_mut().set_window_title(display_name);
            }
            let mut child_ids: Vec<i32> = Vec::default();
            if fse.is_folder {
                child_ids = (*self.subscriptionrepo_r)
                    .borrow()
                    .get_by_parent_repo_id(fse.subs_id)
                    .iter()
                    .filter(|fse| !fse.is_folder)
                    .map(|fse| fse.subs_id as i32)
                    .collect::<Vec<i32>>();
            }
            self.current_selected_subscription = Some((fse, child_ids));
        }
    }

    fn set_selected_message_id(&mut self, subs_id: isize, msg_id: isize) {
        if self.current_selected_subscription.is_none() {
            return;
        }
        if let Some((mut fse, childs)) = self.current_selected_subscription.take() {
            if subs_id == fse.subs_id {
                fse.last_selected_msg = msg_id;
            } else {
                debug!(
                    "cannot set_selected_message_id() old subs_id:{} != {}",
                    fse.subs_id, subs_id
                );
            }
            self.current_selected_subscription.replace((fse, childs));
        }
    }

    fn import_opml(&mut self, filename: String) {
        let new_folder_id = self.add_new_folder_at_parent("import".to_string(), 0);
        let mut opmlreader = OpmlReader::new(self.subscriptionrepo_r.clone());
        match opmlreader.read_from_file(filename) {
            Ok(_) => {
                opmlreader.transfer_to_db(new_folder_id);
                self.addjob(SJob::UpdateTreePaths);
            }
            Err(e) => {
                warn!("reading opml {:?}", e);
            }
        }
        self.addjob(SJob::UpdateTreePaths);
        self.addjob(SJob::FillSubscriptionsAdapter);
        self.addjob(SJob::FillSubscriptionsTreeUpdate);
    }

    fn get_current_selected_subscription(&self) -> Option<(SubscriptionEntry, Vec<i32>)> {
        self.current_selected_subscription.clone()
    }

    fn get_state(&self, search_id: isize) -> Option<SubsMapEntry> {
        self.statemap.borrow().get_state(search_id)
    }

    fn update_cached_paths(&self) {
        self.update_paths_rec(&Vec::<u16>::default(), 0, false);
    }

    fn clear_read_unread(&self, subs_id: isize) {
        self.statemap.borrow_mut().clear_num_all_unread(subs_id);
        self.addjob(SJob::ScanEmptyUnread);
    }

    fn memory_conserve(&mut self, act: bool) {
        self.currently_minimized = act;
    }
}
