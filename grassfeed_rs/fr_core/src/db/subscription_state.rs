use std::collections::HashMap;

pub trait ISubscriptionState {
    fn get_state(&self, id: isize) -> Option<SubsMapEntry>;
    fn get_id_by_path(&self, path: &[u16]) -> Option<isize>;

    fn set_schedule_fetch_all(&mut self);

    fn get_ids_by_status(
        &self,
        statusflag: StatusMask,
        activated: bool,
        include_folder: bool,
    ) -> Vec<isize>;

    fn get_tree_path(&self, db_id: isize) -> Option<Vec<u16>>;
    fn set_tree_path(&mut self, db_id: isize, newpath: Vec<u16>);

    fn set_status(&mut self, idlist: &[isize], statusflag: StatusMask, activated: bool);

    /// searches subscription_entry that has no unread,all  number set
    fn scan_num_all_unread(&self) -> Vec<isize>;

    fn clear_num_all_unread(&mut self, subs_id: isize);

    /// returns the modified entry
    fn set_num_all_unread(
        &mut self,
        subs_id: isize,
        num_all: isize,
        num_unread: isize,
    ) -> Option<SubsMapEntry>;

    fn get_num_all_unread(&self, subs_id: isize) -> Option<(isize, isize)>;

    fn set_deleted(&mut self, subs_id: isize, new_del: bool);

    fn get_length(&self) -> usize;

    fn dump(&self);
}

#[derive(Default)]
pub struct SubscriptionState {
    statemap: HashMap<isize, SubsMapEntry>,
}

impl ISubscriptionState for SubscriptionState {
    fn set_deleted(&mut self, subs_id: isize, new_del: bool) {
        if let Some(mut st) = self.statemap.get_mut(&subs_id) {
            st.is_deleted = new_del;
        }
    }

    fn set_tree_path(&mut self, db_id: isize, newpath: Vec<u16>) {
        /*
                if !self.statemap.contains_key(&db_id) {
                    let mut sme = SubsMapEntry::default();
                    sme.tree_path = Some(newpath);
                    self.statemap.insert(db_id, sme);
                } else if let Some(st) = self.statemap.get_mut(&db_id) {
                    st.set_path(newpath);
                }
        */
        if let std::collections::hash_map::Entry::Vacant(e) = self.statemap.entry(db_id) {
            let sme = SubsMapEntry {
                tree_path: Some(newpath),
                ..Default::default()
            };
            e.insert(sme);
        } else if let Some(st) = self.statemap.get_mut(&db_id) {
            st.set_path(newpath);
        }
    }

    fn get_num_all_unread(&self, subs_id: isize) -> Option<(isize, isize)> {
        if let Some(st) = self.statemap.get(&subs_id) {
            return st.num_msg_all_unread;
        }
        None
    }

    fn set_num_all_unread(
        &mut self,
        subs_id: isize,
        num_all: isize,
        num_unread: isize,
    ) -> Option<SubsMapEntry> {
        if let Some(entry) = self.statemap.get_mut(&subs_id) {
            entry.num_msg_all_unread = Some((num_all, num_unread));
            return Some(entry.clone());
        } else {
            debug!("set_num_all_unread({})  ID not found", subs_id);
        }
        None
    }

    fn clear_num_all_unread(&mut self, subs_id: isize) {
        if let Some(entry) = self.statemap.get_mut(&subs_id) {
            entry.num_msg_all_unread = None;
        }
    }

    /// don't include deleted ones, no folders,
    /// Usability+Speed:  dispatch 2 subscriptions at one time for re-calculating
    fn scan_num_all_unread(&self) -> Vec<isize> {
        let unproc_ids: Vec<isize> = self
            .statemap
            .iter()
            .filter_map(|(id, se)| {
                if !se.is_folder && se.num_msg_all_unread.is_none() && *id > 0 && !se.is_deleted() {
                    Some(*id)
                } else {
                    None
                }
            })
            .take(2)
            .collect::<Vec<isize>>();
        unproc_ids
    }

    fn get_state(&self, search_id: isize) -> Option<SubsMapEntry> {
        self.statemap.iter().find_map(|(id, entry)| {
            if *id == search_id {
                Some((*entry).clone())
            } else {
                None
            }
        })
    }

    fn set_status(&mut self, idlist: &[isize], statusflag: StatusMask, activated: bool) {
        let mask = statusflag as usize;
        self.statemap
            .iter_mut()
            .filter(|(id, _entry)| idlist.contains(*id))
            .for_each(|(_id, entry)| entry.change_bitmask(mask, activated));
    }

    fn get_tree_path(&self, db_id: isize) -> Option<Vec<u16>> {
        if let Some(entry) = self.statemap.get(&db_id) {
            if let Some(p) = &entry.tree_path {
                return Some(p.clone());
            }
        }
        None
    }

