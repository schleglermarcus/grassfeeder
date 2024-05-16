use crate::controller::timer::Timer;
use crate::db::icon_row::CompressionType;
use crate::db::icon_row::IconRow;
use crate::db::sqlite_context::rusqlite_error_to_boxed;
use crate::db::sqlite_context::SqliteContext;
use crate::db::sqlite_context::TableInfo;
use crate::util::timestamp_now;
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

pub const KEY_FOLDERNAME: &str = "subscriptions_folder";
pub const FILENAME: &str = "icons_list.json";

pub trait IIconRepo {
    fn get_ctx(&self) -> &SqliteContext<IconRow>;

    fn get_by_icon(&self, icon_s: String) -> Vec<IconRow>;

    fn get_by_index(&self, icon_id: isize) -> Option<IconRow>;
    fn get_all_entries(&self) -> Vec<IconRow>;

    fn add_icon(
        &self,
        new_icon: String,
        http_date: i64,
        http_length: isize,
        http_url: String,
        compression: CompressionType,
    ) -> Result<i64, Box<dyn std::error::Error>>;

    fn store_icon(
        &self,
        icon_id: isize,
        new_icon: String,
        comp_type: CompressionType,
    ) -> Result<usize, Box<dyn std::error::Error>>;

    /// returns number of deleted rows
    fn delete_icon(&self, icon_id: isize) -> usize;

    fn create_table(&self);

    /// returns number of changed rows
    fn update_icon(
        &self,
        icon_id: isize,
        new_icon: Option<String>,
        comp_type: CompressionType,
    ) -> Result<usize, Box<dyn std::error::Error>>;
}

/*
#[deprecated]
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IconEntry {
    pub icon_id: isize,
    pub icon: String,
}


impl std::fmt::Debug for IconEntry {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("IconEntry")
            .field("id", &self.icon_id)
            .field("IC#", &self.icon.len())
            .finish()
    }
}
 */

pub struct IconRepo {
    ctx: SqliteContext<IconRow>,
}

impl IconRepo {
    pub fn new(folder_name: &str) -> Self {
        let fname = Self::filename(folder_name);
        match std::fs::create_dir_all(folder_name) {
            Ok(()) => (),
            Err(e) => {
                error!("IconRepo cannot create folder {} {:?}", folder_name, e);
            }
        }
        let sqctx = SqliteContext::new(&fname);
        IconRepo { ctx: sqctx }
    }

    #[deprecated]
    pub fn new_(folder_name: &str) -> Self {
        warn!("OLD  IconRepo to DB ");
        let fname = Self::filename(folder_name);
        match std::fs::create_dir_all(&fname) {
            Ok(()) => (),
            Err(e) => {
                error!("IconRepo cannot create folder {} {:?}", fname, e);
            }
        }
        IconRepo {
            ctx: SqliteContext::new_in_memory(),
        }
    }

    pub fn new_by_filename(filename: &str) -> Self {
        trace!("icon_repo::NEW  filename={} ", filename);
        let dbctx = SqliteContext::new(filename);
        IconRepo { ctx: dbctx }
    }

    pub fn new_in_mem() -> Self {
        let ir = IconRepo {
            ctx: SqliteContext::new_in_memory(),
        };
        ir.ctx.create_table();
        ir
    }

    pub fn new_by_connection(con: Arc<Mutex<Connection>>) -> Self {
        IconRepo {
            ctx: SqliteContext::new_by_connection(con),
        }
    }

    pub fn filename(foldername: &str) -> String {
        format!("{foldername}icons.db")
    }
}

//-------------------

impl IIconRepo for IconRepo {
    fn get_ctx(&self) -> &SqliteContext<IconRow> {
        &self.ctx
    }

    fn add_icon(
        &self,
        new_icon: String,
        http_date: i64,
        http_length: isize,
        http_url: String,
        compression: CompressionType,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let entry: IconRow = IconRow {
            icon: new_icon,
            web_date: http_date,
            web_size: http_length,
            web_url: http_url,
            compression_type: compression,
            req_date: timestamp_now(),
            ..Default::default()
        };
        debug!("icon_repo::ADD  {:?} ", entry);
        self.ctx
            .insert(&entry, false)
            .map_err(rusqlite_error_to_boxed)
    }

    fn get_by_icon(&self, icon_s: String) -> Vec<IconRow> {
        let sql = format!(
            "SELECT * FROM {} where icon=\"{}\" ",
            IconRow::table_name(),
            icon_s
        );
        self.ctx.get_list(sql)
    }

    fn get_by_index(&self, icon_id: isize) -> Option<IconRow> {
        self.ctx.get_by_index(icon_id)
    }

