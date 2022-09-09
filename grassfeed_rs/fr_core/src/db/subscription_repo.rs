use crate::db::message::compress;
use crate::db::message::decompress;
use crate::db::sqlite_context::SqliteContext;
use crate::db::sqlite_context::TableInfo;
use crate::db::subscription_entry::FeedSourceState;
use crate::db::subscription_entry::StatusMask;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_DELETED;
use crate::db::subscription_entry::SRC_REPO_ID_DUMMY;
use crate::db::subscription_entry::SRC_REPO_ID_MOVING;
use crate::timer::Timer;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use rusqlite::Connection;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::BufWriter;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

pub const KEY_FOLDERNAME: &str = "subscriptions_folder";

/// sanity for recursion
/// TODO use for update_paths
//  const MAX_PATH_DEPTH: usize = 30;

// pub const FILENAME_TXT: &str = "subscription_list_json.txt";

pub const FILENAME_JSON: &str = "subscription_list.json";

pub const CONV_TO: &dyn Fn(String) -> Option<SubscriptionEntry> = &json_to_subscription_entry;
pub const CONV_FROM: &dyn Fn(&SubscriptionEntry) -> Option<String> = &subscription_entry_to_json;

// TODO  register at timer for shutdown,  do flush!
pub trait ISubscriptionRepo {
    /// sorts by folder_position
    fn get_by_parent_repo_id(&self, parent_subs_id: isize) -> Vec<SubscriptionEntry>;

    /// get by parent_subs_id  and folder_position
    fn get_by_pri_fp(&self, parent_subs_id: isize, folder_pos: isize) -> Vec<SubscriptionEntry>;

    /// sorts by folder_position
    fn get_all_nonfolder(&self) -> Vec<SubscriptionEntry>;

    /// checks for  updated_int,  retrieves those earlier than the given date
    fn get_by_fetch_time(&self, updated_time_s: i64) -> Vec<SubscriptionEntry>;

    fn get_by_index(&self, indexvalue: isize) -> Option<SubscriptionEntry>;

    fn get_all_entries(&self) -> Vec<SubscriptionEntry>;

    #[deprecated(note = "use ctx.get_all()")]
    fn get_list(&self) -> Arc<RwLock<HashMap<isize, SubscriptionEntry>>>;

    /// if subs_id == 0  stores at next possible higher  subs_id.
    /// if subs_id is given, we store that.
    fn store_entry(
        &self,
        entry: &SubscriptionEntry,
    ) -> Result<SubscriptionEntry, Box<dyn std::error::Error>>;

    ///   store IconID into feed source
    fn update_icon_id(&self, src_id: isize, icon_id: usize, timestamp_s: i64);

    fn update_folder_position(&self, src_id: isize, new_folder_pos: isize);

    fn update_expanded(&self, src_ids: Vec<isize>, new_expanded: bool);

    fn update_parent_and_folder_position(
        &self,
        src_id: isize,
        new_parent_id: isize,
        new_folder_pos: isize,
    );

    fn update_displayname(&self, src_id: isize, new_name: String);
    fn update_url(&self, src_id: isize, new_url: String);

    /// change  the updated_int, updated_ext  of suscription subscription_entry
    fn update_timestamps(&self, src_id: isize, updated_int: i64, updated_ext: Option<i64>);

    fn update_last_selected(&self, src_id: isize, content_id: isize);
    fn update_deleted(&self, src_id: isize, is_del: bool);

    fn delete_by_index(&self, del_index: isize);

    fn clear(&self);

    fn debug_dump_tree(&self, ident: &str);

    #[deprecated(note = "into statemap ")]
    fn set_schedule_fetch_all(&self);
    #[deprecated(note = "into statemap ")]
    fn get_ids_by_status(
        &self,
        statusflag: StatusMask,
        activated: bool,
        include_folder: bool,
    ) -> Vec<isize>;
    #[deprecated(note = "into statemap ")]
    fn get_tree_path(&self, db_id: isize) -> Option<Vec<u16>>;
    #[deprecated(note = "into statemap ")]
    fn get_by_path(&self, path: &[u16]) -> Option<SubscriptionEntry>;
    #[deprecated(note = "into statemap ")]
    fn set_status(&self, idlist: &[isize], statusflag: StatusMask, activated: bool);

    /// writes the path array into the cached subscription list
    #[deprecated(note = "into statemap ")]
    fn update_cached_paths(&self);

    #[deprecated(note = "into statemap ")]
    fn clear_num_all_unread(&self, subs_id: isize);

    /// returns the modified entry
    #[deprecated(note = "into statemap ")]
    fn set_num_all_unread(
        &self,
        subs_id: isize,
        num_all: isize,
        num_unread: isize,
    ) -> Option<SubscriptionEntry>;

