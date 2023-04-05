use crate::controller::isourcetree::ISourceTreeController;
use crate::controller::sourcetree::Config;
use crate::controller::sourcetree::NewSourceState;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::controller::sourcetree::JOBQUEUE_SIZE;
use crate::db::subscription_entry;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_MOVING;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::SubscriptionState;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

pub trait ISubscriptionMove {
    fn on_subscription_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool;

    fn get_state_map(&self) -> Rc<RefCell<SubscriptionState>>;
    //
}

pub struct SubscriptionMove {
    subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    feedsources_w: Weak<RefCell<SourceTreeController>>,

    statemap: Rc<RefCell<SubscriptionState>>,
    need_check_fs_paths: RefCell<bool>,
}

impl SubscriptionMove {
    pub fn new_ac(ac: &AppContext) -> Self {
        // let gc_r = (*ac).get_rc::<GuiContext>().unwrap();
        // let u_a = (*gc_r).borrow().get_updater_adapter();
        // let v_s_a = (*gc_r).borrow().get_values_adapter();
        // let dl_r = (*ac).get_rc::<contentdownloader::Downloader>().unwrap();
        // let err_rep = (*ac).get_rc::<ErrorRepo>().unwrap();
        // (*ac).get_rc::<Timer>().unwrap(),
        Self::new((*ac).get_rc::<SubscriptionRepo>().unwrap())
    }

    pub fn new(subs_repo_r: Rc<RefCell<dyn ISubscriptionRepo>>) -> Self {
        let statemap_ = Rc::new(RefCell::new(SubscriptionState::default()));
        SubscriptionMove {
            subscriptionrepo_r: subs_repo_r,
            statemap: statemap_,
            feedsources_w: Weak::new(),
            need_check_fs_paths: RefCell::new(true),
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
} // impl SubscriptionMove

impl ISubscriptionMove for SubscriptionMove {
    fn on_subscription_drag(&self, _tree_nr: u8, from_path: Vec<u16>, to_path: Vec<u16>) -> bool {
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
        if let Some(subs_w) = self.feedsources_w.upgrade() {
            (*subs_w).borrow().addjob(SJob::UpdateTreePaths);
            (*subs_w).borrow().addjob(SJob::FillSubscriptionsAdapter);
            if from_parent_id >= 0 {
                (*subs_w)
                    .borrow()
                    .addjob(SJob::GuiUpdateTree(from_parent_id));
            }
            if to_parent_subs_id >= 0 && to_parent_subs_id != from_parent_id {
                (*subs_w)
                    .borrow()
                    .addjob(SJob::GuiUpdateTree(to_parent_subs_id));
            }
        }

        success
    }

    fn get_state_map(&self) -> Rc<RefCell<SubscriptionState>> {
        self.statemap.clone()
    }
}

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