    fn get_all_entries(&self) -> Vec<IconRow> {
        self.ctx.get_all()
    }

    fn delete_icon(&self, icon_id: isize) -> usize {
        let sql = format!(
            "DELETE FROM {}  WHERE {}={} ",
            IconRow::table_name(),
            IconRow::index_column_name(),
            icon_id
        );
        self.ctx.execute(sql)
    }

    fn store_icon(
        &self,
        icon_id_: isize,
        new_icon: String,
        comp_type: CompressionType,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let entry: IconRow = IconRow {
            icon_id: icon_id_,
            icon: new_icon,
            web_date: 0,
            web_size: 0,
            web_url: String::default(),
            compression_type: comp_type,
            req_date: 0,
        };
        trace!(
            "icon_repo::store_icon: {} C{:?}",
            entry.icon_id,
            entry.compression_type
        );
        match self.ctx.insert(&entry, true) {
            Ok(r) => Result::Ok(r as usize),
            Err(e) => Result::Err(Box::new(e)),
        }
    }

    fn create_table(&self) {
        self.ctx.create_table();
    }

    fn update_icon(
        &self,
        icon_id: isize,
        new_icon: Option<String>,
        comp_type: CompressionType,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let mut up_icon: String = String::default();
        if new_icon.is_some() {
            up_icon = format!(" , icon =\"{}\" ", new_icon.unwrap());
        }
        let sql = format!(
            "UPDATE {}  SET  compression_type = {} {} WHERE icon_id = {}",
            IconRow::table_name(),
            comp_type as u8,
            up_icon,
            icon_id,
        );
        Ok(self.ctx.execute(sql))
    }
}

//-------------------

impl Buildable for IconRepo {
    type Output = IconRepo;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let o_folder = conf.get(KEY_FOLDERNAME);
        match o_folder {
            Some(folder) => IconRepo::new(&folder),
            None => {
                conf.dump();
                panic!("iconrepo config has no {KEY_FOLDERNAME} ");
            }
        }
    }
}

impl StartupWithAppContext for IconRepo {
    fn startup(&mut self, ac: &AppContext) {
        self.ctx.create_table();
        {
            let timer_r: Rc<RefCell<Timer>> = (*ac).get_rc::<Timer>().unwrap();
            let su_r = ac.get_rc::<IconRepo>().unwrap();
            {
                (*timer_r)
                    .borrow_mut()
                    .register(&TimerEvent::Shutdown, su_r, true);
            }
            //  self.startup_();
        }
    }
}

impl TimerReceiver for IconRepo {
    fn trigger_mut(&mut self, event: &TimerEvent) {
        if *event == TimerEvent::Shutdown {
            self.ctx.cache_flush();
        }
    }
}

#[cfg(test)]
mod t_ {
    use super::*;
    /*

    pub const TEST_FOLDER1: &'static str = "../target/db_t_ico_rep";

       // cargo watch -s "(cd fr_core ;  RUST_BACKTRACE=1  cargo test  db::icon_repo::t_::t_store_file   --lib -- --exact --nocapture  )  "
       #[test]
       fn t_store_file() {
           setup();
           {
               let ir = IconRepo::new_in_mem(); // IconRepo::new_(TEST_FOLDER1);
                                                // iconrepo.startup_();
                                                // iconrepo.clear();

               let r_ir: Rc<dyn IIconRepo> = Rc::new(ir);
               let s1 = IconEntry::default();
               assert!((*r_ir).store_entry(&s1).is_ok());
               assert!((*r_ir).store_entry(&s1).is_ok());
               let list = (*r_ir).get_all_entries_();
               assert_eq!(list.len(), 2);
               //iconrepo.check_or_store();
           }
           {
               let mut sr = IconRepo::new_(TEST_FOLDER1);
               sr.startup_();
               let list = sr.get_all_entries_();
               assert_eq!(list.len(), 2);
           }
       }
    */

    // cargo watch -s "(cd fr_core ;  RUST_BACKTRACE=1  cargo test  db::icon_repo::t_::t_db_store   --lib -- --exact --nocapture  )  "
    #[test]
    fn t_db_store() {
        setup();
        let ir = IconRepo::new_in_mem();
        let r_ir: Rc<dyn IIconRepo> = Rc::new(ir);
        let r = (*r_ir).add_icon(
            "hello".to_string(),
            0,
            0,
            "".to_string(),
            CompressionType::None,
        );
        // debug!("R: {:?} ", r);
        assert!(r.is_ok());
        let r2 = (*r_ir).get_by_index(r.unwrap() as isize);
        assert!(r2.is_some());
        assert_eq!("hello", r2.unwrap().icon.as_str());
    }

    // dummy instead of log configuration
    fn setup() {}
}