    #[deprecated(note = "into statemap ")]
    fn get_num_all_unread(&self, subs_id: isize) -> Option<(isize, isize)>;

    /// searches subscription_entry that has no unread,all  number set
    #[deprecated(note = "into statemap ")]
    fn scan_num_all_unread(&self) -> Option<isize>;

    fn get_highest_src_id(&self) -> isize;

    ///  put the topmost entry to deleted-parent,  set the deleted flag on all entries below
    fn set_deleted_rec(&self, del_index: isize);

    fn store_default_db_entries(&self);
}

pub struct SubscriptionRepo {
    folder_name: String,

    ctx: SqliteContext<SubscriptionEntry>,

    pub migr_read_from_json: bool,

    ///  ID -> Entry
    // #[deprecated] // later
    list: Arc<RwLock<HashMap<isize, SubscriptionEntry>>>,
    // 	#[deprecated]	// later
    list_cardinality_last: usize,
}

impl SubscriptionRepo {
    // #[deprecated]	// later
    pub fn new(folder_name: &str) -> Self {
        SubscriptionRepo {
            list: Arc::new(RwLock::new(HashMap::new())),
            folder_name: folder_name.to_string(),
            list_cardinality_last: 0,
            ctx: SqliteContext::new_in_memory(),
            migr_read_from_json: true,
        }
    }

    pub fn new2(folder_name: String) -> Self {
        let filename = format!("{}subscriptions.db", folder_name);
        SubscriptionRepo {
            folder_name: folder_name.to_string(),
            ctx: SqliteContext::new(filename),
            list: Arc::new(RwLock::new(HashMap::new())),
            list_cardinality_last: 0,
            migr_read_from_json: false,
        }
    }

    pub fn by_existing_list(existing: Arc<RwLock<HashMap<isize, SubscriptionEntry>>>) -> Self {
        SubscriptionRepo {
            list: existing,
            folder_name: String::default(),
            list_cardinality_last: 0,
            ctx: SqliteContext::new_in_memory(),
            migr_read_from_json: true,
        }
    }

    pub fn by_existing_connection(con: Arc<Mutex<Connection>>) -> Self {
        SubscriptionRepo {
            list: Arc::new(RwLock::new(HashMap::new())),
            folder_name: String::default(),
            list_cardinality_last: 0,
            ctx: SqliteContext::new_by_connection(con),
            migr_read_from_json: false,
        }
    }

    pub fn new_inmem() -> Self {
        SubscriptionRepo {
            folder_name: String::default(),
            ctx: SqliteContext::new_in_memory(),
            list: Arc::new(RwLock::new(HashMap::new())),
            list_cardinality_last: 0,
            migr_read_from_json: false,
        }
    }

    pub fn startup(&mut self) -> bool {
        match std::fs::create_dir_all(&self.folder_name) {
            Ok(()) => (),
            Err(e) => {
                error!(
                    "SubscriptionRepo cannot create folder {} {:?}",
                    &self.folder_name, e
                );
                return false;
            }
        }
        if self.migr_read_from_json {
            self.load_subscriptions_pretty()
        } else {
            self.ctx.create_table();
            self.store_default_db_entries();

            true
        }
        //		debug!("subs_r : startup - do nothing");
    }

    pub fn load_subscriptions_pretty(&mut self) -> bool {
        let file_name = format!("{}/{}", self.folder_name, FILENAME_JSON);
        if !std::path::Path::new(&file_name).exists() {
            trace!("load_subscriptions_pretty file {} not found. ", &file_name);
            return false;
        }

        let r_string = std::fs::read_to_string(file_name.clone());
        if r_string.is_err() {
            error!("{:?}  {}", r_string.err(), file_name);
            return false;
        }
        let lines = r_string.unwrap();
        let dec_r: serde_json::Result<Vec<SubscriptionEntry>> = serde_json::from_str(&lines);
        if dec_r.is_err() {
            error!("serde_json:from_str {:?}   {:?} ", dec_r.err(), &file_name);
            return false;
        }
        let mut hm = (*self.list).write().unwrap();
        for se in dec_r.unwrap() {
            hm.insert(se.subs_id, se);
        }
        true
    }

    pub fn check_or_store(&mut self) {
        let mut count_changed: bool = false;
        let current_length = (*self.list).read().unwrap().len();
        let dirty_ids: Vec<isize> = (self.list)
            .read()
            .unwrap()
            .iter()
            .filter_map(|(id, se)| if se.is_dirty { Some(*id) } else { None })
            .collect();
        if current_length != self.list_cardinality_last {
            count_changed = true;
        }
        if count_changed || !dirty_ids.is_empty() {
            self.store_to_file_pretty();
            (*self.list)
                .write()
                .unwrap()
                .iter_mut()
                .filter(|(id, _se)| dirty_ids.contains(*id))
                .for_each(|(_id, se)| se.is_dirty = false);
        }
    }