    fn get_ids_by_status(
        &self,
        statusflag: StatusMask,
        activated: bool,
        include_folder: bool,
    ) -> Vec<isize> {
        let mask = statusflag as usize;
        self.statemap
            .iter()
            .filter(|(_id, entry)| include_folder || !entry.is_folder)
            .filter_map(|(id, entry)| {
                if entry.check_bitmask(mask) == activated {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect::<Vec<isize>>()
    }

    fn get_id_by_path(&self, path: &[u16]) -> Option<isize> {
        self.statemap
            .iter()
            .filter_map(|(id, st)| {
                // if let Some(tp) = &st.tree_path {
                //     Some((id, tp))
                // } else {
                //     None
                // }
                st.tree_path.as_ref().map(|tp| (id, tp))
            })
            .find_map(|(id, tp)| if tp == path { Some(*id) } else { None })
    }

    fn set_schedule_fetch_all(&mut self) {
        self.statemap
            .iter_mut()
            .filter(|(_id, entry)| !entry.is_folder && !entry.is_deleted)
            .for_each(|(_id, entry)| {
                entry.set_fetch_scheduled(true);
            });
    }

    fn get_length(&self) -> usize {
        self.statemap.len()
    }

    fn dump(&self) {
        self.statemap
            .iter()
            .for_each(|(k, v)| debug!("SSD {} {:?}", k, v));
        // debug!("subscription_state::dump() {:#?}", self.statemap);
    }
}

#[allow(dead_code)]
pub enum StatusMask {
    Dirty = 1,
    FetchScheduled = 8,
    FetchScheduledJobCreated = 16,
    FetchInProgress = 32,
    ErrFetchReq = 64,
    ErrIconReq = 128,
    //    FolderExpanded = 64,
    //    Deleted = 128,
}

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
pub struct SubsMapEntry {
    pub tree_path: Option<Vec<u16>>,
    pub status: usize,
    pub num_msg_all_unread: Option<(isize, isize)>,

    /// later: remove this
    pub is_dirty: bool,

    /// copied
    pub is_folder: bool,
    /// copied
    pub is_deleted: bool,
    /// copied
    pub is_expanded: bool,
}

pub trait FeedSourceState {
    fn is_expanded(&self) -> bool;
    fn set_expanded(&mut self, new_exp: bool);

    fn is_err_on_fetch(&self) -> bool;
    fn set_err_on_fetch(&mut self, n: bool);

    fn is_fetch_scheduled(&self) -> bool;
    fn set_fetch_scheduled(&mut self, n: bool);

    fn is_fetch_scheduled_jobcreated(&self) -> bool;
    fn set_fetch_scheduled_jobcreated(&mut self, n: bool);

    fn is_fetch_in_progress(&self) -> bool;
    fn set_fetch_in_progress(&mut self, n: bool);

    fn is_deleted(&self) -> bool;
    fn set_deleted(&mut self, n: bool);

    fn check_bitmask(&self, bitmask: usize) -> bool;
    fn change_bitmask(&mut self, bitmask: usize, new_state: bool);

    fn set_path(&mut self, newpath: Vec<u16>);
    fn get_path(&self) -> Option<Vec<u16>>;
}

#[allow(dead_code)]
impl FeedSourceState for SubsMapEntry {
    fn is_err_on_fetch(&self) -> bool {
        self.check_bitmask(StatusMask::ErrFetchReq as usize)
    }

    fn set_err_on_fetch(&mut self, n: bool) {
        self.change_bitmask(StatusMask::ErrFetchReq as usize, n)
    }

    fn is_fetch_scheduled(&self) -> bool {
        self.check_bitmask(StatusMask::FetchScheduled as usize)
    }
    fn set_fetch_scheduled(&mut self, n: bool) {
        self.change_bitmask(StatusMask::FetchScheduled as usize, n)
    }

    fn is_fetch_scheduled_jobcreated(&self) -> bool {
        self.check_bitmask(StatusMask::FetchScheduledJobCreated as usize)
    }
    fn set_fetch_scheduled_jobcreated(&mut self, n: bool) {
        self.change_bitmask(StatusMask::FetchScheduledJobCreated as usize, n)
    }

    fn is_fetch_in_progress(&self) -> bool {
        self.check_bitmask(StatusMask::FetchInProgress as usize)
    }

    fn set_fetch_in_progress(&mut self, n: bool) {
        self.change_bitmask(StatusMask::FetchInProgress as usize, n)
    }

    fn is_deleted(&self) -> bool {
        self.is_deleted
    }
    fn set_deleted(&mut self, n: bool) {
        self.is_deleted = n
    }

    fn is_expanded(&self) -> bool {
        self.is_expanded
    }
    fn set_expanded(&mut self, new_exp: bool) {
        self.is_expanded = new_exp;
    }

    fn check_bitmask(&self, bitmask: usize) -> bool {
        self.status & bitmask == bitmask
    }

    fn change_bitmask(&mut self, bitmask: usize, new_state: bool) {
        let new_st = match new_state {
            true => self.status | bitmask,
            false => self.status & !bitmask,
        };
        if new_st != self.status {
            self.status = new_st;
            self.is_dirty = true;
        }
    }

    fn set_path(&mut self, newpath: Vec<u16>) {
        self.tree_path = Some(newpath);
    }
    fn get_path(&self) -> Option<Vec<u16>> {
        self.tree_path.clone()
    }
}
