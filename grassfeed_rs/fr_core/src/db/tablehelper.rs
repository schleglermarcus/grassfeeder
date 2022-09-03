use gluesql::core::data::value::Value;
use gluesql::memory_storage::Key;
use gluesql::prelude::*;
use sled::IVec;
use std::io::Write;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

pub type GlueExecType = Arc<RwLock<dyn GlueExec + Send + Sync + 'static>>;

pub trait GlueExec {
    fn query(&mut self, statement: &str) -> Result<Payload, Box<dyn std::error::Error>>;
    fn query_tx(&mut self, stm_list: &[String]) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct GlueExecSled {
    glue_sled: Mutex<Glue<IVec, SledStorage>>,
}
impl GlueExecSled {
    pub fn new(filename: String) -> Self {
        let storage = SledStorage::new(&filename).unwrap();
        GlueExecSled {
            glue_sled: Mutex::new(Glue::new(storage)),
        }
    }
}

impl GlueExec for GlueExecSled {
    fn query(&mut self, statement: &str) -> Result<Payload, Box<dyn std::error::Error>> {
        let r = self.glue_sled.lock().unwrap().execute(statement);
        r.map_err(glue_error_to_boxed)
    }

    fn query_tx(&mut self, stm_list: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let t_name: String = std::thread::current().name().unwrap().to_string();
        if let Ok(mut g_sled_e) = self.glue_sled.lock() {
            if false {
                let complete = format!("BEGIN;        {} COMMIT;", stm_list.join(" "));
                trace!("{}:{} BEGIN:", t_name, stm_list.len());
                match g_sled_e.execute(&complete) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("{}  rolling back due to:   {:?}", &t_name, e);
                        let _r = g_sled_e.execute("ROLLBACK; ");
                        return Err(glue_error_to_boxed(e));
                    }
                }
                trace!("{}:{}  committed ", t_name, stm_list.len());
            } else {
                g_sled_e.execute("BEGIN; ")?;
                for stm in stm_list {
                    g_sled_e.execute(stm)?;
                }
                g_sled_e.execute("COMMIT; ")?;
                trace!("   STOP transaction by {:?}", t_name);
            }
        }
        Ok(())
    }
}

pub struct GlueExecMemory {
    glue_mem: Glue<Key, MemoryStorage>,
}
impl GlueExecMemory {
    pub fn new() -> Self {
        let storage = MemoryStorage::default();
        GlueExecMemory {
            glue_mem: Glue::new(storage),
        }
    }
}

impl Default for GlueExecMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl GlueExec for GlueExecMemory {
    fn query(&mut self, statement: &str) -> Result<Payload, Box<dyn std::error::Error>> {
        let r = self.glue_mem.execute(statement);
        r.map_err(glue_error_to_boxed)
    }

    fn query_tx(&mut self, stm_list: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        self.glue_mem.execute("BEGIN; ")?;
        for stm in stm_list {
            match self.glue_mem.execute(stm) {
                Ok(_) => (),
                Err(e) => {
                    error!("rolling back due to:  {:?} {:?}", &stm, e);
                    let _r = self.glue_mem.execute("ROLLBACK; ");
                    return Err(glue_error_to_boxed(e));
                }
            }
        }
        self.glue_mem.execute("COMMIT; ")?;
        Ok(())
    }
}

pub trait TableInfo {
    fn to_insert_values(obj: &Self) -> (String, String);
    fn from_values(labels: &[String], vals: &[Value]) -> Self;
    fn index_column_name() -> String;
    fn table_name() -> String;
    fn index(&self) -> isize;
    fn set_index(&mut self, newindex: isize);
    fn create_string() -> String;
    fn start_index() -> isize {
        1
    }
    fn index_create_snip() -> Option<(String, String, String)>;
}

// #[automock]
pub trait ITableHelper<T> {
    fn create_test_table(&mut self);
    fn table_name(&self) -> String;
    fn index_column_name(&self) -> String;
    fn create_table(&self);
    fn drop_table(&self);
    fn query(&self, statement: &str) -> Result<Payload, Box<dyn std::error::Error>>;
    fn query_tx(&self, stm_list: &[String]) -> Result<(), Box<dyn std::error::Error>>;
    fn table_exists(&self) -> bool;
    fn create_table_if_not_exists(&self);
    fn get_max_index(&self) -> usize;
    fn store_entry(&self, entry: &T) -> Result<T, Box<dyn std::error::Error>>;
    fn store_entries(&self, e_list: &[T]) -> Result<(), Box<dyn std::error::Error>>;
    fn get_all_entries(&self) -> Vec<T>;
    fn get_by_index(&self, indexvalue: isize) -> Option<T>;
    fn get_glue_exec(&self) -> GlueExecType;
}

