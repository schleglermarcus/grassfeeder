use crate::controller::contentlist::get_font_size_from_config;
use crate::controller::contentlist::CJob;
use crate::controller::contentlist::IContentList;
use crate::controller::sourcetree::errorentry_to_line;
use crate::controller::sourcetree::Config;
use crate::controller::sourcetree::NewSourceState;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::controller::sourcetree::JOBQUEUE_SIZE;
use crate::controller::subscriptionmove::ISubscriptionMove;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_state::FeedSourceState;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::StatusMask;
use crate::db::subscription_state::SubsMapEntry;
use crate::util::db_time_to_display;
use crate::util::db_time_to_display_nonnull;
use crate::util::string_is_http_url;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use resources::id::DIALOG_FOLDER_EDIT;
use resources::id::DIALOG_FS_DELETE;
use resources::id::DIALOG_SUBSCRIPTION_STATISTIC;
use resources::id::DIALOG_SUBS_EDIT;
use resources::id::LISTVIEW1;
use resources::id::TOOLBUTTON_RELOAD_ALL;
use resources::id::TREEVIEW0;
use std::cell::RefCell;
use std::rc::Rc;

pub trait ISourceTreeController {
    fn mark_schedule_fetch(&self, subscription_id: isize);
    fn set_tree_expanded(&self, subscription_id: isize, new_expanded: bool);
    fn addjob(&self, nj: SJob);
    fn get_job_sender(&self) -> Sender<SJob>;
    fn get_state(&self, search_id: isize) -> Option<SubsMapEntry>;
    fn clear_read_unread(&self, subs_id: isize);
    fn memory_conserve(&mut self, act: bool);

    fn set_fetch_in_progress(&self, subscription_id: isize);
    fn set_fetch_finished(&self, subscription_id: isize, error_happened: bool);
    fn mark_as_read(&self, src_repo_id: isize);

    fn get_config(&self) -> Rc<RefCell<Config>>;
    fn set_conf_load_on_start(&mut self, n: bool);
    fn set_conf_fetch_interval(&mut self, n: i32);
    fn set_conf_fetch_interval_unit(&mut self, n: i32);
    fn set_conf_display_feedcount_all(&mut self, a: bool);
    fn notify_config_update(&mut self);

    fn start_subscription_edit_dialog(&mut self, source_repo_id: isize);
    fn end_subscr_edit_dialog(&mut self, values: &[AValue]);
    fn start_new_fol_sub_dialog(&mut self, src_repo_id: isize, dialog_id: u8);
    fn start_delete_dialog(&mut self, src_repo_id: isize);
    fn newsource_dialog_edit(&mut self, edit_feed_url: String);
    fn set_ctx_subscription(&self, src_repo_id: isize);
    fn start_statistic_dialog(&self, subscription_id: isize);

    /// returns  Subscription,  Non-Folder-Child-IDs
    fn get_current_selected_subscription(&self) -> Option<(SubscriptionEntry, Vec<i32>)>;
    fn set_selected_message_id(&self, subs_id: isize, msg_id: isize);
    fn get_subs_icon_id(&self, subs_id: isize) -> usize;

    fn move_to_other_subscription(&self, move_up: bool);
}

impl ISourceTreeController for SourceTreeController {
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
            let child_fse: Vec<SubscriptionEntry> =
                (*self.subscriptionrepo_r).borrow().get_children(subs_id);
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

    fn mark_as_read(&self, subs_id: isize) {
        let mut is_folder: bool = false;
        if let Some(st) = self.statemap.borrow().get_state(subs_id) {
            is_folder = st.is_folder();
        }
        if is_folder {
            let child_fse: Vec<SubscriptionEntry> =
                (*self.subscriptionrepo_r).borrow().get_children(subs_id);
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
                .set_read_complete_subscription(subs_id);
            feedcontents.borrow().addjob(CJob::UpdateMessageList);
        }
    }

    fn set_tree_expanded(&self, subs_id: isize, new_expanded: bool) {
        let src_vec = vec![subs_id];
        (*self.subscriptionrepo_r)
            .borrow()
            .update_expanded(src_vec, new_expanded);
    }

