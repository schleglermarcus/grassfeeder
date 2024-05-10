use crate::controller::timer::Timer;
use crate::db::errorentry;
use crate::db::errorentry::ErrorEntry;
use crate::db::sqlite_context::rusqlite_error_to_boxed;
use crate::db::sqlite_context::SqliteContext;
use crate::db::sqlite_context::TableInfo;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use rusqlite::Connection;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

pub const KEY_FOLDERNAME: &str = "cache_folder";
pub const FILENAME: &str = "errors.json.txt";

pub struct ErrorRepo {
    ctx: SqliteContext<ErrorEntry>,
    errorlines_cache: Mutex<Vec<ErrorEntry>>,
}

impl ErrorRepo {
    pub fn new(folder_n: &str) -> Self {
        match std::fs::create_dir_all(folder_n) {
            Ok(()) => (),
            Err(e) => {
                println!("ErrorRepo cannot create folder {} {:?}", &folder_n, e);
                error!("ErrorRepo cannot create folder {} {:?}", &folder_n, e);
            }
        }
        let filename : String = ErrorRepo::filename(folder_n);
        let dbctx = SqliteContext::new(&filename);
        ErrorRepo {
            ctx: dbctx,
            errorlines_cache: Default::default(),
        }
    }

    pub fn new_in_mem() -> Self {
        let cx = SqliteContext::new_in_memory();
        cx.create_table();
        ErrorRepo {
            ctx: cx,
            errorlines_cache: Default::default(),
        }
    }

    pub fn filename(foldername: &str) -> String {
        format!("{foldername}errors.db")
    }

    pub fn by_connection(ex_con: Arc<Mutex<Connection>>) -> Self {
        ErrorRepo {
            ctx: SqliteContext::new_by_connection(ex_con),
            errorlines_cache: Default::default(),
        }
    }

    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        self.ctx.get_connection()
    }

    pub fn add_error(
        &self,
        subsid: isize,
        esrc: errorentry::ESRC,
        eval: isize,
        http_url: String,
        msg: String,
    ) {
        let en = ErrorEntry {
            subs_id: subsid,
            e_src: esrc as isize,
            e_val: eval,
            text: msg.clone(),
            remote_address: http_url,
            date: crate::util::timestamp_now(),
            ..Default::default()
        };
        if let Ok(mut list_g) = self.errorlines_cache.lock() {
            (*list_g).push(en);
        } else {
            error!("Cannot lock Error cache! {:?} ", en);
        }
    }

    pub fn add_error_ts(
        &self,
        subsid: isize,
        esrc: errorentry::ESRC,
        eval: isize,
        http_url: String,
        msg: String,
        timestamp: i64,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let en = ErrorEntry {
            subs_id: subsid,
            e_src: esrc as isize,
            e_val: eval,
            text: msg.clone(),
            remote_address: http_url,
            date: timestamp,
            ..Default::default()
        };
        self.add_error_entry(&en)
    }

    fn add_error_entry(&self, en: &ErrorEntry) -> Result<i64, Box<dyn std::error::Error>> {
        self.ctx.insert(en, false).map_err(rusqlite_error_to_boxed)
    }

    #[allow(clippy::blocks_in_conditions)]
    pub fn get_by_subscription(&self, subs_id: isize) -> Vec<ErrorEntry> {
        let prepared = format!(
            "SELECT * FROM {} WHERE subs_id={}  ORDER BY date DESC ",
            ErrorEntry::table_name(),
            subs_id,
        );
        let mut list: Vec<ErrorEntry> = Vec::default();
        if let Ok(mut stmt) = (*self.get_connection()).lock().unwrap().prepare(&prepared) {
            match stmt.query_map([], |row| {
                list.push(ErrorEntry::from_row(row));
                Ok(())
            }) {
                Ok(mr) => {
                    mr.count(); // seems to be necessary
                }
                Err(e) => error!("{} {:?}", &prepared, e),
            }
        }
        list
    }

    pub fn get_last_entry(&self, subs_id: isize) -> Option<ErrorEntry> {
        let prepared = format!(
            "SELECT * FROM {} WHERE subs_id={}  ORDER BY date DESC LIMIT 1 ",
            ErrorEntry::table_name(),
            subs_id,
        );
        self.ctx.get_one(prepared)
    }

    pub fn get_all_stored_entries(&self) -> Vec<ErrorEntry> {
        self.ctx.get_all()
    }

    pub fn delete_by_index(&self, indices: &[isize]) -> usize {
        let joined = indices
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(
            "DELETE FROM {}   WHERE {} in  ( {} ) ",
            ErrorEntry::table_name(),
            ErrorEntry::index_column_name(),
            joined
        );
        self.ctx.execute(sql)
    }

    pub fn flush_dirty(&self) {
        if let Ok(mut lg) = self.errorlines_cache.lock() {
            if lg.len() > 0 {
                while let Some(entry) = lg.pop() {
                    let r = self.add_error_entry(&entry);
                    if r.is_err() {
                        error!("while storing error lines : {:?} {:?}", r.err(), &entry);
                    }
                }
            }
        }
    }

    pub fn db_vacuum(&self) -> usize {
        self.ctx.execute("VACUUM".to_string())
    }
}