#[derive(Clone)]
pub struct TableHelper<T> {
    pub storage_location: String,
    pub _info: PhantomData<T>,
    pub glue_exec: GlueExecType,
    //	pub max_index : AtomicUsize,
}

impl<T: std::fmt::Debug> TableHelper<T> {
    fn dump_sql_error(&self, list: &[T], sql: &[String], errorstr: &String) {
        let mut file = std::fs::File::create("../target/sql-error.txt")
            .expect("cannot write to sql-error.txt ");
        let _r = file.write_all(errorstr.as_bytes());
        let _r = file.write_all("\n".as_bytes());
        sql.iter().for_each(|s| {
            let _r = file.write_all(s.as_bytes());
            let _r = file.write_all("\n".as_bytes());
        });
        let _r = file.write_all("\n".as_bytes());
        list.iter().for_each(|l| {
            let _r = file.write_all(format!("{:?}", l).as_bytes());
            let _r = file.write_all("\n".as_bytes());
        });
    }
}

impl<T: TableInfo + Clone + std::fmt::Debug> TableHelper<T> {
    pub fn new(filename: String) -> Self {
        let g_e_t: GlueExecType = Arc::new(RwLock::new(GlueExecSled::new(filename.clone())));
        TableHelper {
            storage_location: filename,
            _info: PhantomData,
            glue_exec: g_e_t,
        }
    }

    pub fn clone_from(other_glue_x: GlueExecType) -> Self {
        TableHelper {
            _info: PhantomData,
            storage_location: String::default(),
            glue_exec: other_glue_x.clone(),
        }
    }

    pub fn new_with_memorystore() -> Self {
        let g_e_t: GlueExecType = Arc::new(RwLock::new(GlueExecMemory::new()));
        TableHelper {
            _info: PhantomData,
            storage_location: String::default(),
            glue_exec: g_e_t,
        }
    }

    pub fn index_column_name() -> String {
        T::index_column_name()
    }
    pub fn table_name() -> String {
        T::table_name()
    }
    pub fn create_string() -> String {
        T::create_string()
    }

    pub fn index_create_snip() -> Option<(String, String, String)> {
        T::index_create_snip()
    }

    pub fn get_by_index(&self, indexvalue: isize) -> Option<T> {
        let mut ret: Option<T> = None;
        let tn = TableHelper::<T>::table_name();
        let icn = TableHelper::<T>::index_column_name();
        let o_ind_cre_sni = TableHelper::<T>::index_create_snip();
        let index_snip = match o_ind_cre_sni {
            Some((_cre, with1, _with2)) => with1,
            None => String::default(),
        };
        let stm = format!(
            "SELECT *  from {} {}  where {} = {} ; ",
            tn, index_snip, icn, indexvalue
        );
        let r = self.query(&stm);
        if r.is_err() {
            return ret;
        }
        let payload = r.unwrap();
        if let Payload::Select { rows, labels } = payload {
            if rows.is_empty() {
                return ret;
            };
            let entry = T::from_values(&labels, &rows[0]);
            ret = Some(entry);
        }
        ret
    }

    pub fn drop_table(&self) {
        let tn = TableHelper::<T>::table_name();
        let stm = format!("DROP TABLE if exists {}; ", &tn);
        match self.query(&stm) {
            Ok(_) => (),
            Err(e) => error!("error={:?} {} ", stm, e),
        }
    }

    pub fn create_table(&self) {
        let tn = TableHelper::<T>::table_name();
        let stm = format!("CREATE TABLE {}  ( {} ) ; ", tn, T::create_string());
        match self.query(&stm) {
            Ok(_) => (),
            Err(e) => {
                error!("create_table() {} :  {} ", tn, e);
            }
        }
        if let Some((ind_crea, _with1, _with2)) = T::index_create_snip() {
            //            let (ind_crea, _with1, _with2) = c_i.unwrap();
            match self.query(&ind_crea) {
                Ok(_) => (),
                Err(e) => {
                    error!("create_index() {} :  {} \t {}", tn, e, ind_crea);
                }
            }
        }
    }