    /// renames the old file, then stores the subscription entries in formatted json
    pub fn store_to_file_pretty(&mut self) {
        let mut values = (*self.list)
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<SubscriptionEntry>>();
        values.sort_by(|a, b| a.subs_id.cmp(&b.subs_id));
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
        let file_name = format!("{}/{}", self.folder_name, FILENAME_JSON);
        let file_name_old = format!("{}/{}.old", self.folder_name, FILENAME_JSON);
        let r_rename = std::fs::rename(&file_name, &file_name_old);
        if r_rename.is_err() {
            warn!(
                "renamed {} => {} : Error {:?}",
                &file_name,
                &file_name_old,
                r_rename.err()
            );
        }
        let r_file = std::fs::File::create(file_name.clone());
        if r_file.is_err() {
            warn!("{:?} writing to {} ", r_file.err(), &file_name);
            return;
        }
        let outfile = r_file.unwrap();
        let bufwriter = BufWriter::new(outfile);
        let mut serializer = serde_json::Serializer::with_formatter(bufwriter, formatter);
        let r_ser = values.serialize(&mut serializer);
        if r_ser.is_err() {
            warn!("serializing into {} => {:?}", file_name, r_ser.err());
        }
    }

    /// recursive, depth-first
    pub fn dump_tree_rec(&self, lpath: &[u16], parent_subs_id: isize, ident: &str) {
        let entries = self.get_by_parent_repo_id(parent_subs_id as isize);
        entries.iter().enumerate().for_each(|(n, fse)| {
            let mut path: Vec<u16> = Vec::new();
            path.extend_from_slice(lpath);
            path.push(n as u16);
            trace!("{}\t{:?}\t{} ", ident, path, fse);
            self.dump_tree_rec(&path, fse.subs_id, ident);
        });
    }

    // TODO : catch exceeding depth
    pub fn update_paths_rec(
        &self,
        localpath: &[u16],
        parent_subs_id: i32,
        mut is_deleted: bool,
    ) -> bool {
        if parent_subs_id < 0 {
            is_deleted = true;
        }
        let entries: Vec<SubscriptionEntry> = self.get_by_parent_repo_id(parent_subs_id as isize);
        let child_ids: Vec<isize> = entries
            .iter()
            .map(|entry| entry.subs_id)
            .collect::<Vec<isize>>();
        child_ids.iter().enumerate().for_each(|(num, child_id)| {
            let mut path: Vec<u16> = Vec::new();
            path.extend_from_slice(localpath);
            path.push(num as u16);
            self.update_paths_rec(&path, *child_id as i32, is_deleted);
            if let Some(mut subs_e) = self.list.write().unwrap().get_mut(child_id) {
                subs_e.tree_path = Some(path);
                subs_e.set_deleted(is_deleted)
            }
        });
        false
    }

    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        self.ctx.get_connection()
    }
}

impl ISubscriptionRepo for SubscriptionRepo {
    /// sorts by folder_position
    fn get_by_parent_repo_id(&self, parent_subs_id: isize) -> Vec<SubscriptionEntry> {
        if self.migr_read_from_json {
            let mut list = (*self.list)
                .read()
                .unwrap()
                .iter()
                .map(|(_id, sub)| sub)
                .filter(|sub| sub.parent_subs_id == parent_subs_id)
                .cloned()
                .collect::<Vec<SubscriptionEntry>>();
            list.sort_by(|a, b| a.folder_position.cmp(&b.folder_position));
            return list;
        }

        let prepared = format!(
            "SELECT * FROM {} WHERE parent_subs_id={} order by folder_position ",
            SubscriptionEntry::table_name(),
            parent_subs_id
        );

        self.ctx.get_list(prepared)
    }

    /// get by parent_subs_id  and folder_position
    fn get_by_pri_fp(&self, parent_subs_id: isize, folder_pos: isize) -> Vec<SubscriptionEntry> {
        if self.migr_read_from_json {
            let mut list = (*self.list)
                .read()
                .unwrap()
                .iter()
                .map(|(_id, sub)| sub)
                .filter(|sub| sub.parent_subs_id == parent_subs_id)
                .filter(|sub| sub.folder_position == folder_pos)
                .cloned()
                .collect::<Vec<SubscriptionEntry>>();
            list.sort_by(|a, b| a.folder_position.cmp(&b.folder_position));
            return list;
        }

        let prepared = format!(
		"SELECT * FROM {} WHERE parent_subs_id={} AND folder_position={}  order by folder_position ",
		SubscriptionEntry::table_name(),
		parent_subs_id,
		folder_pos
	);

        self.ctx.get_list(prepared)
        /*
                let mut list: Vec<SubscriptionEntry> = Vec::default();
                if let Ok(mut stmt) = (*self.get_connection()).lock().unwrap().prepare(&prepared) {
                    match stmt.query_map([], |row| {
                        list.push(SubscriptionEntry::from_row(row));
                        Ok(())
                    }) {
                        Ok(mr) => {
                            mr.count(); // seems to be necessary
                        }
                        Err(e) => error!("{} {:?}", &prepared, e),
                    }
                }
                list
        */
    }

