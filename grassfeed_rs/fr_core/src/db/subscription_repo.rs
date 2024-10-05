use crate::controller::timer::Timer;
use crate::db::errors_repo;
use crate::db::sqlite_context::SqliteContext;
use crate::db::sqlite_context::TableInfo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_entry::SRC_REPO_ID_DELETED;
use crate::db::subscription_entry::SRC_REPO_ID_DUMMY;
use crate::db::subscription_entry::SRC_REPO_ID_MOVING;
use crate::util::file_exists;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use rusqlite::Connection;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;

pub const KEY_FOLDERNAME: &str = "subscriptions_folder";


pub trait ISubscriptionRepo {
    /// sorts by folder_position
    fn get_children(&self, parent_subs_id: isize) -> Vec<SubscriptionEntry>;

    /// get by parent_subs_id  and folder_position
    fn get_by_pri_fp(&self, parent_subs_id: isize, folder_pos: isize) -> Vec<SubscriptionEntry>;

    /// sorts by folder_position
    fn get_all_nonfolder(&self) -> Vec<SubscriptionEntry>;

    /// checks for  updated_int,  retrieves those earlier than the given date
    fn get_by_fetch_time(&self, updated_time_s: i64) -> Vec<SubscriptionEntry>;

    fn get_by_index(&self, indexvalue: isize) -> Option<SubscriptionEntry>;

    fn get_all_entries(&self) -> Vec<SubscriptionEntry>;

    fn get_by_icon_id(&self, icon_id: isize) -> Vec<SubscriptionEntry>;

    /// if subs_id == 0  stores at next possible higher  subs_id.
    /// if subs_id is given, we store that.
    fn store_entry(
        &self,
        entry: &SubscriptionEntry,
    ) -> Result<SubscriptionEntry, Box<dyn std::error::Error>>;

    ///   store IconID into subscription entry
    fn update_icon_id_time(&self, src_id: isize, icon_id: usize, timestamp_s: i64);
    fn update_icon_id(&self, src_id: isize, icon_id: usize);
    fn update_icon_id_many(&self, src_ids: Vec<i32>, icon_id: usize);

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

    fn update_homepage(&self, src_id: isize, new_url: &str);

    fn delete_by_index(&self, del_index: isize);

    /// clear:   deletes the table, and recreates it. Use only inside tests.
    fn scrub_all_subscriptions(&self);

    fn debug_dump_tree(&self, ident: &str);

    fn get_highest_src_id(&self) -> isize;

    ///  put the topmost entry to deleted-parent,  set the deleted flag on all entries below
    fn set_deleted_rec(&self, del_index: isize);

    fn store_default_db_entries(&self);

    fn get_connection(&self) -> Arc<Mutex<Connection>>;

    fn db_vacuum(&self) -> usize;

    //  is false if no DB was present - on fresh start
    fn db_existed_before(&self) -> bool;
}

pub struct SubscriptionRepo {
    folder_name: String,
    ctx: SqliteContext<SubscriptionEntry>,
}

impl SubscriptionRepo {
    pub fn by_folder(folder_conf: &str, folder_cache: &str) -> Self {
        let reg_filename = Self::filename(folder_conf);
        let reg_existed = file_exists(&reg_filename);
        if reg_existed {
            let month_num = chrono::prelude::Utc::now().format("%m"); // %Y-%m-%d
            let sub_copy_file = format!("{folder_cache}subscriptions.db-{month_num}");
            let r = std::fs::copy(&reg_filename, &sub_copy_file);
            if r.is_err() {
                error!(
                    "Error: copy {} to {}  => {:?}",
                    reg_filename,
                    sub_copy_file,
                    r.err()
                );
            }
        }

        SubscriptionRepo {
            folder_name: folder_conf.to_string(),
            ctx: SqliteContext::new(&reg_filename),
        }
    }

    pub fn filename(foldername: &str) -> String {
        format!("{foldername}subscriptions.db")
    }

