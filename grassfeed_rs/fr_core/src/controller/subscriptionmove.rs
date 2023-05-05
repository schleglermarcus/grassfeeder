use crate::controller::isourcetree::ISourceTreeController;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::db::errors_repo::ErrorRepo;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_DELETED;
use crate::db::subscription_entry::SRC_REPO_ID_MOVING;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::SubscriptionState;
use crate::opml::opmlreader::OpmlReader;
use crate::util::filter_by_iso8859_1;
use crate::util::remove_invalid_chars_from_input;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::rc::Weak;

pub trait ISubscriptionMove {
    fn on_subscription_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool;

    fn get_state_map(&self) -> Rc<RefCell<SubscriptionState>>;
    fn update_cached_paths(&self);

    fn set_delete_subscription_id(&mut self, o_fs_id: Option<usize>);
    fn move_subscription_to_trash(&mut self);

    /// using internal state for parent id
    fn add_new_folder(&mut self, folder_name: String) -> isize;
    fn add_new_folder_at_parent(&self, folder_name: String, parent_id: isize) -> isize;
    fn add_new_subscription(&mut self, newsource: String, display: String) -> isize;
    fn add_new_subscription_at_parent(
        &self,
        newsource: String,
        display: String,
        parent_id: isize,
        load_messages: bool,
    ) -> isize;

    fn import_opml(&self, filename: String);
    fn empty_create_default_subscriptions(&mut self);

    fn set_fs_delete_id(&mut self, o_fs_id: Option<usize>);
    fn feedsource_delete(&mut self);

    fn request_check_paths(&self, needs_check: bool);
    fn check_paths(&self);
}

pub struct SubscriptionMove {
    subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    pub messagesrepo_r: Rc<RefCell<dyn IMessagesRepo>>,
    feedsources_w: Weak<RefCell<SourceTreeController>>,
    erro_repo_r: Rc<RefCell<ErrorRepo>>,

    statemap: Rc<RefCell<SubscriptionState>>,
    need_check_fs_paths: RefCell<bool>,
    feedsource_delete_id: Option<usize>,
    current_new_folder_parent_id: Option<isize>,
}

impl SubscriptionMove {
    pub fn new_ac(ac: &AppContext) -> Self {
        Self::new(
            (*ac).get_rc::<SubscriptionRepo>().unwrap(),
            (*ac).get_rc::<MessagesRepo>().unwrap(),
            (*ac).get_rc::<ErrorRepo>().unwrap(),
        )
    }

    pub fn new(
        subs_repo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
        msg_repo_r: Rc<RefCell<dyn IMessagesRepo>>,
        err_rep: Rc<RefCell<ErrorRepo>>,
    ) -> Self {
        let statemap_ = Rc::new(RefCell::new(SubscriptionState::default()));
        SubscriptionMove {
            subscriptionrepo_r: subs_repo_r,
            messagesrepo_r: msg_repo_r,
            feedsources_w: Weak::new(),
            erro_repo_r: err_rep,
            statemap: statemap_,
            need_check_fs_paths: RefCell::new(true),
            feedsource_delete_id: Default::default(),
            current_new_folder_parent_id: Default::default(),
        }
    }

