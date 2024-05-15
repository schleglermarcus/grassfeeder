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
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

pub const KEY_FOLDERNAME: &str = "subscriptions_folder";
pub const FILENAME: &str = "icons_list.json";

/// line to convert, line number
// pub const CONV_TO: &dyn Fn(String, usize) -> Option<IconEntry> = &json_to_icon_entry;
// pub const CONV_FROM: &dyn Fn(&IconEntry) -> Option<String> = &icon_entry_to_json;

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

///
/// List of  Icon
///
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

pub struct IconRepo {
    // filename: String,
    ///  ID -> Entry
    // list: Arc<RwLock<HashMap<isize, IconEntry>>>,
    // last_list_count: usize,
    ctx: SqliteContext<IconRow>,
}

impl IconRepo {
    /// with DB
    pub fn new(folder_name: &str) -> Self {
        let fname = Self::filename(folder_name);
        match std::fs::create_dir_all(&folder_name) {
            Ok(()) => (),
            Err(e) => {
                error!("IconRepo cannot create folder {} {:?}", folder_name, e);
            }
        }
        let sqctx = SqliteContext::new(&fname);
        IconRepo {
            // list: Arc::new(RwLock::new(HashMap::new())),
            ctx: sqctx,
            //            filename: fname,
            // last_list_count: 0,
        }
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
            // list: Arc::new(RwLock::new(HashMap::new())),
            // filename: folder_name.to_string(),
            // last_list_count: 0,
            ctx: SqliteContext::new_in_memory(),
        }
    }

    /*
       #[deprecated]
       pub fn by_existing_list_(existing: Arc<RwLock<HashMap<isize, IconEntry>>>) -> Self {
           debug!("OLD__by_existing_list ");
           IconRepo {
               // list: existing,
               // filename: String::default(),
               // last_list_count: 0,
               ctx: SqliteContext::new_in_memory(),
           }
       }
    */

    pub fn new_by_filename(filename: &str) -> Self {
        trace!("icon_repo::NEW  filename={} ", filename);
        let dbctx = SqliteContext::new(filename);
        IconRepo {
            ctx: dbctx,
            // filename: String::default(),
            // list: Arc::new(RwLock::new(HashMap::new())),
            // last_list_count: 0,
        }
    }

    pub fn new_in_mem() -> Self {
        let ir = IconRepo {
            ctx: SqliteContext::new_in_memory(),
            // filename: String::default(),
            // list: Arc::new(RwLock::new(HashMap::new())),
            // last_list_count: 0,
        };
        ir.ctx.create_table();
        ir
    }

    pub fn new_by_connection(con: Arc<Mutex<Connection>>) -> Self {
        IconRepo {
            ctx: SqliteContext::new_by_connection(con),
            // filename: String::default(),
            // list: Arc::new(RwLock::new(HashMap::new())),
            // last_list_count: 0,
        }
    }

    pub fn filename(foldername: &str) -> String {
        format!("{foldername}icons.db")
    }

    /*
       #[deprecated]
       fn startup_(&mut self) -> bool {
           debug!("CREATE_DIR : {}  ", &self.filename);
           match std::fs::create_dir_all(&self.filename) {
               Ok(()) => (),
               Err(e) => {
                   error!("IconRepo cannot create folder {} {:?}", &self.filename, e);
                   return false;
               }
           }
           let filename = format!("{}/{}", self.filename, FILENAME);
           debug!("filename= {}  ", filename);

           self.filename = filename;
           if std::path::Path::new(&self.filename).exists() {
               let slist = read_from(self.filename.clone(), CONV_TO);
               let mut hm = (*self.list).write().unwrap();
               slist.into_iter().for_each(|se| {
                   let id = se.icon_id;
                   hm.insert(id, se);
               });
           } else {
               debug!("icon list file not found: {}", &self.filename);
           }
           debug!("startup_  done");
           true
       }
    */

    /*

       // #[deprecated]
       fn clear(&self) {
           (*self.list).write().unwrap().clear();
       }

       #[deprecated(note = " use add_icon()  or store_icon() ")]
       fn store_entry(&self, entry: &IconEntry) -> Result<IconEntry, Box<dyn std::error::Error>> {
           let mut new_id = entry.icon_id;
           if new_id <= 0 {
               let max_id = match (*self.list).read().unwrap().keys().max() {
                   Some(id) => *id,
                   None => 9, // start value
               };
               new_id = max_id + 1;
           }
           let mut store_entry = entry.clone();
           store_entry.icon_id = new_id;
           (*self.list)
               .write()
               .unwrap()
               .insert(new_id, store_entry.clone());
           // trace!(            "icons: store_entry newID {}  len{}",            store_entry.icon_id,            store_entry.icon.len()        );
           Ok(store_entry)
       }

       fn get_list(&self) -> Arc<RwLock<HashMap<isize, IconEntry>>> {
           self.list.clone()
       }

       fn store_icon_(&mut self, icon_id_: isize, new_icon: String) {
           info!("icon_repo::store_icon: {} ", icon_id_);
           (*self.list).write().unwrap().insert(
               icon_id_,
               IconEntry {
                   icon_id: icon_id_,
                   icon: new_icon,
               },
           );
           self.last_list_count += 1;
       }

       #[deprecated(note = " use get_by_icon()  ")]
       fn get_by_icon_(&self, icon_s: String) -> Vec<IconEntry> {
           (*self.list)
               .read()
               .unwrap()
               .iter()
               .filter(|(_id, ie)| ie.icon == icon_s)
               .map(|(_id, ie)| ie.clone())
               .collect()
       }

       fn get_by_index_(&self, icon_id: isize) -> Option<IconEntry> {
           (*self.list)
               .read()
               .unwrap()
               .iter()
               .filter(|(_id, ie)| ie.icon_id == icon_id)
               .map(|(_id, ie)| ie.clone())
               .next()
       }

       fn get_all_entries_(&self) -> Vec<IconEntry> {
           (*self.list)
               .read()
               .unwrap()
               .iter()
               .map(|(_id, sub)| sub.clone())
               .collect::<Vec<IconEntry>>()
       }

       #[deprecated(note = " use delete_icon ")]
       fn remove_icon(&self, icon_id: isize) {
           let o_r = (*self.list).write().unwrap().remove(&icon_id);
           assert!(o_r.is_some());
       }
    */
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
            ..Default::default()
        };
        trace!(
            "icon_repo::store_icon: {} C{:?}",
            entry.icon_id,
            entry.compression_type
        );
        match self.ctx.insert(&entry, true) {
            Ok(r) => return Result::Ok(r as usize),
            Err(e) => return Result::Err(Box::new(e) ),
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
        match event {
            TimerEvent::Shutdown => {
                self.ctx.cache_flush();
            }
            _ => (),
        }
    }
}