    pub fn by_file(filename: &str) -> Self {
        SubscriptionRepo {
            folder_name: String::default(),
            ctx: SqliteContext::new(filename),
        }
    }

    pub fn by_existing_connection(con: Arc<Mutex<Connection>>) -> Self {
        SubscriptionRepo {
            folder_name: String::default(),
            ctx: SqliteContext::new_by_connection(con),
        }
    }

    pub fn new_inmem() -> Self {
        SubscriptionRepo {
            folder_name: String::default(),
            ctx: SqliteContext::new_in_memory(),
        }
    }

    pub fn startup_int(&mut self) -> bool {
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
        self.ctx.create_table();
        self.store_default_db_entries();
        true
    }

    /// recursive, depth-first
    pub fn dump_tree_rec(&self, lpath: &[u16], parent_subs_id: isize, ident: &str) {
        let entries = self.get_children(parent_subs_id);
        entries.iter().enumerate().for_each(|(n, fse)| {
            let mut path: Vec<u16> = Vec::new();
            path.extend_from_slice(lpath);
            path.push(n as u16);
            trace!("{}\t{:?}  {} ", ident, path, fse);
            self.dump_tree_rec(&path, fse.subs_id, ident);
        });
    }

    fn update_deleted_list(&self, src_ids: Vec<isize>, is_del: bool) {
        let joined = src_ids
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");

        let sql = format!(
            "UPDATE {}  SET   deleted={}   WHERE {} in ({}) ",
            SubscriptionEntry::table_name(),
            is_del,
            SubscriptionEntry::index_column_name(),
            joined
        );
        self.ctx.execute(sql);
    }
}

impl ISubscriptionRepo for SubscriptionRepo {
    /// sorts by folder_position
    fn get_children(&self, parent_subs_id: isize) -> Vec<SubscriptionEntry> {
        let prepared = format!(
            "SELECT * FROM {} WHERE parent_subs_id={} order by folder_position ",
            SubscriptionEntry::table_name(),
            parent_subs_id
        );
        self.ctx.get_list(prepared)
    }

    /// get by parent_subs_id  and folder_position
    fn get_by_pri_fp(&self, parent_subs_id: isize, folder_pos: isize) -> Vec<SubscriptionEntry> {
        let prepared = format!(
		"SELECT * FROM {} WHERE parent_subs_id={} AND folder_position={}  order by folder_position ",
		SubscriptionEntry::table_name(),		parent_subs_id,		folder_pos			);
        self.ctx.get_list(prepared)
    }

    /// sorts by folder_position
    fn get_all_nonfolder(&self) -> Vec<SubscriptionEntry> {
        let prepared = format!(
            "SELECT * FROM {} WHERE is_folder=false order by folder_position ",
            SubscriptionEntry::table_name(),
        );
        self.ctx.get_list(prepared)
    }

    /// checks for  updated_int,  retrieves those earlier than the given date
    /// returns the list   order by updated-time
    fn get_by_fetch_time(&self, updated_time_s: i64) -> Vec<SubscriptionEntry> {
        let prepared = format!(
            "SELECT * FROM {} WHERE updated_int<{}  order by updated_int ",
            SubscriptionEntry::table_name(),
            updated_time_s
        );
        self.ctx.get_list(prepared)
    }

    fn get_by_icon_id(&self, icon_id: isize) -> Vec<SubscriptionEntry> {
        let prepared = format!(
            "SELECT * FROM {} WHERE icon_id={}  order by updated_int ",
            SubscriptionEntry::table_name(),
            icon_id
        );
        self.ctx.get_list(prepared)
    }

    fn get_by_index(&self, indexvalue: isize) -> Option<SubscriptionEntry> {
        self.ctx.get_by_index(indexvalue)
    }

    /// sorted by subs_id
    fn get_all_entries(&self) -> Vec<SubscriptionEntry> {
        self.ctx.get_all()
    }