    /// sorts by folder_position
    fn get_all_nonfolder(&self) -> Vec<SubscriptionEntry> {
        if self.migr_read_from_json {
            let mut list = (*self.list)
                .read()
                .unwrap()
                .iter()
                .map(|(_id, sub)| sub)
                .filter(|sub| !sub.is_folder)
                .cloned()
                .collect::<Vec<SubscriptionEntry>>();
            list.sort_by(|a, b| a.folder_position.cmp(&b.folder_position));
            return list;
        }

        let prepared = format!(
            "SELECT * FROM {} WHERE is_folder=false order by folder_position ",
            SubscriptionEntry::table_name(),
        );
        self.ctx.get_list(prepared)
    }

    /// checks for  updated_int,  retrieves those earlier than the given date
    /// new:  order by updated-time. TODO check the order
    fn get_by_fetch_time(&self, updated_time_s: i64) -> Vec<SubscriptionEntry> {
        if self.migr_read_from_json {
            return (*self.list)
                .read()
                .unwrap()
                .iter()
                .map(|(_id, sub)| sub)
                .filter(|sub| !sub.is_folder && sub.updated_int <= updated_time_s)
                .cloned()
                .collect::<Vec<SubscriptionEntry>>();
        }

        let prepared = format!(
            "SELECT * FROM {} WHERE updated_int<{}  order by updated_int ",
            SubscriptionEntry::table_name(),
            updated_time_s
        );
        self.ctx.get_list(prepared)
    }

    fn get_by_index(&self, indexvalue: isize) -> Option<SubscriptionEntry> {
        if self.migr_read_from_json {
            return (*self.list)
                .read()
                .unwrap()
                .iter()
                .map(|(_id, sub)| sub)
                .find(|sub| sub.subs_id == indexvalue)
                .cloned();
        }

        self.ctx.get_by_index(indexvalue)
    }

    /// sorted by subs_id
    fn get_all_entries(&self) -> Vec<SubscriptionEntry> {
        if self.migr_read_from_json {
            let mut se_list = (*self.list)
                .read()
                .unwrap()
                .iter()
                .map(|(_id, sub)| sub)
                .cloned()
                .collect::<Vec<SubscriptionEntry>>();
            se_list.sort_by(|a, b| a.subs_id.cmp(&b.subs_id));
            return se_list;
        }

        self.ctx.get_all()
    }