    fn addjob(&self, nj: SJob) {
        if self.job_queue_sender.is_full() {
            warn!("SJob queue full: {}  Skipping {:?}", JOBQUEUE_SIZE, nj);
        } else {
            self.job_queue_sender.send(nj).unwrap();
        }
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
                let fc = (*feedcontents).borrow();
                if fse.subs_id == source_repo_id {
                    fc.addjob(CJob::UpdateMessageList);
                    fc.addjob(CJob::ListSetCursorToPolicy);
                } else {
                    fc.update_message_list(fse.subs_id);
                }
            }
        }
        self.addjob(SJob::ScanEmptyUnread);
        self.tree_store_update_one(source_repo_id);
    }

    fn get_job_sender(&self) -> Sender<SJob> {
        self.job_queue_sender.clone()
    }

    fn start_subscription_edit_dialog(&mut self, subs_id: isize) {
        let mut dialog_id = DIALOG_SUBS_EDIT;
        let o_fse = (*self.subscriptionrepo_r).borrow().get_by_index(subs_id);
        if o_fse.is_none() {
            return;
        }
        let fse = o_fse.unwrap();
        self.current_edit_fse.replace(fse.clone());
        let mut num_all: i32 = -1;
        let mut num_unread: i32 = -1;
        if let Some(feedcontents) = self.feedcontents_w.upgrade() {
            (num_all, num_unread) = (*feedcontents).borrow().get_counts(subs_id).unwrap();
        }
        let mut dd: Vec<AValue> = Vec::default();
        let mut iconval = AValue::None;
        if let Some(ie) = (*self.iconrepo_r)
            .borrow()
            .get_by_index(fse.icon_id as isize)
        {
            iconval = AValue::AIMG(ie.icon);
        }
        dd.push(AValue::ASTR(fse.display_name.clone())); // 0  url
        if fse.is_folder {
            dialog_id = DIALOG_FOLDER_EDIT;
        } else {
            dd.push(AValue::ASTR(fse.url.clone())); // 1
            dd.push(iconval); // 2
            dd.push(AValue::AI32(num_all)); // 3
            dd.push(AValue::AI32(num_unread)); // 4
            dd.push(AValue::ASTR(fse.website_url)); // 5 main website
            dd.push(AValue::ASTR(db_time_to_display_nonnull(fse.updated_int))); // 6
            dd.push(AValue::ASTR(db_time_to_display_nonnull(fse.updated_ext))); // 7
            let lines: Vec<String> = (*self.erro_repo_r)
                .borrow()
                .get_by_subscription(subs_id)
                .iter()
                .map(errorentry_to_line)
                .collect();
            let joined = lines.join("\n");
            dd.push(AValue::ASTR(joined)); // 8 error lines
        }
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(dialog_id, &dd);
        (*self.gui_updater).borrow().update_dialog(dialog_id);
        (*self.gui_updater).borrow().show_dialog(dialog_id);
    }

    fn start_statistic_dialog(&self, subscription_id: isize) {
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(subscription_id);
        if o_fse.is_none() {
            return;
        }
        let subscription = o_fse.unwrap();
        if subscription.is_folder || subscription.deleted {
            trace!("no info dialog on folder or deleted subscription");
            return;
        }
        let mut num_all: i32 = -1;
        let mut num_unread: i32 = -1;
        if let Some(feedcontents) = self.feedcontents_w.upgrade() {
            (num_all, num_unread) = (*feedcontents)
                .borrow()
                .get_counts(subscription_id)
                .unwrap();
        }
        let mut iconval = AValue::None;
        if let Some(ie) = (*self.iconrepo_r)
            .borrow()
            .get_by_index(subscription.icon_id as isize)
        {
            iconval = AValue::AIMG(ie.icon);
        }
        let mut num_favorites: i32 = -1;
        if let Some(msg_r) = self.messagesrepo_w.upgrade() {
            num_favorites = msg_r.borrow().count_favorites(subscription_id) as i32;
        }
        let mut dd: Vec<AValue> = vec![
            AValue::None,                                                       // 0
            AValue::ASTR(subscription.url.clone()),                             // 1
            iconval,                                                            // 2
            AValue::AI32(num_all),                                              // 3
            AValue::AI32(num_unread),                                           // 4
            AValue::ASTR(subscription.website_url),                             // 5 main website
            AValue::ASTR(db_time_to_display_nonnull(subscription.updated_int)), // 6
            AValue::ASTR(db_time_to_display_nonnull(subscription.updated_ext)), // 7
            AValue::AI32(num_favorites),                                        // 8
        ];
        if true {
            let err_list = (*self.erro_repo_r)
                .borrow()
                .get_by_subscription(subscription_id);
            // debug!(                "start_statistic_dialog   error messages into list !!   len:{} ",                err_list.len()            );
            let mut valstore = (*self.gui_val_store).write().unwrap();
            valstore.clear_list(LISTVIEW1);
            err_list.iter().enumerate().for_each(|(i, ee)| {
                let mut vrow: Vec<AValue> = Vec::default();
                vrow.push(AValue::ASTR(db_time_to_display(ee.date))); //0: DateTime
                let mut esrc_txt = t!(&format!("EM_DL_{}", ee.e_src));
                esrc_txt.truncate(40);
                vrow.push(AValue::ASTR(esrc_txt)); // 1 src - message
                vrow.push(AValue::AI32(ee.e_val as i32)); // 2 value
                vrow.push(AValue::ASTR(ee.remote_address.clone())); // 3 remote addr
                vrow.push(AValue::ASTR(ee.text.clone())); // 4 detail
                vrow.push(AValue::None); // 5 not yet
                vrow.push(AValue::ASTR(format!("TT ESRC {}", ee.e_src))); //6
                valstore.insert_list_item(LISTVIEW1, i as i32, &vrow);
            });
        } else {
            let lines: Vec<String> = (*self.erro_repo_r)
                .borrow()
                .get_by_subscription(subscription_id)
                .iter()
                .map(errorentry_to_line)
                .collect();
            let joined = lines.join("\n");
            dd.push(AValue::ASTR(joined)); // 8 error lines
        }
        (*self.gui_updater).borrow().update_list(LISTVIEW1);
        // debug!("start_statistic_dialog  update_list  idx:{} ", LISTVIEW1);
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_SUBSCRIPTION_STATISTIC, &dd);
        (*self.gui_updater)
            .borrow()
            .update_dialog(DIALOG_SUBSCRIPTION_STATISTIC);
        (*self.gui_updater)
            .borrow()
            .show_dialog(DIALOG_SUBSCRIPTION_STATISTIC);
    }

    fn end_subscr_edit_dialog(&mut self, values: &[AValue]) {
        if self.current_edit_fse.is_none() || values.is_empty() {
            return;
        }
        let subscr: SubscriptionEntry = self.current_edit_fse.take().unwrap();
        assert!(!values.is_empty());
        let mut newname = String::default();
        if let Some(s) = values.first() {
            if let Some(t) = s.str() {
                newname = t;
            }
        }
        let newname = (*newname).trim();
        if !newname.is_empty() && subscr.display_name != newname {
            (*self.subscriptionrepo_r)
                .borrow()
                .update_displayname(subscr.subs_id, newname.to_string());
            self.tree_store_update_one(subscr.subs_id);
        }
        if !subscr.is_folder {
            let new_url = values.get(1).unwrap().str().unwrap();
            let new_url = (*new_url).trim();
            if !new_url.is_empty() && subscr.url != new_url {
                (*self.subscriptionrepo_r)
                    .borrow()
                    .update_url(subscr.subs_id, new_url.to_string());
                self.addjob(SJob::ScheduleUpdateFeed(subscr.subs_id));
            }
            (*self.downloader_r)
                .borrow()
                .load_icon(subscr.subs_id, subscr.url, subscr.icon_id);
        }
    }

    fn start_new_fol_sub_dialog(&mut self, src_repo_id: isize, dialog_id: u8) {
        let mut new_parent_id = -1;
        match (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(src_repo_id)
        {
            None => {
                debug!("subscription {} not found ", src_repo_id);
            }
            Some(fse) => {
                if fse.is_folder {
                    new_parent_id = fse.subs_id;
                } else {
                    new_parent_id = fse.parent_subs_id;
                }
            }
        }
        if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
            subs_mov.borrow_mut().set_new_folder_parent(new_parent_id)
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
        if let Some(subs_mov) = self.subscriptionmove_w.upgrade() {
            subs_mov
                .borrow_mut()
                .set_fs_delete_id(Some(src_repo_id as usize));
        }
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
            SourceTreeController::CONF_DISPLAY_FEEDCOUNT_ALL,
            a.to_string(),
        );
        self.addjob(SJob::SetGuiTreeColumn1Width);
    }

    fn newsource_dialog_edit(&mut self, edit_feed_url: String) {
        if edit_feed_url != self.new_source.borrow().edit_url {
            self.new_source.borrow_mut().edit_url = edit_feed_url.trim().to_string();
            self.new_source.borrow_mut().state = NewSourceState::UrlChanged;
            if string_is_http_url(&self.new_source.borrow().edit_url) {
                (*self.downloader_r)
                    .borrow()
                    .new_feedsource_request(&self.new_source.borrow().edit_url);
            }
        }
    }

    fn notify_config_update(&mut self) {
        (*self.config).borrow_mut().tree_fontsize =
            get_font_size_from_config(self.configmanager_r.clone()) as u8;
        self.addjob(SJob::FillSubscriptionsAdapter);
        self.addjob(SJob::GuiUpdateTreeAll);
    }

    fn set_ctx_subscription(&self, src_repo_id: isize) {
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
                    .get_children(fse.subs_id)
                    .iter()
                    .filter(|fse| !fse.is_folder)
                    .map(|fse| fse.subs_id as i32)
                    .collect::<Vec<i32>>();
            }

            let activate_reloadbutton = !fse.is_folder || !child_ids.is_empty();
            // trace!(                "isfolder{}  #childs{}  act {}  ",                fse.is_folder,                child_ids.len(),                activate_reloadbutton            );
            (*self.gui_updater)
                .borrow()
                .toolbutton_set_sensitive(TOOLBUTTON_RELOAD_ALL, activate_reloadbutton);
            self.current_selected_subscription
                .replace(Some((fse, child_ids)));
        }
    }

    fn set_selected_message_id(&self, subs_id: isize, msg_id: isize) {
        if self.current_selected_subscription.borrow().is_none() {
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
            self.current_selected_subscription
                .replace(Some((fse, childs)));
        }
    }

    fn get_current_selected_subscription(&self) -> Option<(SubscriptionEntry, Vec<i32>)> {
        self.current_selected_subscription.borrow().clone()
    }

    fn get_state(&self, search_id: isize) -> Option<SubsMapEntry> {
        self.statemap.borrow().get_state(search_id)
    }

    fn clear_read_unread(&self, subs_id: isize) {
        self.statemap.borrow_mut().clear_num_all_unread(subs_id);
        self.addjob(SJob::ScanEmptyUnread);
    }

    fn memory_conserve(&mut self, act: bool) {
        self.currently_minimized = act;
    }

    fn move_to_other_subscription(&self, move_up: bool) {
        let mut subs_id: isize = -1;
        if let Some((subs_e, _nonfolder_child_ids)) =
            self.current_selected_subscription.borrow().as_ref()
        {
            subs_id = subs_e.subs_id;
        }
        if subs_id < 0 {
            return;
        }
        let mut relative_idx = -1;
        if let Some(st) = self.statemap.borrow().get_state(subs_id) {
            relative_idx = st.get_relative_idx();
        }
        let mut new_rel_ix = relative_idx;
        let mut o_path: Option<Vec<u16>> = None;
        if move_up {
            while new_rel_ix > 1 && o_path.is_none() {
                new_rel_ix -= 1;
                o_path = self.statemap.borrow().get_path_for_rel(new_rel_ix);
            }
        } else {
            while new_rel_ix < relative_idx + 30 && o_path.is_none() {
                new_rel_ix += 1;
                o_path = self.statemap.borrow().get_path_for_rel(new_rel_ix);
            }
        }
        if let Some(path) = o_path {
            // trace!(                " MOVE:  {:?}     up:{}  rel:{} p:{:?} =>  {}   path:{:?}  ",                subs_id,                move_up,                relative_idx,                oldpath,                new_rel_ix,                path,            );
            (*self.gui_updater)
                .borrow()
                .tree_set_cursor(TREEVIEW0, path);
        }
    }

    fn get_subs_icon_id(&self, subs_id: isize) -> usize {
        self.statemap.borrow().get_icon_id(subs_id)
    }
}