/*

fn icon_entry_to_json(input: &IconEntry) -> Option<String> {
    match serde_json::to_string(input) {
        Ok(encoded) => Some(encoded),
        Err(er) => {
            error!("serde_json {:?} \n {:?}", er, &input.icon_id);
            None
        }
    }
}


fn icon_entry_to_txt(input: &IconEntry) -> Option<String> {
    match bincode::serialize(input) {
        Ok(encoded) => Some(compress(String::from_utf8(encoded).unwrap().as_str())),
        Err(er) => {
            error!("bincode_serizalize {:?} \n {:?}", er, &input.icon_id);
            None
        }
    }
}


fn json_to_icon_entry(line: String, linenumber: usize) -> Option<IconEntry> {
    let dec_r: serde_json::Result<IconEntry> = serde_json::from_str(&line);
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!(
                "serde_json:from_str {:?} {:?}  on line:{} ",
                e,
                &line,
                (linenumber + 1)
            );
            None
        }
    }
}



fn txt_to_icon_entry(line: String) -> Option<IconEntry> {
    let dc_bytes: String = decompress(&line);
    let dec_r: bincode::Result<IconEntry> = bincode::deserialize(dc_bytes.as_bytes());
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("bincode:deserialize {:?}   {:?} ", e, &line);
            None
        }
    }
}
 */

/*
fn write_to(
    filename: String,
    input: &[IconEntry],
    converter: &dyn Fn(&IconEntry) -> Option<String>,
) -> std::io::Result<usize> {
    let mut bytes_written: usize = 0;
    let out = std::fs::File::create(filename)?;
    let mut buf = BufWriter::new(out);
    input.iter().filter_map(converter).for_each(|line| {
        let bbuf = line.as_bytes();
        match buf.write(bbuf) {
            Ok(bytes) => {
                let _r = buf.write(&[b'\n']);
                bytes_written += bytes + 1;
            }
            Err(e) => {
                error!("{:?}", e);
            }
        }
    });
    buf.flush()?;
    Ok(bytes_written)
}

fn read_from(
    filename: String,
    converter: &dyn Fn(String, usize) -> Option<IconEntry>,
) -> Vec<IconEntry> {
    let mut subscriptions_list: Vec<IconEntry> = Vec::default();
    match std::fs::read_to_string(filename.clone()) {
        Ok(f_str) => {
            subscriptions_list = f_str
                .lines()
                .enumerate()
                .filter_map(|(num, line)| converter(line.to_string(), num))
                .collect();
        }
        Err(e) => {
            error!("{:?}  {}", e, filename)
        }
    }
    subscriptions_list
}

 */

#[cfg(test)]
mod t_ {
    use super::*;
    pub const TEST_FOLDER1: &'static str = "../target/db_t_ico_rep";

    // cargo watch -s "(cd fr_core ;  RUST_BACKTRACE=1  cargo test  db::icon_repo::t_::t_store_file   --lib -- --exact --nocapture  )  "
    // #[test]
    fn t_store_file() {
        setup();
        {
            let mut iconrepo = IconRepo::new_(TEST_FOLDER1);
            iconrepo.startup_();
            iconrepo.clear();
            let s1 = IconEntry::default();
            assert!(iconrepo.store_entry(&s1).is_ok());
            assert!(iconrepo.store_entry(&s1).is_ok());
            let list = iconrepo.get_all_entries_();
            assert_eq!(list.len(), 2);
            iconrepo.check_or_store();
        }
        {
            let mut sr = IconRepo::new_(TEST_FOLDER1);
            sr.startup_();
            let list = sr.get_all_entries_();
            assert_eq!(list.len(), 2);
        }
    }

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
