use crate::util::file_exists;
use rusqlite::params_from_iter;
use rusqlite::Connection;
use rusqlite::ParamsFromIter;
use rusqlite::ToSql;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

pub trait TableInfo {
    fn table_name() -> String;
    fn create_string() -> String;
    fn index_column_name() -> String;
    fn create_indices() -> Vec<String>;

    /// without index column
    fn get_insert_columns(&self) -> Vec<String>;
    /// without index column
    fn get_insert_values(&self) -> Vec<Wrap>;
    fn from_row(row: &rusqlite::Row) -> Self;
    fn get_index_value(&self) -> isize;
}

pub struct SqliteContext<T>
where
    T: TableInfo,
{
    connection: Arc<Mutex<Connection>>,
    _p: PhantomData<T>,
    db_file_existed_before: bool,
}

impl<T: TableInfo> SqliteContext<T> {
    pub fn new(filenam: &str) -> Self {
        let existed = file_exists(filenam);
        let c = Connection::open(filenam).unwrap();
        SqliteContext {
            connection: Arc::new(Mutex::new(c)),
            _p: PhantomData,
            db_file_existed_before: existed,
        }
    }

    pub fn new_by_connection(con: Arc<Mutex<Connection>>) -> Self {
        SqliteContext {
            connection: con,
            _p: PhantomData,
            db_file_existed_before: false,
        }
    }

