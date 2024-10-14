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
use rusqlite::params_from_iter;
use rusqlite::Connection;
use rusqlite::ParamsFromIter;
use rusqlite::ToSql;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

use super::sqlite_context::Wrap;

pub const KEY_FOLDERNAME: &str = "subscriptions_folder";
pub const FILENAME: &str = "icons_list.json";

pub trait IIconRepo {
    fn get_ctx(&self) -> &SqliteContext<IconRow>;
    fn get_all_entries(&self) -> Vec<IconRow>;
    fn get_by_icon(&self, icon_s: String) -> Vec<IconRow>;
    fn get_by_index(&self, icon_id: isize) -> Option<IconRow>;
    fn get_by_web_url(&self, url: &str) -> Vec<IconRow>;

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

    fn delete_icons(&self, ids: Vec<u8>) -> usize;

    /// returns number of tables created
    fn create_table(&self) -> usize;

    /// returns number of changed rows
    fn update_icon_content(
        &self,
        icon_id: isize,
        new_icon: Option<String>,
        comp_type: CompressionType,
    ) -> Result<usize, Box<dyn std::error::Error>>;

    fn store_icons_tx(
        &self,
        list: Vec<(isize, String)>,
        comp_type: CompressionType,
    ) -> Result<usize, Box<dyn std::error::Error>>;

    fn update_icon_webdate(
        &self,
        icon_id: isize,
        web_date: i64,
    ) -> Result<usize, Box<dyn std::error::Error>>;
}

pub struct IconRepo {
    ctx: SqliteContext<IconRow>,
}

impl IconRepo {
    pub fn new(folder_name: &str) -> Self {
        assert!(folder_name.ends_with('/'));
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

    pub fn new_by_filename(filename: &str) -> Self {
        // trace!("icon_repo::NEW  filename={} ", filename);
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

    fn get_by_web_url(&self, url: &str) -> Vec<IconRow> {
        let sql = format!(
            "SELECT * FROM {} where web_url=\"{}\" ",
            IconRow::table_name(),
            url
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
        //  trace!(            "icon_repo::store_icon: {} C{:?}",            entry.icon_id,            entry.compression_type        );
        match self.ctx.insert(&entry, true) {
            Ok(r) => Result::Ok(r as usize),
            Err(e) => Result::Err(Box::new(e)),
        }
    }

    fn create_table(&self) -> usize {
        self.ctx.create_table()
    }

    fn update_icon_content(
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

    fn update_icon_webdate(
        &self,
        icon_id: isize,
        web_date: i64,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let sql = format!(
            "UPDATE {}  SET  web_date = {} WHERE icon_id = {}",
            IconRow::table_name(),
            web_date,
            icon_id,
        );
        Ok(self.ctx.execute(sql))
    }

    fn delete_icons(&self, indices: Vec<u8>) -> usize {
        let joined = indices
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(
            "DELETE FROM {}   WHERE {} in  ( {} ) ",
            IconRow::table_name(),
            IconRow::index_column_name(),
            joined
        );
        self.ctx.execute(sql)
    }

    fn store_icons_tx(
        &self,
        list: Vec<(isize, String)>,
        comp_type: CompressionType,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let prep_sql = format!(
            "INSERT INTO {} ( {} ) VALUES ( {} )",
            IconRow::table_name(),
            " icon_id, compression_type,  icon ",
            " ?, ?, ? "
        );
        let conn = self.ctx.get_connection();
        let mut locked_conn = conn.lock().unwrap();
        let tx = locked_conn.transaction().unwrap();
        let mut num_success: usize = 0;
        for (id, content) in list {
            let wrap_vec: Vec<Wrap> = [
                Wrap::INT(id),
                Wrap::INT(comp_type.clone() as isize),
                Wrap::STR(content),
            ]
            .to_vec();
            let vec_dyn_tosql: Vec<&dyn ToSql> = wrap_vec
                .iter()
                .map(|w| w.to_dyn_tosql())
                .collect::<Vec<&dyn ToSql>>();
            let params_fi: ParamsFromIter<&Vec<&dyn ToSql>> = params_from_iter(&vec_dyn_tosql);
            let mut stmt = tx.prepare_cached(&prep_sql).unwrap();
            match stmt.execute(params_fi) {
                Ok(num) => num_success += num,
                Err(e) => {
                    error!("{} => {:?}", prep_sql, e)
                }
            };
        }
        match tx.commit() {
            Ok(_) => Ok(num_success),
            Err(e) => Err(Box::new(e)),
        }
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
        assert!(r.is_ok());
        let r2 = (*r_ir).get_by_index(r.unwrap() as isize);
        assert!(r2.is_some());
        assert_eq!("hello", r2.unwrap().icon.as_str());
    }

    // dummy instead of log configuration
    fn setup() {}
}
