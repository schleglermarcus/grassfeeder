use crate::db::message::compress;
use crate::db::message::decompress;
use crate::timer::Timer;
use crate::util::db_time_to_display;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;

pub const KEY_FOLDERNAME: &str = "cache_folder";
pub const FILENAME: &str = "errors.json.txt";
pub const CONV_TO: &dyn Fn(String) -> Option<ErrorEntry> = &json_to_error_entry;
pub const CONV_FROM: &dyn Fn(&ErrorEntry) -> Option<String> = &error_entry_to_json;

///
/// List of Errors
///
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorEntry {
    pub err_id: isize,
    pub subs_id: isize,
    pub date: i64,
    pub err_code: isize,
    pub remote_address: String,
    pub text: String,
}

impl std::fmt::Debug for ErrorEntry {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("ErrorE")
            .field("ID", &self.err_id)
            .field("subs_id", &self.subs_id)
            .field("date", &db_time_to_display(self.date))
            .field("code", &self.err_code)
            .field("text", &self.text)
            .field("url", &self.remote_address)
            .finish()
    }
}

impl ErrorEntry {
    pub fn to_line(&self, display_name: String) -> String {
        let mut disp = display_name;
        disp.truncate(30);
        let mut e_text = self.text.clone();
        e_text.truncate(50);
        let mut e_remot = self.remote_address.clone();
        e_remot.truncate(40);
        format!(
            "{:20} {:16} {:4} {:50} {:40}",
            disp,
            db_time_to_display(self.date),
            self.err_code,
            e_text,
            e_remot,
        )
    }
}

pub struct ErrorRepo {
    ///  ID -> Entry
    list_unstored: Arc<RwLock<MapAndId>>,
    folder_name: String,
    unstored_list_count: RwLock<usize>,
    list_stored: Arc<RwLock<HashMap<isize, ErrorEntry>>>,
}

#[derive(Default, Debug)]
pub struct MapAndId {
    map: HashMap<isize, ErrorEntry>,
    highest_id: isize,
}

