use crate::db::message::compress;
use crate::db::message::decompress;
use crate::timer::Timer;
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
use std::io::BufWriter;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;

pub const KEY_FOLDERNAME: &str = "subscriptions_folder";
pub const FILENAME: &str = "icons_list.json";
pub const CONV_TO: &dyn Fn(String) -> Option<IconEntry> = &json_to_icon_entry;
pub const CONV_FROM: &dyn Fn(&IconEntry) -> Option<String> = &icon_entry_to_json;

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
            .field("icon_id", &self.icon_id)
            .field("IC#", &self.icon.len())
            .finish()
    }
}

pub struct IconRepo {
    filename: String,
    ///  ID -> Entry
    list: Arc<RwLock<HashMap<isize, IconEntry>>>,
    last_list_count: usize,
}

impl IconRepo {
    pub fn new(folder_name: &str) -> Self {
        IconRepo {
            list: Arc::new(RwLock::new(HashMap::new())),
            filename: folder_name.to_string(),
            last_list_count: 0,
        }
    }

    pub fn by_existing_list(existing: Arc<RwLock<HashMap<isize, IconEntry>>>) -> Self {
        IconRepo {
            list: existing,
            filename: String::default(),
            last_list_count: 0,
        }
    }

    pub fn startup(&mut self) -> bool {
        match std::fs::create_dir_all(&self.filename) {
            Ok(()) => (),
            Err(e) => {
                error!("IconRepo cannot create folder {} {:?}", &self.filename, e);
                return false;
            }
        }
        let filename = format!("{}/{}", self.filename, FILENAME);
        self.filename = filename;
        if std::path::Path::new(&self.filename).exists() {
            let slist = read_from(self.filename.clone(), CONV_TO);
            let mut hm = (*self.list).write().unwrap();
            slist.into_iter().for_each(|se| {
                let id = se.icon_id;
                hm.insert(id, se);
            });
        } else {
            trace!("subscription file not found: {}", &self.filename);
        }

        true
    }

    pub fn check_or_store(&mut self) {
        if (*self.list).read().unwrap().len() != self.last_list_count {
            self.store_to_file();
        }
    }

    fn store_to_file(&mut self) {
        let mut values = (*self.list)
            .read()
            .unwrap()
            .values()
            // .map(|e| e.clone())
            .cloned()
            .collect::<Vec<IconEntry>>();
        values.sort_by(|a, b| a.icon_id.cmp(&b.icon_id));
        match write_to(self.filename.clone(), &values, CONV_FROM) {
            Ok(_bytes_written) => {
                self.last_list_count = values.len();
            }
            Err(e) => {
                error!("IconRepo:store_to_file  {}  {:?} ", &self.filename, e);
            }
        }
    }

    pub fn clear(&self) {
        (*self.list).write().unwrap().clear();
    }

    pub fn store_icon(&mut self, icon_id_: isize, new_icon: String) {
        (*self.list).write().unwrap().insert(
            icon_id_,
            IconEntry {
                icon_id: icon_id_,
                icon: new_icon,
            },
        );
        self.last_list_count += 1;
    }

    pub fn get_by_icon(&self, icon_s: String) -> Vec<IconEntry> {
        (*self.list)
            .read()
            .unwrap()
            .iter()
            .filter(|(_id, ie)| ie.icon == icon_s)
            .map(|(_id, ie)| ie.clone())
            .collect()
    }

    pub fn get_by_index(&self, icon_id: isize) -> Option<IconEntry> {
        (*self.list)
            .read()
            .unwrap()
            .iter()
            .filter(|(_id, ie)| ie.icon_id == icon_id)
            .map(|(_id, ie)| ie.clone())
            .next()
    }

    pub fn get_all_entries(&self) -> Vec<IconEntry> {
        (*self.list)
            .read()
            .unwrap()
            .iter()
            .map(|(_id, sub)| sub.clone())
            .collect::<Vec<IconEntry>>()
    }

    pub fn store_entry(&self, entry: &IconEntry) -> Result<IconEntry, Box<dyn std::error::Error>> {
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

    pub fn get_list(&self) -> Arc<RwLock<HashMap<isize, IconEntry>>> {
        self.list.clone()
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
                panic!("iconrepo config has no {} ", KEY_FOLDERNAME);
            }
        }
    }

    fn section_name() -> String {
        String::from("subscriptions_repo")
    }
}

impl StartupWithAppContext for IconRepo {
    fn startup(&mut self, ac: &AppContext) {
        let timer_r: Rc<RefCell<Timer>> = (*ac).get_rc::<Timer>().unwrap();
        let su_r = ac.get_rc::<IconRepo>().unwrap();
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

impl TimerReceiver for IconRepo {
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

#[allow(dead_code)]
fn icon_entry_to_json(input: &IconEntry) -> Option<String> {
    match serde_json::to_string(input) {
        Ok(encoded) => Some(encoded),
        Err(er) => {
            error!("serde_json {:?} \n {:?}", er, &input.icon_id);
            None
        }
    }
}

#[allow(dead_code)]
fn icon_entry_to_txt(input: &IconEntry) -> Option<String> {
    match bincode::serialize(input) {
        //         Ok(encoded) => Some(compress(&encoded)),
        Ok(encoded) => Some(compress(String::from_utf8(encoded).unwrap().as_str())),
        Err(er) => {
            error!("bincode_serizalize {:?} \n {:?}", er, &input.icon_id);
            None
        }
    }
}

#[allow(dead_code)]
fn json_to_icon_entry(line: String) -> Option<IconEntry> {
    let dec_r: serde_json::Result<IconEntry> = serde_json::from_str(&line);
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("serde_json:from_str {:?}   {:?} ", e, &line);
            None
        }
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn write_to(
    filename: String,
    input: &[IconEntry],
    converter: &dyn Fn(&IconEntry) -> Option<String>,
) -> std::io::Result<usize> {
    let mut bytes_written: usize = 0;
    let out = std::fs::File::create(filename)?;
    let mut buf = BufWriter::new(out);
    input
        .iter()
        .filter_map(|se| converter(se))
        .for_each(|line| {
            let bbuf = line.as_bytes();
            match buf.write(bbuf) {
                Ok(bytes) => {
                    let _r = buf.write(&[ b'\n' ]);
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

fn read_from(filename: String, converter: &dyn Fn(String) -> Option<IconEntry>) -> Vec<IconEntry> {
    let mut subscriptions_list: Vec<IconEntry> = Vec::default();
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
    use super::*;
    pub const TEST_FOLDER1: &'static str = "../target/db_t_ico_rep";
    #[test]
    fn t_store_file() {
        setup();
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

    // dummy instead of log configuration
    fn setup() {}
}