    ///   store IconID into subscription entry
    fn update_icon_id_time(&self, subs_id: isize, icon_id: usize, timestamp_s: i64) {
        let sql = format!(
            "UPDATE {}  SET  icon_id={}, updated_icon={} WHERE {}={} ",
            SubscriptionEntry::table_name(),
            icon_id,
            timestamp_s,
            SubscriptionEntry::index_column_name(),
            subs_id
        );
        self.ctx.execute(sql);
    }

    ///   store IconID into subscription entry
    fn update_icon_id(&self, subs_id: isize, icon_id: usize) {
        let sql = format!(
            "UPDATE {}  SET  icon_id={}  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            icon_id,
            SubscriptionEntry::index_column_name(),
            subs_id
        );
        self.ctx.execute(sql);
    }

    fn update_icon_id_many(&self, src_ids: Vec<i32>, icon_id: usize) {
        let joined = src_ids
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");

        let sql = format!(
            "UPDATE {}  SET  icon_id={}  WHERE {} in ( {} ) ",
            SubscriptionEntry::table_name(),
            icon_id,
            SubscriptionEntry::index_column_name(),
            joined
        );
        self.ctx.execute(sql);
    }

    fn update_folder_position(&self, src_id: isize, new_folder_pos: isize) {
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
        let sql = format!(
            "UPDATE {}  SET   url=\"{}\"  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            new_url,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn update_homepage(&self, src_id: isize, new_url: &str) {
        let sql = format!(
            "UPDATE {}  SET   website_url=\"{}\"  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            new_url,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn update_timestamps(&self, src_id: isize, updated_int: i64, updated_ext: Option<i64>) {
        let upd_ext_s = if let Some(ue) = updated_ext {
            format!(", updated_ext={ue}")
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
        let sql = format!(
            "UPDATE {}  SET   last_selected_msg={}  WHERE {}={} ",
            SubscriptionEntry::table_name(),
            content_id,
            SubscriptionEntry::index_column_name(),
            src_id
        );
        self.ctx.execute(sql);
    }

    fn store_entry(
        &self,
        entry: &SubscriptionEntry,
    ) -> Result<SubscriptionEntry, Box<dyn std::error::Error>> {
        match self.ctx.insert(entry, entry.subs_id != 0) {
            Ok(indexval) => {
                let mut ret_e: SubscriptionEntry = entry.clone();
                ret_e.subs_id = indexval as isize;
                Ok(ret_e)
            }
            Err(e) => {
                error!("store_entry: {:?} {:?} ", &entry, e);
                Err(Box::new(e))
            }
        }
    }

    fn delete_by_index(&self, del_index: isize) {
        let sql = format!(
            "DELETE FROM {}   WHERE {}={} ",
            SubscriptionEntry::table_name(),
            SubscriptionEntry::index_column_name(),
            del_index
        );
        self.ctx.execute(sql);
    }

    fn set_deleted_rec(&self, del_index: isize) {
        let mut to_delete_list: HashSet<isize> = HashSet::default();
        to_delete_list.insert(del_index);
        let mut scan_list: Vec<isize> = Vec::default();
        scan_list.push(del_index);
        while let Some(idx) = scan_list.pop() {
            let child_list = self.get_children(idx);
            for se in &child_list {
                scan_list.push(se.subs_id);
                to_delete_list.insert(se.subs_id);
            }
        }
        self.update_deleted_list(to_delete_list.into_iter().collect(), true);
    }

    fn debug_dump_tree(&self, ident: &str) {
        self.dump_tree_rec(&[], SRC_REPO_ID_MOVING, ident); // parent_id for moving elements
        self.dump_tree_rec(&[], 0, ident);
    }

    fn scrub_all_subscriptions(&self) {
        let _r = self.ctx.delete_table();
        self.ctx.create_table();
    }

    fn get_highest_src_id(&self) -> isize {
        let sql = format!(
            "SELECT MAX( subs_id ) FROM {} ",
            SubscriptionEntry::table_name()
        );
        self.ctx.one_number(sql)
    }

    fn store_default_db_entries(&self) {
        let mut fse = SubscriptionEntry {
            subs_id: SRC_REPO_ID_DELETED,
            display_name: "_deleted".to_string(),
            is_folder: true,
            parent_subs_id: -1,
            ..Default::default()
        };
        self.delete_by_index(fse.subs_id);
        let _r = self.store_entry(&fse);

        fse.subs_id = SRC_REPO_ID_MOVING;
        fse.display_name = "_moving".to_string();
        self.delete_by_index(fse.subs_id);
        let _r = self.store_entry(&fse);

        fse.subs_id = SRC_REPO_ID_DUMMY;
        fse.display_name = "_dummy".to_string();
        self.delete_by_index(fse.subs_id);
        let _r = self.store_entry(&fse);
    }

    fn get_connection(&self) -> Arc<Mutex<Connection>> {
        self.ctx.get_connection()
    }

    fn db_vacuum(&self) -> usize {
        self.ctx.execute("VACUUM".to_string())
    }

    fn db_existed_before(&self) -> bool {
        self.ctx.db_existed_before()
    }
} // ISubscriptionRepo

//-------------------

impl Buildable for SubscriptionRepo {
    type Output = SubscriptionRepo;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let o_cache = conf.get(errors_repo::KEY_FOLDERNAME);
        let cachedir = o_cache.unwrap();
        let o_folder = conf.get(KEY_FOLDERNAME);
        match o_folder {
            Some(folder) => SubscriptionRepo::by_folder(&folder, &cachedir),
            None => {
                conf.dump();
                panic!("subscription config has no {KEY_FOLDERNAME} ");
            }
        }
    }
}

impl StartupWithAppContext for SubscriptionRepo {
    fn startup(&mut self, ac: &AppContext) {
        let timer_r = ac.get_rc::<Timer>().unwrap();
        let sr_r = ac.get_rc::<SubscriptionRepo>().unwrap();
        {
            (*timer_r)
                .borrow_mut()
                .register(&TimerEvent::Shutdown, sr_r, false);
        }
        self.startup_int();
    }
}

impl TimerReceiver for SubscriptionRepo {
    fn trigger(&self, event: &TimerEvent) {
        if event == &TimerEvent::Shutdown {
            self.ctx.cache_flush();
        }
    }
}

#[cfg(test)]
mod ut {
    use super::*;

    #[test]
    fn t_update_last_selected() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_last_selected(10, 20);
        assert_eq!(sr.get_by_index(10).unwrap().last_selected_msg, 20);
    }

    #[test]
    fn t_update_url() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_url(10, "hhttps:".to_string());
        assert_eq!(sr.get_by_index(10).unwrap().url, "hhttps:".to_string());
    }