    pub fn get_all_entries(&self) -> Vec<T> {
        let mut ret: Vec<T> = Vec::new();
        let tn = TableHelper::<T>::table_name();
        let stm = format!("SELECT *  from {}  ; ", tn);
        let r = self.query(&stm);
        if r.is_err() {
            return ret;
        }
        let payload = r.unwrap();
        if let Payload::Select { rows, labels } = payload {
            ret = rows
                .iter()
                .map(|row| T::from_values(&labels, row))
                .collect::<Vec<T>>();
        }
        ret
    }

    pub fn get_max_index(&self) -> usize {
        let tn = TableHelper::<T>::table_name();
        let stm = format!(
            "SELECT MAX({})  from {}  ; ",
            TableHelper::<T>::index_column_name(),
            tn
        );
        let r = self.query(&stm);
        if r.is_err() {
            return 0;
        }
        let payload = r.unwrap();
        if let Payload::Select { rows, .. } = payload {
            if rows.is_empty() {
                return 0;
            }
            let val = rows[0].get(0);
            if let Some(Value::I64(i)) = val {
                return *i as usize;
            }
        }
        0
    }

    pub fn store_entry(&self, entry: &T) -> Result<T, Box<dyn std::error::Error>> {
        let tn = TableHelper::<T>::table_name();

        let mut ret: T = (*entry).clone();
        if ret.index() <= 0 {
            let existing_max_index = (self.get_max_index() + 1) as isize;
            let new_index = std::cmp::max(existing_max_index, T::start_index());
            ret.set_index(new_index);
        }

        let (namestring, valuestring) = T::to_insert_values(&ret);
        let stm = format!(
            "INSERT INTO  {}  ( {} ) VALUES ( {} ) ; ",
            &tn, namestring, valuestring
        );
        match self.query(&stm) {
            Ok(payload) => {
                if let Payload::Select { rows, labels } = payload {
                    let r_entry = T::from_values(&labels, &rows[0]);
                    return Ok(r_entry);
                }
                // debug!("store_entry: No payload for {}", stm);
                Ok(ret)
            }
            Err(e) => {
                let t_name: String = std::thread::current().name().unwrap().to_string();
                error!("store_entry {}: {} {:?} \t {}", t_name, tn, e, stm);
                Err(e)
            }
        }
    }

    pub fn store_entries(&self, e_list: &Vec<T>) -> Result<(), Box<dyn std::error::Error>> {
        let tn = TableHelper::<T>::table_name();
        let mut stm_list: Vec<String> = Vec::default();
        let mut start_index = self.get_max_index() + 1;
        for entry in e_list {
            let mut e = entry.clone();
            if e.index() <= 0 {
                e.set_index(start_index as isize);
            }
            let (namestring, valuestring) = T::to_insert_values(&e);
            let stm = format!(
                "INSERT INTO  {}  ( {} ) VALUES ( {} ) ; ",
                &tn, namestring, valuestring
            );
            stm_list.push(stm);
            start_index += 1;
        }
        match self.query_tx(&stm_list) {
            Ok(_) => Ok(()),
            Err(e) => {
                let t_name: String = std::thread::current().name().unwrap().to_string();
                error!(
                    "store_entries {}:{}:{}  #stm={:?}",
                    t_name,
                    stm_list.len(),
                    tn,
                    e,
                );
                self.dump_sql_error(e_list, &stm_list, &e.to_string());
                Err(e)
            }
        }
    }

    pub fn delete_by_index(&self, indexvalue: isize) {
        let tn = TableHelper::<T>::table_name();
        let icn = TableHelper::<T>::index_column_name();
        let stm = format!("DELETE  from {}  where {} = {} ; ", tn, icn, indexvalue);
        let r = self.query(&stm);
        if r.is_err() {
            error!("delete_by_index {:?}  =>{:?}", stm, r.err());
        }
    }
}

#[derive(Debug)]
struct QueryError {}
impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "QueryError")
    }
}

impl std::error::Error for QueryError {}

impl<T: TableInfo + Clone + std::fmt::Debug> ITableHelper<T> for TableHelper<T> {
    fn index_column_name(&self) -> String {
        T::index_column_name()
    }
    fn table_name(&self) -> String {
        T::table_name()
    }

    fn create_table(&self) {
        self.create_table()
    }

    fn drop_table(&self) {
        self.drop_table()
    }