//-------------------

impl Buildable for ErrorRepo {
    type Output = ErrorRepo;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let o_folder = conf.get(KEY_FOLDERNAME);
        if o_folder.is_none() {
            conf.dump();
            panic!("E-Repo config has no {KEY_FOLDERNAME} ");
        }
        let folder = o_folder.unwrap();
        if folder.is_empty() {
            error!("ErrorRepo: Folder empty!!");
        }
        ErrorRepo::new(&folder)
    }
}

impl StartupWithAppContext for ErrorRepo {
    fn startup(&mut self, ac: &AppContext) {
        self.ctx.create_table();
        let timer_r: Rc<RefCell<Timer>> = (*ac).get_rc::<Timer>().unwrap();
        let su_r = ac.get_rc::<ErrorRepo>().unwrap();
        {
            (*timer_r)
                .borrow_mut()
                .register(&TimerEvent::Timer10s, su_r.clone(), true);
            (*timer_r)
                .borrow_mut()
                .register(&TimerEvent::Shutdown, su_r, true);
        }
    }
}

impl TimerReceiver for ErrorRepo {
    fn trigger_mut(&mut self, event: &TimerEvent) {
        match event {
            TimerEvent::Timer10s => {
                self.flush_dirty();
            }
            TimerEvent::Shutdown => {
                self.flush_dirty();
                self.ctx.cache_flush();
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod t {

    use super::*;
    use crate::db::errorentry::ESRC;

    // RUST_BACKTRACE=1 cargo watch -s "cargo test  db::errors_repo::t::t_error_repo_store     --lib -- --exact --nocapture"
    #[test]
    fn t_error_repo_store() {
        let e_repo = ErrorRepo::new_in_mem(); //  ErrorRepo::new("../target/e_rep_store/");
        let _ = e_repo.ctx.delete_table();
        e_repo.ctx.create_table();
        let r0 = e_repo.add_error_ts(12, ESRC::None, 0, String::default(), String::from("E_0"), 2);
        assert!(r0.is_ok());
        let r1 = e_repo.add_error_ts(12, ESRC::None, 0, String::default(), String::from("E_1"), 3);
        assert!(r1.is_ok());
        e_repo.flush_dirty();
        let subs_list = e_repo.get_by_subscription(12);
        // println!(" {:?} ", subs_list);
        assert_eq!(subs_list.len(), 2);
        assert_eq!(subs_list.get(0).unwrap().err_id, 2);
        assert_eq!(subs_list.get(1).unwrap().err_id, 1);
    }

    //  cargo watch -s "(cd fr_core ; RUST_BACKTRACE=1 cargo test  db::errors_repo::t::t_error_repo_last   )"
    #[test]
    fn t_error_repo_last() {
        let e_repo = ErrorRepo::new_in_mem(); // ErrorRepo::new("../target/e_rep_last/");
        let _ = e_repo.ctx.delete_table();
        e_repo.ctx.create_table();
        let tnow = crate::util::timestamp_now();
        let _r = e_repo.add_error_ts(
            12,
            ESRC::None,
            0,
            String::default(),
            String::from("E_0"),
            tnow - 1,
        );
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _r = e_repo.add_error_ts(
            12,
            ESRC::None,
            0,
            String::default(),
            String::from("E_1"),
            tnow,
        );
        e_repo.flush_dirty();
        let last_one = e_repo.get_last_entry(12).unwrap();
        assert_eq!(last_one.err_id, 2);
        assert_eq!(last_one.date, tnow);
    }
}