    #[test]
    fn t_update_displayname() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_displayname(10, "updated".to_string());
        assert_eq!(
            sr.get_by_index(10).unwrap().display_name,
            "updated".to_string()
        );
    }

    #[test]
    fn t_update_parent_and_folder_position() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_parent_and_folder_position(10, 20, 30);
        let e = sr.get_by_index(10).unwrap();
        assert_eq!(e.parent_subs_id, 20);
        assert_eq!(e.folder_position, 30);
    }

    #[test]
    fn t_update_expanded() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_expanded(vec![10], true);
        let e = sr.get_by_index(10).unwrap();
        assert!(e.expanded);
    }

    #[test]
    fn t_update_folder_position() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_folder_position(10, 4);
        let e = sr.get_by_index(10).unwrap();
        assert_eq!(e.folder_position, 4);
    }

    #[test]
    fn t_update_icon_id() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_icon_id_time(10, 2, 3);
        let e = sr.get_by_index(10).unwrap();
        assert_eq!(e.icon_id, 2);
        assert_eq!(e.updated_icon, 3);
    }

    #[test]
    fn t_get_all_nonfolder() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
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
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
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
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 20;
        assert!(sr.store_entry(&s1).is_ok());
        s1.folder_position = 1;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_by_pri_fp(20, 1);
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0).unwrap().subs_id, 11);
    }

    // ------------------------------------------------------------

    #[test]
    fn t_update_timestamps() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.update_timestamps(10, 20, None);
        sr.update_timestamps(11, 30, Some(40));
        assert_eq!(sr.get_by_index(10).unwrap().updated_int, 20);
        assert_eq!(sr.get_by_index(11).unwrap().updated_int, 30);
        assert_eq!(sr.get_by_index(11).unwrap().updated_ext, 40);
    }

    #[test]
    fn t_delete_by_index() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        assert!(sr.store_entry(&SubscriptionEntry::default()).is_ok());
        sr.delete_by_index(10);
        let list = sr.get_all_entries();
        assert_eq!(list.len(), 4);
        assert_eq!(list.get(3).unwrap().subs_id, 11);
    }

    #[test]
    //cargo test   db::subscription_repo::ut::t_get_by_fetch_time  --lib  -- --exact
    fn t_get_by_fetch_time() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 20;
        assert!(sr.store_entry(&s1).is_ok());
        s1.updated_int = 5;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_by_fetch_time(3);
        assert_eq!(list.len(), 4);
        assert_eq!(list.get(3).unwrap().subs_id, 10);
    }

    //cargo test   db::subscription_repo::ut::t_delete_subscription  --lib  -- --exact --nocapture
    #[test]
    fn t_delete_subscription() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        let mut e = SubscriptionEntry::default();
        let _r = sr.store_entry(&e);
        e.parent_subs_id = 10;
        let _r = sr.store_entry(&e);
        e.parent_subs_id = 10;
        let _r = sr.store_entry(&e);
        sr.set_deleted_rec(10);
        let all = sr.get_all_entries();
        // all.iter().for_each(|e| debug!("## {:?}", &e));
        assert!(all.get(3).unwrap().deleted);
        assert!(all.get(4).unwrap().deleted);
        assert!(all.get(5).unwrap().deleted);
    }

    #[test]
    //cargo test   db::subscription_repo::ut::t_get_by_parent_subs_id  --lib  -- --exact --nocapture
    fn t_get_by_parent_subs_id() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 7;
        s1.folder_position = 0;
        let r1 = sr.store_entry(&s1);
        assert!(r1.is_ok());
        s1.parent_subs_id = 7;
        s1.folder_position = 1;
        assert!(sr.store_entry(&s1).is_ok());
        s1.parent_subs_id = 7;
        s1.folder_position = 2;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_children(7);
        list.iter().for_each(|e| debug!("##  {:?}", e));
        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0).unwrap().subs_id, 10);
        assert_eq!(list.get(0).unwrap().folder_position, 0);
        assert_eq!(list.get(1).unwrap().subs_id, 11);
        assert_eq!(list.get(1).unwrap().folder_position, 1);
        assert_eq!(list.get(2).unwrap().subs_id, 12);
        assert_eq!(list.get(2).unwrap().folder_position, 2);
    }

    #[test]
    //cargo test   db::subscription_repo::ut::t_get_by_icon_id  --lib  -- --exact
    fn t_get_by_icon_id() {
        setup();
        let mut sr = SubscriptionRepo::new_inmem();
        sr.startup_int();
        let mut s1 = SubscriptionEntry::default();
        s1.parent_subs_id = 20;
        s1.icon_id = 77;
        assert!(sr.store_entry(&s1).is_ok());
        s1.icon_id = 55;
        assert!(sr.store_entry(&s1).is_ok());
        let list = sr.get_by_icon_id(77);
        assert_eq!(list.len(), 1);
        assert_eq!(list.get(0).unwrap().icon_id, 77);
    }

    // dummy instead of log configuration
    fn setup() {}
}