    pub fn new_in_memory() -> Self {
        SqliteContext {
            connection: Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            _p: PhantomData,
            db_file_existed_before: false,
        }
    }

    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        self.connection.clone()
    }
    pub fn delete_table(&self) -> rusqlite::Result<usize> {
        let stm = format!("DROP TABLE {} ", T::table_name(),);
        (*self.connection).lock().unwrap().execute(&stm, [])
    }

    pub fn create_table(&self) {
        let create_str = format!(
            "CREATE TABLE  IF NOT EXISTS   {} ( {} )",
            T::table_name(),
            T::create_string()
        );
        let _num_rows = self.execute(create_str);
        for cr_idx in T::create_indices() {
            self.execute(cr_idx);
        }
    }

    // On success, returns the number of rows that were changed
    pub fn execute(&self, sql: String) -> usize {
        match (*self.connection).lock().unwrap().execute(&sql, []) {
            Ok(num) => num,
            Err(e) => {
                error!("SqliteContext.execute  {:?}  {:?}", sql, e);
                0
            }
        }
    }

    /// inserts without primary column.  subs_id is auto-imcremented by sqlite
    /// returns index value
    pub fn insert(&self, entry: &T, with_index: bool) -> Result<i64, rusqlite::Error> {
        let mut col_names: Vec<String> = Vec::default();
        let mut wrap_vec: Vec<Wrap> = Vec::default();
        if with_index {
            col_names.push(T::index_column_name());
            wrap_vec.push(Wrap::INT(entry.get_index_value()));
        }
        col_names.extend(entry.get_insert_columns());
        wrap_vec.extend(entry.get_insert_values());

        let questionmarks = vec!["?"; col_names.len()].to_vec().join(", ");
        let prepared = format!(
            "INSERT INTO {} ( {} ) VALUES ( {} )",
            T::table_name(),
            col_names.join(", "),
            questionmarks
        );
        let vec_dyn_tosql: Vec<&dyn ToSql> = wrap_vec
            .iter()
            .map(|w| w.to_dyn_tosql())
            .collect::<Vec<&dyn ToSql>>();
        let params_fi: ParamsFromIter<&Vec<&dyn ToSql>> = params_from_iter(&vec_dyn_tosql);
        match (*self.connection).lock().unwrap().prepare(&prepared) {
            Ok(mut stmt) => stmt.insert(params_fi),
            Err(e) => {
                warn!("insert: {:?}  idx={} ", &e, &entry.get_index_value());
                Err(e)
            }
        }
    }

    pub fn insert_tx(&self, list: &[T]) -> Result<i64, rusqlite::Error> {
        if list.is_empty() {
            return Ok(0);
        }
        let e0 = list.first().unwrap();
        let col_names = e0.get_insert_columns();
        let questionmarks = vec!["?"; col_names.len()].to_vec().join(", ");
        let prep_sql = format!(
            "INSERT INTO {} ( {} ) VALUES ( {} )",
            T::table_name(),
            col_names.join(", "),
            questionmarks
        );
        let mut conn = (*self.connection).lock().unwrap();
        let tx = conn.transaction().unwrap();
        let mut num_success: i64 = 0;
        {
            let mut stmt = tx.prepare_cached(&prep_sql).unwrap();
            list.iter().for_each(|e| {
                let wrap_vec: Vec<Wrap> = e.get_insert_values();
                let vec_dyn_tosql: Vec<&dyn ToSql> = wrap_vec
                    .iter()
                    .map(|w| w.to_dyn_tosql())
                    .collect::<Vec<&dyn ToSql>>();
                let params_fi: ParamsFromIter<&Vec<&dyn ToSql>> = params_from_iter(&vec_dyn_tosql);
                match stmt.execute(params_fi) {
                    Ok(num) => num_success += num as i64,
                    Err(e) => {
                        error!("{} => {:?}", prep_sql, e)
                    }
                };
            });
        }
        match tx.commit() {
            Ok(_) => Ok(num_success),
            Err(e) => Err(e),
        }
    }

    #[allow(clippy::blocks_in_conditions)]
    pub fn get_one(&self, sql: String) -> Option<T> {
        let mut ret: Option<T> = None;
        if let Ok(mut stmt) = (*self.connection).lock().unwrap().prepare(&sql) {
            match stmt.query_map([], |row| {
                ret = Some(T::from_row(row));
                Ok(())
            }) {
                Ok(mr) => {
                    mr.count(); // seems to be necessary
                }
                Err(e) => error!("{} {:?}", &sql, e),
            }
        }
        ret
    }

    pub fn get_by_index(&self, index: isize) -> Option<T> {
        let prepared = format!(
            "SELECT * FROM {} WHERE {}={}",
            T::table_name(),
            T::index_column_name(),
            index
        );
        self.get_one(prepared)
    }

    pub fn get_all(&self) -> Vec<T> {
        let prepared = format!("SELECT * FROM {} ", T::table_name(),);
        self.get_list(prepared)
    }

    #[allow(clippy::blocks_in_conditions)]
    pub fn get_list(&self, sql: String) -> Vec<T> {
        let mut list: Vec<T> = Vec::default();
        if let Ok(mut stmt) = (*self.connection).lock().unwrap().prepare(&sql) {
            match stmt.query_map([], |row| {
                list.push(T::from_row(row));
                Ok(())
            }) {
                Ok(mr) => {
                    mr.count(); // does nothing. seeems to be necessary
                }
                Err(e) => error!("{} {:?}", &sql, e),
            }
        }
        list
    }

    pub fn count_all(&self) -> isize {
        let prepared = format!(
            "SELECT COUNT({}) FROM {} ",
            T::index_column_name(),
            T::table_name()
        );
        self.one_number(prepared)
    }

    /// return -1 on errors
    pub fn one_number(&self, sql: String) -> isize {
        let o_conn = (*self.connection).lock();
        if o_conn.is_err() {
            error!("count_all() NO connection! {:?}", o_conn.err());
            return -1;
        }
        let conn = o_conn.unwrap();
        let r_stmt = conn.prepare(&sql);
        if r_stmt.is_err() {
            error!("statement {} {:?}", sql, r_stmt.err());
            return -1;
        }
        let mut stmt = r_stmt.unwrap();

        let r_rows = stmt.query([]);
        if r_rows.is_err() {
            error!("query {:?}", r_rows.err());
            return -1;
        }
        let mut rows = r_rows.unwrap();
        let r_row = rows.next();
        if r_row.is_err() {
            error!("no_row! {:?}", r_row.err());
            return -1;
        }
        let o_row = r_row.unwrap();
        if o_row.is_none() {
            error!("row_empty! ");
            return -1;
        }
        let row = o_row.unwrap();
        if let Ok(col0) = row.get(0) {
            return col0;
        }
        -1
    }

    ///  if cache_flush  is not there, rusqlite+bundled is missing
    pub fn cache_flush(&self) {
        let m_c: MutexGuard<Connection> = (*self.connection).lock().unwrap();
        let r = (*m_c).cache_flush();
        if r.is_err() {
            warn!("cache_flush {:?}", r.err());
        }
    }

    pub fn db_existed_before(&self) -> bool {
        self.db_file_existed_before
    }

    pub fn is_column_present(&self, column_name: &str) -> bool {
        let stm = format!("select {} from {}", column_name, T::table_name());
        let mut errmsg: String = String::default();
        match (*self.connection).lock().unwrap().execute(&stm, []) {
            Ok(_) => (),
            Err(e) => errmsg = e.to_string(),
        }
        if errmsg.contains("no such column") {
            debug!("{}   {:?}", stm, errmsg);
            return false;
        }
        true
    }

    pub fn add_column(&self, column_name: &str, column_type: &str) -> usize {
        let stm = format!(
            "ALTER TABLE   {} ADD COLUMN {} {}",
            T::table_name(),
            column_name,
            column_type
        );
        match (*self.connection).lock().unwrap().execute(&stm, []) {
            Ok(n) => return n,
            Err(e) => warn!(" {:?} {:?}", stm, e),
        }
        0
    }
}

#[derive(Debug)]
pub enum Wrap {
    INT(isize),
    I64(i64),
    STR(String),
    BOO(bool),
    BLOB(Vec<u8>),
    U64(u64),
}

impl Wrap {
    fn to_dyn_tosql(&self) -> &dyn ToSql {
        match self {
            Wrap::INT(i) => i,
            Wrap::I64(i) => i,
            Wrap::U64(u) => u,
            Wrap::STR(s) => s,
            Wrap::BLOB(v) => v,
            Wrap::BOO(b) => b,
        }
    }
}

pub fn rusqlite_error_to_boxed(e: rusqlite::Error) -> Box<dyn std::error::Error> {
    Box::new(e)
}