    ///   store IconID into feed source
    fn update_icon_id(&self, src_id: isize, icon_id: usize, timestamp_s: i64) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.icon_id = icon_id;
                entry.updated_icon = timestamp_s;
                entry.is_dirty = true;
            }
            None => {
                debug!("update_icon_id: not found {}", src_id);
            }
        };

        let sql = format!(
            "UPDATE {}  SET  icon_id={}, updated_icon={} WHERE {}={} ",
            SubscriptionEntry::table_name(),
            icon_id,
            timestamp_s,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn update_folder_position(&self, src_id: isize, new_folder_pos: isize) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.folder_position = new_folder_pos;
                entry.is_dirty = true;
            }
            None => {
                debug!("update_folder_position: not found {}", src_id);
            }
        };

        let sql = format!(
            "UPDATE {}  SET  folder_position={}  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            new_folder_pos,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn update_expanded(&self, src_ids: Vec<isize>, new_expanded: bool) {
        (*self.list)
            .write()
            .unwrap()
            .iter_mut()
            .filter(|(id, _se)| src_ids.contains(id))
            .for_each(|(_id, se)| {
                se.expanded = new_expanded;
                se.is_dirty = true;
            });

        let joined = src_ids
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(
            "UPDATE {}  SET  expanded={}  WHERE {} in ( {} ) ",
            SubscriptionEntry::table_name(),
            new_expanded,
            SubscriptionEntry::index_column_name(),
            joined
        );
        self.ctx.execute(sql);
    }

    fn update_parent_and_folder_position(
        &self,
        src_id: isize,
        new_parent_id: isize,
        new_folder_pos: isize,
    ) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.parent_subs_id = new_parent_id;
                entry.folder_position = new_folder_pos;
                entry.is_dirty = true;
            }
            None => {
                debug!("update_parent_and_folder_position: not found {}", src_id);
            }
        };

        let sql = format!(
            "UPDATE {}  SET   parent_subs_id={},  folder_position={}  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            new_parent_id,
            new_folder_pos,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    /// display name shall be encoded
    fn update_displayname(&self, src_id: isize, new_name: String) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.display_name = new_name.clone();
                entry.is_dirty = true;
            }
            None => {
                debug!("update_displayname: not found {}", src_id);
            }
        };

        let sql = format!(
            "UPDATE {}  SET   display_name=\"{}\"  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            new_name,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    /// url shall be encoded
    fn update_url(&self, src_id: isize, new_url: String) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.url = new_url.clone();
                entry.is_dirty = true;
            }
            None => {
                debug!("update_url: not found {}", src_id);
            }
        };

        let sql = format!(
            "UPDATE {}  SET   url=\"{}\"  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            new_url,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn update_timestamps(&self, src_id: isize, updated_int: i64, updated_ext: Option<i64>) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.updated_int = updated_int;
                if let Some(e) = updated_ext {
                    entry.updated_ext = e;
                }
                entry.is_dirty = true;
            }
            None => {
                debug!("update_timestamps: not found {}", src_id);
            }
        };

        let upd_ext_s = if let Some(ue) = updated_ext {
            format!(" updated_ext={},", ue)
        } else {
            String::default()
        };

        let sql = format!(
            "UPDATE {}  SET   updated_int={} {}  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            updated_int,
            upd_ext_s,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn update_last_selected(&self, src_id: isize, content_id: isize) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.last_selected_msg = content_id;
                entry.is_dirty = true;
            }
            None => {
                debug!("update_last_selected: not found {}", src_id);
            }
        };

        let sql = format!(
            "UPDATE {}  SET   last_selected_msg={}  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            content_id,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn set_schedule_fetch_all(&self) {
        self.list
            .write()
            .unwrap()
            .iter_mut()
            .filter(|(_id, entry)| !entry.is_folder && !entry.is_deleted())
            .for_each(|(_id, entry)| {
                entry.set_fetch_scheduled(true);
            });
    }

    fn store_entry(
        &self,
        entry: &SubscriptionEntry,
    ) -> Result<SubscriptionEntry, Box<dyn std::error::Error>> {
        if self.migr_read_from_json {
            let mut new_id = entry.subs_id;
            if new_id == 0 {
                let max_id = match (*self.list).read().unwrap().keys().max() {
                    Some(id) => *id,
                    None => 0,
                };
                let max_id = std::cmp::max(max_id, 9); // start value
                new_id = max_id + 1;
            }
            let mut store_entry = entry.clone();
            store_entry.subs_id = new_id;
            store_entry.is_dirty = false;
            // debug!("INSERT:{}   {:?}", &self.filename, &store_entry);
            (*self.list)
                .write()
                .unwrap()
                .insert(new_id, store_entry.clone());
            return Ok(store_entry);
        }

        // TODO handle    subs_id == 0

        match self.ctx.insert(&entry) {
            Ok(indexval) => {
                let mut ret_e: SubscriptionEntry = entry.clone();
                ret_e.subs_id = indexval as isize;
                return Ok(ret_e);
            }
            Err(e) => {
                error!(" {:?}", e);
                return Err(Box::new(e));
            }
        }
    }

    fn delete_by_index(&self, del_index: isize) {
        match (*self.list).write().unwrap().remove(&del_index) {
            Some(e) => debug!("deleted : {:?}", e),
            None => debug!("delete_by_index: not found {}", del_index),
        }

        let sql = format!(
            "DELETE FROM {}   WHERE {}={} ",
            SubscriptionEntry::table_name(),
            SubscriptionEntry::index_column_name(),
            del_index
        );
        self.ctx.execute(sql);
    }

    fn set_deleted_rec(&self, del_index: isize) {
        let mut scan_list: Vec<isize> = Vec::default();
        scan_list.push(del_index);
        while let Some(idx) = scan_list.pop() {
            let child_list = self.get_by_parent_repo_id(idx);
            for se in &child_list {
                scan_list.push(se.subs_id);
            }
            child_list
                .iter()
                .for_each(|se| self.update_deleted(se.subs_id, true));
        }
        self.update_deleted(del_index, true);

        info!("TODO:  update_deleted_rec  for database ");
    }

    // deprecated
    fn get_list(&self) -> Arc<RwLock<HashMap<isize, SubscriptionEntry>>> {
        self.list.clone()
    }

    fn debug_dump_tree(&self, ident: &str) {
        self.dump_tree_rec(&[], SRC_REPO_ID_MOVING, ident); // parent_id for moving elements
        self.dump_tree_rec(&[], 0, ident);
    }

    fn clear(&self) {
        (*self.list).write().unwrap().clear();

        self.ctx.delete_table();
        self.ctx.create_table();
    }

    fn get_ids_by_status(
        &self,
        statusflag: StatusMask,
        activated: bool,
        include_folder: bool,
    ) -> Vec<isize> {
        let mask = statusflag as usize;
        self.list
            .read()
            .unwrap()
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

    fn get_tree_path(&self, db_id: isize) -> Option<Vec<u16>> {
        if let Some(entry) = self.list.read().unwrap().get(&db_id) {
            if let Some(p) = &entry.tree_path {
                return Some(p.clone());
            }
        }
        None
    }

    fn set_status(&self, idlist: &[isize], statusflag: StatusMask, activated: bool) {
        let mask = statusflag as usize;
        self.list
            .write()
            .unwrap()
            .iter_mut()
            .filter(|(id, _entry)| idlist.contains(*id))
            .for_each(|(_id, entry)| entry.change_bitmask(mask, activated));
    }

    fn get_by_path(&self, path: &[u16]) -> Option<SubscriptionEntry> {
        self.list.read().unwrap().iter().find_map(|(_id, entry)| {
            if let Some(e_path) = &entry.tree_path {
                if e_path == path {
                    return Some(entry.clone());
                }
            }
            None
        })
    }

    fn update_cached_paths(&self) {
        self.update_paths_rec(&Vec::<u16>::default(), 0, false);
    }

    fn set_num_all_unread(
        &self,
        subs_id: isize,
        num_all: isize,
        num_unread: isize,
    ) -> Option<SubscriptionEntry> {
        if let Some(entry) = self.list.write().unwrap().get_mut(&subs_id) {
            entry.num_msg_all_unread = Some((num_all, num_unread));
            return Some(entry.clone());
        } else {
            debug!("set_num_all_unread({})  ID not found", subs_id);
        }
        None
    }

    fn clear_num_all_unread(&self, subs_id: isize) {
        if let Some(entry) = self.list.write().unwrap().get_mut(&subs_id) {
            entry.num_msg_all_unread = None;
        }
    }

    fn get_num_all_unread(&self, subs_id: isize) -> Option<(isize, isize)> {
        if let Some(entry) = self.list.write().unwrap().get_mut(&subs_id) {
            return entry.num_msg_all_unread;
        }
        None
    }

    /// don't include deleted ones, no folders,
    fn scan_num_all_unread(&self) -> Option<isize> {
        let unproc_id: Option<isize> = self.list.read().unwrap().iter().find_map(|(id, se)| {
            if !se.is_folder
                && se.num_msg_all_unread.is_none()
                && se.subs_id > 0
                && se.parent_subs_id > 0
                && !se.is_deleted()
            {
                Some(*id)
            } else {
                None
            }
        });
        unproc_id
    }

    fn get_highest_src_id(&self) -> isize {
        let o_highest = self.list.read().unwrap().iter().map(|(id, _fse)| *id).max();
        o_highest.unwrap_or(0)
    }

    fn update_deleted(&self, src_id: isize, is_del: bool) {
        match (*self.list).write().unwrap().get_mut(&src_id) {
            Some(mut entry) => {
                entry.deleted = is_del;
            }
            None => {
                debug!("update_deleted: not found {}", src_id);
            }
        };
    }

    fn store_default_db_entries(&self) {
        let mut fse = SubscriptionEntry {
            subs_id: SRC_REPO_ID_DELETED,
            display_name: "_deleted".to_string(),
            is_folder: true,
            parent_subs_id: -1,
            ..Default::default()
        };
        let _r = self.store_entry(&fse);

        fse.subs_id = SRC_REPO_ID_MOVING;
        fse.display_name = "_moving".to_string();
        let _r = self.store_entry(&fse);

        fse.subs_id = SRC_REPO_ID_DUMMY;
        fse.display_name = "_dummy".to_string();
        let _r = self.store_entry(&fse);
    }
} // ISubscriptionRepo