    fn create_test_table(&mut self) {
        let mut err_happened = false;
        let tn = "write_test_table";
        let stm = format!("DROP TABLE if exists {}; ", &tn);
        match self.query(&stm) {
            Ok(_) => {}
            Err(e) => {
                warn!(" droppping test table : {:?}", e);
                err_happened = true;
            }
        }
        let stm = format!("CREATE TABLE {}  ( first INTEGER ) ; ", &tn);
        match self.query(&stm) {
            Ok(_) => {}
            Err(e) => {
                warn!(" creating test table : {:?}", e);
                err_happened = true;
            }
        }
        if err_happened {
            let old_storage = format!("{}.old", self.storage_location);
            let r = std::fs::remove_dir_all(&old_storage);
            debug!(" removed {} {:?} ", &old_storage, r);
            let r = std::fs::rename(&self.storage_location, &old_storage);
            debug!(" renaming  to  {} {:?} ", &old_storage, r);
            let new_glue_exec_sled = GlueExecSled::new(self.storage_location.clone());
            let new_glue_x = Arc::new(RwLock::new(new_glue_exec_sled));
            let entries = self.get_all_entries();
            debug!(" copying elements  {} {}  ", entries.len(), T::table_name());
            for e in entries {
                let (namestring, valuestring) = T::to_insert_values(&e);
                let stm = format!(
                    "INSERT INTO  {}  ( {} ) VALUES ( {} ) ; ",
                    &tn, namestring, valuestring
                );
                let _r = (*new_glue_x).write().unwrap().query(&stm);
            }
            drop((*self.glue_exec).write().unwrap());
            self.glue_exec = new_glue_x;
        }
    }

    fn query(&self, statement: &str) -> Result<Payload, Box<dyn std::error::Error>> {
        let r = (*self.glue_exec).write().unwrap().query(statement);
        r
    }

    fn query_tx(&self, stm_list: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let r = (*self.glue_exec).write().unwrap().query_tx(stm_list);
        r
    }

    fn table_exists(&self) -> bool {
        let tn = TableHelper::<T>::table_name();
        let stm = format!("select * from {}  LIMIT 1 ; ", tn);
        let r = self.query(&stm);
        if let Ok(Payload::Select { labels, .. }) = r {
            if !labels.is_empty() {
                return true;
            }
        }
        false
    }

    fn create_table_if_not_exists(&self) {
        if !self.table_exists() {
            self.create_table();
        }
    }

    fn get_max_index(&self) -> usize {
        self.get_max_index()
    }

    fn store_entry(&self, entry: &T) -> Result<T, Box<dyn std::error::Error>> {
        self.store_entry(entry)
    }

    fn store_entries(&self, e_list: &[T]) -> Result<(), Box<dyn std::error::Error>> {
        self.store_entries(&e_list.to_vec())
    }

    fn get_all_entries(&self) -> Vec<T> {
        self.get_all_entries()
    }

    fn get_by_index(&self, indexvalue: isize) -> Option<T> {
        self.get_by_index(indexvalue)
    }

    fn get_glue_exec(&self) -> GlueExecType {
        self.glue_exec.clone()
    }
}

pub fn isize_from_value_or(val_o: Option<&Value>, or_value: isize) -> isize {
    match val_o {
        Some(Value::I64(i)) => *i as isize,
        _ => or_value,
    }
}
pub fn usize_from_value_or(val_o: Option<&Value>, or_value: usize) -> usize {
    match val_o {
        Some(Value::I64(i)) => *i as usize,
        _ => or_value,
    }
}

pub fn string_from_value_or(val_o: Option<&Value>, or_value: String) -> String {
    match val_o {
        Some(Value::Str(s)) => s.to_string(),
        _ => or_value,
    }
}
pub fn bool_from_value_or(val_o: Option<&Value>, or_value: bool) -> bool {
    match val_o {
        Some(Value::Bool(i)) => *i as bool,
        _ => or_value,
    }
}

pub fn i64_from_value_or(val_o: Option<&Value>, or_value: i64) -> i64 {
    match val_o {
        Some(Value::I64(i)) => *i as i64,
        _ => or_value,
    }
}

pub fn u64_from_value_or(val_o: Option<&Value>, or_value: u64) -> u64 {
    match val_o {
        Some(Value::I64(i)) => *i as u64,
        _ => or_value,
    }
}

fn glue_error_to_boxed(e: gluesql::core::result::Error) -> Box<dyn std::error::Error> {
    Box::new(e)
}