    /// returns:   From-Entry,   To-Parent-ID,  to-folderpos
    ///
    /// When dragging on a folder, we get a sub-sub-Path  from gtk
    ///
    /// Mouse-Drag  to [0]  creates a drag-event  to [0, 0]
    /// Mouse-Drag  to [1]  creates a drag-event  to [1, 0]
    /// Mouse-Drag  under [0]  creates a drag-event  to [1]
    ///
    pub fn drag_calc_positions(
        &self,
        from_path: &[u16],
        to_path: &[u16],
    ) -> Result<(SubscriptionEntry, isize, isize), String> {
        let o_from_entry = self.get_by_path(from_path);
        if o_from_entry.is_none() {
            self.need_check_fs_paths.replace(true);
            let msg = format!("from_path={from_path:?}  Missing, check statemap");
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

    /// scans the messages for highest subscription id, if there is a higher one, use next higher subscription id
    /// returns 0     to use   autoincrement
    pub fn get_next_available_subscription_id(&self) -> isize {
        let subs_repo_highest = (*self.subscriptionrepo_r).borrow().get_highest_src_id();
        let mut next_subs_id = std::cmp::max(subs_repo_highest + 1, 10);
        let h = (*self.messagesrepo_r).borrow().get_max_src_index();
        if h >= next_subs_id {
            next_subs_id = h + 1;
        } else {
            next_subs_id = 0; // default auto increment
        }
        next_subs_id
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

    pub fn addjob(&self, j: SJob) {
        if let Some(subs_w) = self.feedsources_w.upgrade() {
            (*subs_w).borrow().addjob(j);
        }
    }

    #[allow(dead_code)]
    fn get_siblings_ids(&self, f_path: &Vec<u16>) -> Vec<isize> {
        let mut parent_path = f_path.clone();
        if parent_path.len() > 0 {
            parent_path.pop();
        }
        let mut child_ids: Vec<isize> = Vec::default();
        let o_parent_id = self.statemap.borrow().get_id_by_path(&parent_path);
        if o_parent_id.is_none() {
            return child_ids;
        }
        let parent_id = o_parent_id.unwrap();
        // if let Some(p_sub) = self.get_by_path(&parent_path) {
        child_ids = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_parent_repo_id(parent_id)
            .iter()
            .map(|fse| fse.subs_id)
            .collect::<Vec<isize>>();
        // }
        debug!(" children:  {:?} ", &child_ids);
        child_ids
    }
} // impl SubscriptionMove

impl ISubscriptionMove for SubscriptionMove {
    fn on_subscription_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool {
        trace!("START_DRAG {:?} => {:?}      ", &from_path, &to_path);
        let all1 = (*self.subscriptionrepo_r).borrow().get_all_entries();
        let length_before = all1.len();
        let mut success: bool = false;

        let mut update_tree_ids: HashSet<isize> = HashSet::default();

        match self.drag_calc_positions(&from_path, &to_path) {
            Ok((from_entry, to_parent_id, to_folderpos)) => {
                let from_parent_id = from_entry.parent_subs_id;
                self.drag_move(from_entry, to_parent_id, to_folderpos);
                let all2 = (*self.subscriptionrepo_r).borrow().get_all_entries();
                if all2.len() != length_before {
                    error!("Drag lost entries: {}->{}", length_before, all2.len());
                    success = false;
                } else {
                    success = true;
                }
                update_tree_ids.insert(from_parent_id);
                if to_parent_id > 0 {
                    update_tree_ids.insert(to_parent_id);
                }

                if let Some(subs_w) = self.feedsources_w.upgrade() {
                    for id in update_tree_ids {
                        (*subs_w).borrow().addjob(SJob::GuiUpdateTree(id));
                    }
                }
            }
            Err(msg) => {
                warn!("DragFail: {:?}=>{:?} --> {} ", from_path, to_path, msg);
                //  (*self.subscriptionrepo_r)                    .borrow()                    .debug_dump_tree("DragFail");

                let mut from_path_parent = from_path.clone();
                if from_path_parent.len() > 0 {
                    from_path_parent.pop();
                };
                let mut to_path_parent = to_path.clone();
                if to_path_parent.len() > 0 {
                    to_path_parent.pop();
                };

                debug!(
                    "FROM_parent={:?}  TO_parent={:?}",
                    from_path_parent, from_path_parent
                );
                if let Some(subs_w) = self.feedsources_w.upgrade() {
                    (*subs_w)
                        .borrow()
                        .addjob(SJob::GuiUpdateTreePartial(from_path_parent.clone()));

                    if to_path_parent != from_path_parent {
                        (*subs_w)
                            .borrow()
                            .addjob(SJob::GuiUpdateTreePartial(to_path_parent));
                    }
                }
                /*
                               for id in self.get_siblings_ids(&from_path) {
                                   update_tree_ids.insert(id);
                               }
                               for id in self.get_siblings_ids(&to_path) {
                                   update_tree_ids.insert(id);
                               }

                               debug!("UPD_IDS= {:?} ", &update_tree_ids);
                               if let Some(subs_w) = self.feedsources_w.upgrade() {
                                   (*subs_w).borrow().addjob(SJob::UpdateTreePaths);
                                   (*subs_w).borrow().addjob(SJob::FillSubscriptionsAdapter);
                                   for id in update_tree_ids {
                                       if let Some(st) = self.statemap.borrow().get_state(id) {
                                           if let Some(path) = st.tree_path {
                                               (*subs_w).borrow().addjob(SJob::GuiUpdateTreePartial(path));
                                           }
                                       }
                                   }
                               }
                */
            }
        }

        success
    }

    fn get_state_map(&self) -> Rc<RefCell<SubscriptionState>> {
        self.statemap.clone()
    }

    fn move_subscription_to_trash(&mut self) {
        if self.feedsource_delete_id.is_none() {
            return;
        }
        let fs_id = self.feedsource_delete_id.unwrap();
        let fse: SubscriptionEntry = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(fs_id as isize)
            .unwrap();
        (*self.subscriptionrepo_r)
            .borrow()
            .update_parent_and_folder_position(fse.subs_id, SRC_REPO_ID_DELETED, 0);
        (*self.subscriptionrepo_r)
            .borrow()
            .set_deleted_rec(fse.subs_id);
        self.resort_parent_list(fse.parent_subs_id);
        self.feedsource_delete_id = None;
        if let Some(subs_w) = self.feedsources_w.upgrade() {
            (*subs_w).borrow().addjob(SJob::UpdateTreePaths);
            (*subs_w).borrow().addjob(SJob::FillSubscriptionsAdapter);
            (*subs_w).borrow().addjob(SJob::GuiUpdateTreeAll);
        }
    }

    fn set_delete_subscription_id(&mut self, o_fs_id: Option<usize>) {
        self.feedsource_delete_id = o_fs_id;
    }

    // moving
    /// returns  source_repo_id
    fn add_new_folder(&mut self, folder_name: String) -> isize {
        let mut new_parent_id = 0;
        if self.current_new_folder_parent_id.is_some() {
            new_parent_id = self.current_new_folder_parent_id.take().unwrap();
        }
        self.add_new_folder_at_parent(folder_name, new_parent_id)
    }

    // moving
    fn add_new_folder_at_parent(&self, folder_name: String, parent_id: isize) -> isize {
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
                if let Some(subs_w) = self.feedsources_w.upgrade() {
                    (*subs_w).borrow().addjob(SJob::UpdateTreePaths);
                    (*subs_w).borrow().addjob(SJob::FillSubscriptionsAdapter);
                    (*subs_w).borrow().addjob(SJob::GuiUpdateTreeAll);
                    (*subs_w)
                        .borrow()
                        .addjob(SJob::SetCursorToSubsID(fse.subs_id));
                }
                fse.subs_id
            }
            Err(e2) => {
                error!("add_new_folder: {:?}", e2);
                -1
            }
        }
    }

    fn update_cached_paths(&self) {
        self.update_paths_rec(&Vec::<u16>::default(), 0, false);
    }

    fn add_new_subscription(&mut self, newsource: String, display: String) -> isize {
        let p_id = self.current_new_folder_parent_id.unwrap_or(0);
        self.add_new_subscription_at_parent(newsource, display, p_id, false)
    }

    fn add_new_subscription_at_parent(
        &self,
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
                self.addjob(SJob::FillSubscriptionsAdapter);
                self.addjob(SJob::GuiUpdateTreeAll);
                self.addjob(SJob::SetCursorToSubsID(fse2.subs_id));
                if load_messages {
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

    // moving
    fn import_opml(&self, filename: String) {
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
        self.addjob(SJob::GuiUpdateTreeAll);
    }

    fn empty_create_default_subscriptions(&mut self) {
        let before = (*self.subscriptionrepo_r).borrow().db_existed_before();
        if before {
            return;
        }
        let url_names: [(&str, &str); 3] = [
            ("https://rss.slashdot.org/Slashdot/slashdot", "Slashdot"),
            ("https://www.reddit.com/r/aww.rss", "Reddit - Aww"),
            ("https://xkcd.com/atom.xml", "XKCD"),
        ];
        let folder1 = self.add_new_folder_at_parent(t!("SUBSC_DEFAULT_FOLDER1"), 0);
        url_names.iter().for_each(|(u, n)| {
            self.add_new_subscription_at_parent(u.to_string(), n.to_string(), folder1, true);
        });
        let url_names: [(&str, &str); 5] = [
            ("https://blog.linuxmint.com/?feed=rss2", "Linux Mint"),
            ("http://blog.rust-lang.org/feed.xml", "Rust Language"),
            ("https://rss.golem.de/rss.php?feed=ATOM1.0", "Golem.de"),
            ("https://www.heise.de/rss/heise-atom.xml", "Heise.de"),
            (
                "https://github.com/schleglermarcus/grassfeeder/releases.atom",
                "Grassfeeder Releases",
            ),
        ];
        let folder2 = self.add_new_folder_at_parent(t!("SUBSC_DEFAULT_FOLDER2"), 0);
        url_names.iter().for_each(|(u, n)| {
            self.add_new_subscription_at_parent(u.to_string(), n.to_string(), folder2, true);
        });
    }

    fn set_fs_delete_id(&mut self, o_fs_id: Option<usize>) {
        self.feedsource_delete_id = o_fs_id;
    }

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
        self.addjob(SJob::GuiUpdateTreeAll);
        self.feedsource_delete_id = None;
    }

    fn request_check_paths(&self, needs_check: bool) {
        self.need_check_fs_paths.replace(needs_check);
    }

    fn check_paths(&self) {
        if *self.need_check_fs_paths.borrow() {
            self.update_cached_paths();
            self.need_check_fs_paths.replace(false);
        }
    }
} //  impl ISubscriptionMove

impl Buildable for SubscriptionMove {
    type Output = SubscriptionMove;
    fn build(_conf: Box<dyn BuildConfig>, ac: &AppContext) -> Self::Output {
        SubscriptionMove::new_ac(ac)
    }
}

impl StartupWithAppContext for SubscriptionMove {
    fn startup(&mut self, ac: &AppContext) {
        self.feedsources_w = Rc::downgrade(&(*ac).get_rc::<SourceTreeController>().unwrap());
    }
}