//-------------------

impl Buildable for SubscriptionRepo {
    type Output = SubscriptionRepo;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let o_folder = conf.get(KEY_FOLDERNAME);
        match o_folder {
            Some(folder) => SubscriptionRepo::new(&folder),
            None => {
                conf.dump();
                panic!("subscription config has no {} ", KEY_FOLDERNAME);
            }
        }
    }

    fn section_name() -> String {
        String::from("subscriptions_repo")
    }
}

impl StartupWithAppContext for SubscriptionRepo {
    fn startup(&mut self, ac: &AppContext) {
        let timer_r: Rc<RefCell<Timer>> = (*ac).get_rc::<Timer>().unwrap();
        let su_r = ac.get_rc::<SubscriptionRepo>().unwrap();
        {
            (*timer_r)
                .borrow_mut()
                .register(&TimerEvent::Timer10s, su_r.clone());
            (*timer_r)
                .borrow_mut()
                .register(&TimerEvent::Shutdown, su_r);
        }

        self.startup();
    }
}

impl TimerReceiver for SubscriptionRepo {
    fn trigger(&mut self, event: &TimerEvent) {
        match event {
            TimerEvent::Timer10s => {
                self.check_or_store();
            }
            TimerEvent::Shutdown => {
                debug!("SubscriptionRepo-shutdown");
                self.check_or_store();
            }
            _ => (),
        }
    }
}

