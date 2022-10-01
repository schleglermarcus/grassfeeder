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
use gui_layer::gui_values::PropDef;
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
        fmt.debug_struct("IconEntry")
            .field("ID", &self.err_id)
            .field("subs_id", &self.subs_id)
            .field("date", &db_time_to_display(self.date))
            .finish()
    }
}

pub struct ErrorRepo {
    folder_name: String,
    ///  ID -> Entry
    list_unstored: Arc<RwLock<HashMap<isize, ErrorEntry>>>,
    unstored_list_count: usize,
    highest_id: isize,
}

impl ErrorRepo {
    pub fn new(folder_name_: &str) -> Self {
        ErrorRepo {
            list_unstored: Arc::new(RwLock::new(HashMap::new())),
            folder_name: folder_name_.to_string(),
            unstored_list_count: 0,
            highest_id: -1,
        }
    }

    pub fn by_existing_list(existing: Arc<RwLock<HashMap<isize, ErrorEntry>>>) -> Self {
        ErrorRepo {
            list_unstored: existing,
            folder_name: String::default(),
            unstored_list_count: 0,
            highest_id: 0,
        }
    }

    fn filename(&self) -> String {
        let slash = if self.folder_name.ends_with('/') {
            ""
        } else {
            "/"
        };
        format!("{}{}{}", self.folder_name, slash, FILENAME)
    }

    // make sure the file exists
    pub fn check_file(&self) -> std::io::Result<()> {
        let filename = self.filename();
        if !std::path::Path::new(&filename).exists() {
            std::fs::create_dir_all(&self.folder_name)?;
            let _file = File::create(&filename)?;
        }
        Ok(())
    }

    pub fn startup(&mut self) {
        if let Err(e) = self.check_file() {
            warn!("ErrorRepo Startup {:?}", e);
        }
    }

    pub fn check_or_store(&mut self) {
        let unstored_len = (*self.list_unstored).read().unwrap().len();
        if unstored_len > 0 {
            debug!("check_or_store: {}", unstored_len);
            if self.store_to_file() {
                (*self.list_unstored).write().unwrap().clear();
            }
        }
    }

    fn store_to_file(&mut self) -> bool {
        let mut values = (*self.list_unstored)
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<ErrorEntry>>();
        values.sort_by(|a, b| a.err_id.cmp(&b.err_id));

        let _r = self.check_file();
        debug!("check result={:?}", _r);

        match append_to_file(self.filename(), &values, CONV_FROM) {
            Ok(_bytes_written) => {
                self.unstored_list_count = values.len();
            }
            Err(e) => {
                error!("IconRepo:store_to_file  {}  {:?} ", &self.filename(), e);
                return false;
            }
        }
        true
    }

    // pub fn clear(&self) {       (*self.list_unstored).write().unwrap().clear();    }

    pub fn add_error(&mut self, entry: &ErrorEntry) {
        self.unstored_list_count += 1;
        self.highest_id += 1;
        let mut entrym = entry.clone();
        entrym.date = crate::util::timestamp_now();
        entrym.err_id = self.highest_id as isize;
        (*self.list_unstored)
            .write()
            .unwrap()
            .insert(self.highest_id as isize, entrym);
    }

    pub fn next_id(&mut self) -> isize {
        if self.highest_id < 1 {
            if self.check_file().is_err() {
                panic!("cannot access error storage");
            }
            /*
                        let slist = read_from(self.filename.clone(), CONV_TO);
                        let mut hm = (*self.list).write().unwrap();
                        slist.into_iter().for_each(|se| {
                            let id = se.icon_id;
                            hm.insert(id, se);
                        });
            */
        }
        self.highest_id += 1;
        self.highest_id
    }

    /*
        pub fn get_by_icon(&self, icon_s: String) -> Vec<ErrorEntry> {
            (*self.list)
                .read()
                .unwrap()
                .iter()
                .filter(|(_id, ie)| ie.icon == icon_s)
                .map(|(_id, ie)| ie.clone())
                .collect()
        }

        pub fn get_by_index(&self, icon_id: isize) -> Option<ErrorEntry> {
            (*self.list)
                .read()
                .unwrap()
                .iter()
                .filter(|(_id, ie)| ie.icon_id == icon_id)
                .map(|(_id, ie)| ie.clone())
                .next()
        }
    */

    /*	TODO
        pub fn store_entry(&self, entry: &ErrorEntry) -> Result<ErrorEntry, Box<dyn std::error::Error>> {
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
            Ok(store_entry)
        }
    */
    /*
        pub fn get_list(&self) -> Arc<RwLock<HashMap<isize, ErrorEntry>>> {
            self.list.clone()
        }
    */

    /* TODO   load all error entries,  but discard them on timer


        pub fn get_all_entries(&self) -> Vec<ErrorEntry> {
            (*self.list)
                .read()
                .unwrap()
                .iter()
                .map(|(_id, sub)| sub.clone())
                .collect::<Vec<ErrorEntry>>()
        }
    */
}

//-------------------

impl Buildable for ErrorRepo {
    type Output = ErrorRepo;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let o_folder = conf.get(&PropDef::BrowserDir.tostring());
        match o_folder {
            Some(folder) => ErrorRepo::new(&folder),
            None => {
                conf.dump();
                panic!("iconrepo config has no {} ", PropDef::BrowserDir.tostring());
            }
        }
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

        self.startup();
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

// #[allow(dead_code)]
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
        //         Ok(encoded) => Some(compress(&encoded)),
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

// #[allow(dead_code)]
fn append_to_file(
    filename: String,
    input: &[ErrorEntry],
    converter: &dyn Fn(&ErrorEntry) -> Option<String>,
) -> std::io::Result<usize> {
    let mut bytes_written: usize = 0;
    let file: File;

    if std::path::Path::new(&filename).exists() {
        debug!("appending... {}", filename.clone());
        file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(filename.clone())?;
    } else {
        debug!("creating ... {}", filename);
        file = File::create(&filename)?;
    }

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

#[allow(dead_code)]
fn read_from(
    filename: String,
    converter: &dyn Fn(String) -> Option<ErrorEntry>,
) -> Vec<ErrorEntry> {
    let mut subscriptions_list: Vec<ErrorEntry> = Vec::default();
    match std::fs::read_to_string(filename.clone()) {
        Ok(f_str) => {
            subscriptions_list = f_str
                .lines()
                .filter_map(|line| converter(line.to_string()))
                .collect();
        }
        Err(e) => {
            error!("{:?}  {}", e, filename)
        }
    }
    subscriptions_list
}

#[cfg(test)]
mod ut {
    /*


    use super::*;
    pub const TEST_FOLDER1: &'static str = "../target/db_t_ico_rep";



        #[test]
        fn t_store_file() {
            {
                let mut iconrepo = IconRepo::new(TEST_FOLDER1);
                iconrepo.startup();
                iconrepo.clear();
                let s1 = IconEntry::default();
                assert!(iconrepo.store_entry(&s1).is_ok());
                assert!(iconrepo.store_entry(&s1).is_ok());
                let list = iconrepo.get_all_entries();
                assert_eq!(list.len(), 2);
                iconrepo.check_or_store();
            }

            {
                let mut sr = IconRepo::new(TEST_FOLDER1);
                sr.startup();
                let list = sr.get_all_entries();
                assert_eq!(list.len(), 2);
            }
        }
    */
}