impl ErrorRepo {
    pub fn new(folder_name_: &str) -> Self {
        ErrorRepo {
            list_unstored: Default::default(), //  Arc::new(RwLock::new(HashMap::new())),
            folder_name: folder_name_.to_string(),
            unstored_list_count: Default::default(),
            list_stored: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn by_existing_list(existing: Arc<RwLock<MapAndId>>) -> Self {
        ErrorRepo {
            list_unstored: existing,
            folder_name: String::default(),
            unstored_list_count: Default::default(),
            list_stored: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_list(&self) -> Arc<RwLock<MapAndId>> {
        self.list_unstored.clone()
    }

    fn filename(&self) -> String {
        let slash = if self.folder_name.ends_with('/') {
            ""
        } else {
            "/"
        };
        format!("{}{}{}", self.folder_name, slash, FILENAME)
    }

    /// make sure the file exists
    pub fn check_file(&self) -> std::io::Result<()> {
        let filename = self.filename();
        if !std::path::Path::new(&filename).exists() {
            std::fs::create_dir_all(&self.folder_name)?;
            let _file = File::create(&filename)?;
        }
        Ok(())
    }

    pub fn startup_read(&self) {
        if let Err(e) = self.check_file() {
            warn!("ErrorRepo Startup {:?}", e);
        }
        self.read_stored();
    }

    pub fn check_or_store(&mut self) {
        let unstored_len = (*self.list_unstored).read().unwrap().map.len();
        let stored_len = (*self.list_stored).read().unwrap().len();
        if unstored_len > 0 && self.store_to_file() {
            (*self.list_unstored).write().unwrap().map.clear();
        }
        if stored_len > 0 {
            (*self.list_stored).write().unwrap().clear();
        }
    }

    fn store_to_file(&self) -> bool {
        let mut values = (*self.list_unstored)
            .read()
            .unwrap()
            .map
            .values()
            .cloned()
            .collect::<Vec<ErrorEntry>>();
        values.sort_by(|a, b| a.err_id.cmp(&b.err_id));
        let _r = self.check_file();
        match append_to_file(self.filename(), &values, CONV_FROM) {
            Ok(_bytes_written) => {
                *self.unstored_list_count.write().unwrap() = values.len();
            }
            Err(e) => {
                error!("IconRepo:store_to_file  {}  {:?} ", &self.filename(), e);
                return false;
            }
        }
        true
    }

    pub fn add_error(&self, subs_id_: isize, error_code_: isize, http_url: String, msg: String) {
        let en = ErrorEntry {
            subs_id: subs_id_,
            err_code: error_code_,
            text: msg,
            remote_address: http_url,
            date: crate::util::timestamp_now(),
            ..Default::default()
        };
        self.store_error(&en);
    }

    pub fn store_error(&self, entry: &ErrorEntry) {
        *self.unstored_list_count.write().unwrap() += 1;
        let n_id = self.next_id();
        let mut entrym = entry.clone();
        entrym.date = crate::util::timestamp_now();
        entrym.err_id = n_id;
        (*self.list_unstored)
            .write()
            .unwrap()
            .map
            .insert(n_id as isize, entrym);
    }

    pub fn next_id(&self) -> isize {
        let mut highest_id = (*self.list_unstored).read().unwrap().highest_id;
        if highest_id <= 0 {
            error!("need to call startup_read");
            return 7;
        }
        highest_id += 1;
        (*self.list_unstored).write().unwrap().highest_id = highest_id;
        highest_id
    }

    pub fn read_stored(&self) {
        let slist = read_from(self.filename(), CONV_TO);
        let mut st = (*self.list_stored).write().unwrap();
        let mut highest: isize = 9;
        slist.into_iter().for_each(|se| {
            highest = std::cmp::max(highest, se.err_id);
            st.insert(se.err_id, se);
        });
        let highest_cur = (*self.list_unstored).read().unwrap().highest_id;
        if highest > highest_cur {
            (*self.list_unstored).write().unwrap().highest_id = highest;
        }
    }

    fn check_stored_are_present(&self) {
        let numstored = (*self.list_stored).read().unwrap().len();
        // debug!("check_stored_are_present : numstored={}", numstored);
        if numstored == 0 {
            self.read_stored();
        }
    }

    pub fn get_by_subscription(&self, subs_id: isize) -> Vec<ErrorEntry> {
        self.check_stored_are_present();
        (*self.list_stored)
            .read()
            .unwrap()
            .iter()
            .filter_map(|(_id, se)| {
                if se.subs_id == subs_id {
                    Some(se)
                } else {
                    None
                }
            })
            .cloned()
            .collect()
    }

    pub fn get_last_entry(&self, subs_id: isize) -> Option<ErrorEntry> {
        let mut ret_list: Vec<ErrorEntry> = (*self.list_unstored)
            .read()
            .unwrap()
            .map
            .iter()
            .filter_map(|(_id, se)| {
                if se.subs_id == subs_id {
                    Some(se)
                } else {
                    None
                }
            })
            .cloned()
            .collect();
        if ret_list.is_empty() {
            self.check_stored_are_present();
            ret_list = (*self.list_stored)
                .read()
                .unwrap()
                .iter()
                .filter_map(|(_id, se)| {
                    if se.subs_id == subs_id {
                        Some(se)
                    } else {
                        None
                    }
                })
                .cloned()
                .collect()
        }
        ret_list.sort_by(|a, b| a.date.cmp(&b.date));
        ret_list.get(0).cloned()
    }
}

//-------------------

impl Buildable for ErrorRepo {
    type Output = ErrorRepo;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let o_folder = conf.get(KEY_FOLDERNAME);
        if o_folder.is_none() {
            conf.dump();
            panic!("E-Repo config has no {} ", KEY_FOLDERNAME);
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
        let timer_r: Rc<RefCell<Timer>> = (*ac).get_rc::<Timer>().unwrap();
        let su_r = ac.get_rc::<ErrorRepo>().unwrap();
        {
            (*timer_r)
                .borrow_mut()
                .register(&TimerEvent::Timer10s, su_r.clone());
            (*timer_r)
                .borrow_mut()
                .register(&TimerEvent::Shutdown, su_r);
        }
        self.startup_read();
    }
}

impl TimerReceiver for ErrorRepo {
    fn trigger(&mut self, event: &TimerEvent) {
        match event {
            TimerEvent::Timer10s => {
                self.check_or_store();
            }
            TimerEvent::Shutdown => {
                self.check_or_store();
            }
            _ => (),
        }
    }
}

fn error_entry_to_json(input: &ErrorEntry) -> Option<String> {
    match serde_json::to_string(input) {
        Ok(encoded) => Some(encoded),
        Err(er) => {
            error!("serde_json {:?} \n {:?}", er, &input.err_id);
            None
        }
    }
}

#[allow(dead_code)]
fn error_entry_to_txt(input: &ErrorEntry) -> Option<String> {
    match bincode::serialize(input) {
        Ok(encoded) => Some(compress(String::from_utf8(encoded).unwrap().as_str())),
        Err(er) => {
            error!("bincode_serizalize {:?} \n {:?}", er, &input.err_id);
            None
        }
    }
}

// #[allow(dead_code)]
fn json_to_error_entry(line: String) -> Option<ErrorEntry> {
    let dec_r: serde_json::Result<ErrorEntry> = serde_json::from_str(&line);
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("serde_json:from_str {:?}   {:?} ", e, &line);
            None
        }
    }
}

#[allow(dead_code)]
fn txt_to_error_entry(line: String) -> Option<ErrorEntry> {
    let dc_bytes: String = decompress(&line);
    let dec_r: bincode::Result<ErrorEntry> = bincode::deserialize(dc_bytes.as_bytes());
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("bincode:deserialize {:?}   {:?} ", e, &line);
            None
        }
    }
}