#[allow(dead_code)]
fn subscription_entry_to_json(input: &SubscriptionEntry) -> Option<String> {
    match serde_json::to_string(input) {
        Ok(encoded) => Some(encoded),
        Err(er) => {
            error!("serde_json {:?} \n {:?}", er, &input.subs_id);
            None
        }
    }
}

#[allow(dead_code)]
fn subscription_entry_to_txt(input: &SubscriptionEntry) -> Option<String> {
    match bincode::serialize(input) {
        //         Ok(encoded) => Some(compress(&encoded)),
        Ok(encoded) => Some(compress(String::from_utf8(encoded).unwrap().as_str())),
        Err(er) => {
            error!("bincode_serizalize {:?} \n {:?}", er, &input.subs_id);
            None
        }
    }
}

#[allow(dead_code)]
fn json_to_subscription_entry(line: String) -> Option<SubscriptionEntry> {
    let dec_r: serde_json::Result<SubscriptionEntry> = serde_json::from_str(&line);
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("serde_json:from_str {:?}   {:?} ", e, &line);
            None
        }
    }
}

#[allow(dead_code)]
fn txt_to_subscription_entry(line: String) -> Option<SubscriptionEntry> {
    let dc_bytes = decompress(&line);
    let dec_r: bincode::Result<SubscriptionEntry> = bincode::deserialize(dc_bytes.as_bytes());
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("bincode:deserialize {:?}   {:?} ", e, &line);
            None
        }
    }
}

#[cfg(test)]
mod ut {
    use super::*;

    pub const TEST_FOLDER1: &'static str = "../target/db_t_sub_rep";

    //cargo test   db::subscription_repo::ut::t_delete_subscription  --lib  -- --exact --nocapture
    #[test]
    fn t_delete_subscription() {
        setup();
        let subscrip_repo = SubscriptionRepo::new("");
        let mut e = SubscriptionEntry::default();
        e.subs_id = 1;
        let _r = subscrip_repo.store_entry(&e);
        e.subs_id = 2;
        e.parent_subs_id = 1;
        let _r = subscrip_repo.store_entry(&e);
        e.subs_id = 3;
        e.parent_subs_id = 2;
        let _r = subscrip_repo.store_entry(&e);
        subscrip_repo.set_deleted_rec(1);
        let all = subscrip_repo.get_all_entries();
        dbg!(&all);
        assert!(all.get(0).unwrap().is_deleted());
        assert!(all.get(1).unwrap().is_deleted());
        assert!(all.get(2).unwrap().is_deleted());
    }

    #[test]
    fn t_store_file() {
        setup();
        {
            let mut sr = SubscriptionRepo::new(TEST_FOLDER1);
            sr.startup();
            sr.clear();
            let s1 = SubscriptionEntry::default();
            assert!(sr.store_entry(&s1).is_ok());
            assert!(sr.store_entry(&s1).is_ok());
            let list = sr.get_all_entries();
            assert_eq!(list.len(), 2);
            sr.check_or_store();
        }
        {
            let mut sr = SubscriptionRepo::new(TEST_FOLDER1);
            sr.startup();
            let list = sr.get_all_entries();
            // list.iter().for_each(|l| debug!("ST {:?}", l));
            assert_eq!(list.len(), 2);
        }
    }

