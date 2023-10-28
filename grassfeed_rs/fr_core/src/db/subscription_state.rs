use resources::parameter::SCAN_EMPTY_UNREAD_GROUP;
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
    fn set_tree_path(&mut self, db_id: isize, newpath: Vec<u16>, is_folder: bool);

    fn set_status(&mut self, idlist: &[isize], statusflag: StatusMask, activated: bool);

    /// searches subscription_entry that has no unread,all  number set
    fn scan_num_all_unread(&self) -> Vec<(isize, bool)>;

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

    fn get_fetch_scheduled(&self) -> Vec<isize>; // Vec<(&isize, &SubsMapEntry)>;
}

#[derive(Default)]
pub struct SubscriptionState {
    statemap: HashMap<isize, SubsMapEntry>,
}

impl ISubscriptionState for SubscriptionState {
    fn set_deleted(&mut self, subs_id: isize, new_del: bool) {
        if let Some(st) = self.statemap.get_mut(&subs_id) {
            st.set_deleted(new_del);
        }
    }

    fn set_tree_path(&mut self, db_id: isize, newpath: Vec<u16>, is_folder: bool) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.statemap.entry(db_id) {
            let mut sme = SubsMapEntry {
                tree_path: Some(newpath),
                ..Default::default()
            };
            sme.set_folder(is_folder);
            e.insert(sme);
        } else if let Some(st) = self.statemap.get_mut(&db_id) {
            st.set_path(newpath);
        }
    }

    fn get_num_all_unread(&self, subs_id: isize) -> Option<(isize, isize)> {
        let st = self.statemap.get(&subs_id)?;
        st.num_msg_all_unread
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
        } else {
            debug!("clear_num_all_unread:  id {} not found! ", subs_id);
        }
    }

    /// don't include deleted ones, sort folders to the end
    /// returns    subs_id,  is_folder
    fn scan_num_all_unread(&self) -> Vec<(isize, bool)> {
        let mut unproc_ids: Vec<(isize, bool)> = self
            .statemap
            .iter()
            .filter_map(|(id, se)| {
                if se.num_msg_all_unread.is_none() && *id > 0 && !se.is_deleted() {
                    Some((*id, se.is_folder()))
                } else {
                    None
                }
            })
            .collect::<Vec<(isize, bool)>>();
        unproc_ids.sort_by(|a, b| a.1.cmp(&b.1));
        if unproc_ids.len() > SCAN_EMPTY_UNREAD_GROUP as usize {
            unproc_ids.truncate(SCAN_EMPTY_UNREAD_GROUP as usize)
        }
        // trace!(            "scan_num_all_unread:  unproc: {:?}  KEYS={:?}",           unproc_ids,            self.statemap.keys()        );
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
            .filter(|(_id, entry)| include_folder || !entry.is_folder())
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
            .filter_map(|(id, st)| st.tree_path.as_ref().map(|tp| (id, tp)))
            .find_map(|(id, tp)| if tp == path { Some(*id) } else { None })
    }

    fn set_schedule_fetch_all(&mut self) {
        self.statemap
            .iter_mut()
            .filter(|(_id, entry)| !entry.is_folder() && !entry.is_deleted())
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
    }

    fn get_fetch_scheduled(&self) -> Vec<isize> {
        self.statemap
            .iter()
            .filter(|(_id, entry)| !entry.is_folder())
            .filter(|(_id, entry)| !entry.is_deleted())
            .filter(|(_id, entry)| entry.check_bitmask(StatusMask::FetchScheduled as usize))
            .filter(|(_id, entry)| !entry.is_fetch_scheduled_jobcreated())
            .map(|(id, _entry)| *id)
            .collect::<Vec<isize>>()
    }
}

#[allow(dead_code)]
pub enum StatusMask {
    // Dirty = 1,
    FetchScheduled = 8,
    FetchScheduledJobCreated = 16,
    FetchInProgress = 32,
    ErrFetchReq = 64,
    ErrIconReq = 128,
    IsFolderCopy = 256,
    IsDeletedCopy = 512,
    IsExpandedCopy = 1024,
    MessageCountsChecked = 2048,
}

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
pub struct SubsMapEntry {
    pub tree_path: Option<Vec<u16>>,
    pub status: usize,
    pub num_msg_all_unread: Option<(isize, isize)>,
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

    fn is_folder(&self) -> bool;
    fn set_folder(&mut self, n: bool);

    fn is_messagecounts_checked(&self) -> bool;
    fn set_messagecounts_checked(&mut self, n: bool);
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
        self.check_bitmask(StatusMask::IsDeletedCopy as usize)
    }
    fn set_deleted(&mut self, n: bool) {
        self.change_bitmask(StatusMask::IsDeletedCopy as usize, n)
    }

    fn is_expanded(&self) -> bool {
        self.check_bitmask(StatusMask::IsExpandedCopy as usize)
    }
    fn set_expanded(&mut self, n: bool) {
        self.change_bitmask(StatusMask::IsExpandedCopy as usize, n);
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
        }
    }

    fn set_path(&mut self, newpath: Vec<u16>) {
        self.tree_path = Some(newpath);
    }
    fn get_path(&self) -> Option<Vec<u16>> {
        self.tree_path.clone()
    }

    fn is_folder(&self) -> bool {
        self.check_bitmask(StatusMask::IsFolderCopy as usize)
    }

    fn set_folder(&mut self, n: bool) {
        self.change_bitmask(StatusMask::IsFolderCopy as usize, n);
    }

    fn is_messagecounts_checked(&self) -> bool {
        self.check_bitmask(StatusMask::MessageCountsChecked as usize)
    }

    fn set_messagecounts_checked(&mut self, n: bool) {
        self.change_bitmask(StatusMask::MessageCountsChecked as usize, n)
    }
}

#[cfg(test)]
pub mod t {
    use super::*;

    //cargo watch -s "cargo test db::subscription_state::t::t_scan_num_all_unread  --lib -- --exact --nocapture"
    #[test]
    fn t_scan_num_all_unread() {
        let lim: isize = SCAN_EMPTY_UNREAD_GROUP as isize;
        let mut ss = SubscriptionState::default();
        let mut sme = SubsMapEntry {
            status: StatusMask::IsFolderCopy as usize,
            ..Default::default()
        };
        for a in 0..lim {
            ss.statemap.insert(a + 1, sme.clone());
        }
        sme.status = 0;
        ss.statemap.insert(lim + 3, sme.clone());
        let r = ss.scan_num_all_unread();
        assert_eq!(r.len(), lim as usize);
        println!("scan: {:?}", r);
        assert!(r.contains(&(lim + 3, false)));
    }
}