fn append_to_file(
    filename: String,
    input: &[ErrorEntry],
    converter: &dyn Fn(&ErrorEntry) -> Option<String>,
) -> std::io::Result<usize> {
    let mut bytes_written: usize = 0;
    let file: File = if std::path::Path::new(&filename).exists() {
        OpenOptions::new().write(true).append(true).open(filename)?
    } else {
        File::create(&filename)?
    };
    let mut buf = BufWriter::new(file);
    input
        .iter()
        .filter_map(|se| converter(se))
        .for_each(|line| {
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
    converter: &dyn Fn(String) -> Option<ErrorEntry>,
) -> Vec<ErrorEntry> {
    let mut e_list: Vec<ErrorEntry> = Vec::default();
    match std::fs::read_to_string(filename.clone()) {
        Ok(f_str) => {
            e_list = f_str
                .lines()
                .filter_map(|line| converter(line.to_string()))
                .collect();
        }
        Err(e) => {
            error!("{:?}  {}", e, filename)
        }
    }
    e_list
}

#[cfg(test)]
mod t {
    use super::*;

    // RUST_BACKTRACE=1 cargo watch -s "cargo test  db::errors_repo::t::t_error_repo_store     --lib -- --exact --nocapture"
    #[test]
    fn t_error_repo_store() {
        setup();
        let mut e_repo = ErrorRepo::new("../target/err_rep/");
        let mut e1 = ErrorEntry::default();
        e1.text = "Hello!".to_string();
        e1.subs_id = 13;
        e_repo.store_error(&e1);
        e_repo.check_or_store();
        let next_id = e_repo.next_id();

        println!("next_id={}", next_id);
        assert!(next_id >= 7);
        let subs_list = e_repo.get_by_subscription(13);
        println!("#subs_list={}", subs_list.len());
        assert!(subs_list.len() >= 1);
    }

    // dummy instead of log configuration
    fn setup() {}
}