    #[test]
    fn t_update_last_selected() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1); // update_url
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_last_selected(10, 20);
        assert_eq!(sr.get_by_index(10).unwrap().last_selected_msg, 20);
    }

    #[test]
    fn t_update_timestamps() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1); // update_url
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_timestamps(10, 20, None);
        sr.update_timestamps(11, 30, Some(40));
        assert_eq!(sr.get_by_index(10).unwrap().updated_int, 20);
        assert_eq!(sr.get_by_index(11).unwrap().updated_int, 30);
        assert_eq!(sr.get_by_index(11).unwrap().updated_ext, 40);
    }

    #[test]
    fn t_update_url() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1); // update_url
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_url(10, "hhttps:".to_string());
        assert_eq!(sr.get_by_index(10).unwrap().url, "hhttps:".to_string());
    }

    #[test]
    fn t_update_displayname() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_displayname(10, "updated".to_string());
        assert_eq!(
            sr.get_by_index(10).unwrap().display_name,
            "updated".to_string()
        );
    }

    #[test]
    fn t_delete_by_index() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.delete_by_index(10);
        let list = sr.get_all_entries();
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0).unwrap().subs_id, 11);
    }

    #[test]
    fn t_update_parent_and_folder_position() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_parent_and_folder_position(10, 20, 30);
        let e = sr.get_by_index(10).unwrap();
        assert_eq!(e.parent_subs_id, 20);
        assert_eq!(e.folder_position, 30);
    }

    #[test]
    fn t_update_expanded() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_expanded(vec![10], true);
        let e = sr.get_by_index(10).unwrap();
        assert!(e.expanded);
    }

    #[test]
    fn t_update_folder_position() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_folder_position(10, 4);
        let e = sr.get_by_index(10).unwrap();
        assert_eq!(e.folder_position, 4);
    }

    #[test]
    fn t_update_icon_id() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_icon_id(10, 2, 3);

        let e = sr.get_by_index(10).unwrap();
        assert_eq!(e.icon_id, 2);
        assert_eq!(e.updated_icon, 3);
    }

    #[test]
    //cargo test   db::subscription_repo::ut::t_get_by_fetch_time  --lib  -- --exact
    fn t_get_by_fetch_time() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 20;
        assert!(sr.store_entry(&s1).is_ok());
        s1.updated_int = 5;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_by_fetch_time(3);
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0).unwrap().subs_id, 10);
    }

    #[test]
    fn t_get_all_nonfolder() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 20;
        assert!(sr.store_entry(&s1).is_ok());
        s1.is_folder = true;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_all_nonfolder();
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0).unwrap().subs_id, 10);
    }

    #[test]
    fn t_store_entry() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        let mut s1 = SubscriptionEntry::default();
        s1.display_name = "t_store_entry".to_string();
        let r = sr.store_entry(&s1);
        assert!(r.is_ok());
        let entry = r.unwrap();
        assert_eq!(entry.subs_id, 10);
    }

    #[test]
    fn t_get_by_pri_fp() {
        setup();
        let sr = SubscriptionRepo::new(TEST_FOLDER1);
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 20;
        assert!(sr.store_entry(&s1).is_ok());
        s1.folder_position = 1;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_by_pri_fp(20, 1);
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0).unwrap().subs_id, 11);
    }


	#[ignore]	// TODO
    #[test]
    //cargo test   db::subscription_repo::ut::t_get_by_parent_subs_id  --lib  -- --exact --nocapture
    fn t_get_by_parent_subs_id() {
        setup();
        let sr = SubscriptionRepo::new_inmem(); // SubscriptionRepo::new(TEST_FOLDER1);
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 7;
        s1.folder_position = 0;
        assert!(sr.store_entry(&s1).is_ok());
        s1.parent_subs_id = 7;
        s1.folder_position = 1;
        assert!(sr.store_entry(&s1).is_ok());
        s1.parent_subs_id = 7;
        s1.folder_position = 2;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_by_parent_repo_id(7);
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0).unwrap().subs_id, 10);
        assert_eq!(list.get(0).unwrap().folder_position, 0);
        assert_eq!(list.get(1).unwrap().subs_id, 11);
        assert_eq!(list.get(1).unwrap().folder_position, 1);
        assert_eq!(list.get(2).unwrap().subs_id, 12);
        assert_eq!(list.get(2).unwrap().folder_position, 2);
    }

    // TODO remove
    //cargo test   db::subscription_repo::ut::t_store_and_read_pretty_json  --lib  -- --exact --nocapture
	#[ignore]
    #[test]
    fn t_store_and_read_pretty_json() {
        setup();
        let repopath = "../target/db_sr_pretty";
        {
            let mut sr = SubscriptionRepo::new(repopath);
            sr.startup();
            sr.clear();
            let s1 = SubscriptionEntry::default();
            assert!(sr.store_entry(&s1).is_ok());
            assert!(sr.store_entry(&s1).is_ok());
            let list = sr.get_all_entries();
            assert_eq!(list.len(), 2);
            sr.store_to_file_pretty();
        }
        {
            let mut sr = SubscriptionRepo::new(repopath);
            sr.startup();
            let entries = sr.get_all_entries();
            assert_eq!(entries.len(), 2);
        }
    }

	#[ignore]		// TODO 
    //cargo test   db::subscription_repo::ut::t_store_and_read_pretty_json  --lib  -- --exact --nocapture
    #[test]
    fn t_store_and_read_on_fs() {
        setup();
        let repopath = "../target/db_sr_pretty";
        {
            let mut sr = SubscriptionRepo::new2(repopath.to_string());
            sr.startup();
            sr.clear();
            let s1 = SubscriptionEntry::default();
            let r1 = sr.store_entry(&s1);
            assert!(r1.is_ok());
            assert!(sr.store_entry(&s1).is_ok());
            let list = sr.get_all_entries();
            assert_eq!(list.len(), 2);
            sr.store_to_file_pretty();
        }
        {
            let mut sr = SubscriptionRepo::new2(repopath.to_string());
            sr.startup();
            let entries = sr.get_all_entries();
            assert_eq!(entries.len(), 2);
        }
    }

    // dummy instead of log configuration
    fn setup() {}
}
