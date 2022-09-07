use crate::timer::Timer;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use gui_layer::gui_values::PropDef;
use ini::Ini;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::BufWriter;
use std::io::Result;
use std::rc::Rc;

const ID_CONFIG: &str = "config";
// const SECTION_GUI_RUNNER: &str = "window";

pub struct ConfigManager {
    // #[deprecated]		// later
    cconf: Rc<RefCell<Ini>>,

    cconf_modified: RefCell<bool>,
    cconf_filename: String,

    user_config: Rc<RefCell<HashMap<String, String>>>,
    system_config: Rc<RefCell<HashMap<String, String>>>,
}

impl ConfigManager {
    pub const CONF_PATH_KEY: &'static str = "conf_filename";
    pub const CONF_MODE_DEBUG: &'static str = "mode_debug";

    pub fn store_window_size(&mut self, width: i32, height: i32) {
        /*
                self.set_section_key(
                    SECTION_GUI_RUNNER,
                    &PropDef::GuiWindowWidth.tostring(),
                    &width.to_string(),
                );
                self.set_section_key(
                    SECTION_GUI_RUNNER,
                    &PropDef::GuiWindowHeight.tostring(),
                    &height.to_string(),
                );
        */
        self.set_val(&PropDef::GuiWindowWidth.tostring(), width.to_string());
        self.set_val(&PropDef::GuiWindowHeight.tostring(), height.to_string());
    }

    pub fn store_gui_pane1_pos(&mut self, pos: i32) {
        /*
                self.set_section_key(
                    SECTION_GUI_RUNNER,
                    &PropDef::GuiPane1Pos.tostring(),
                    &pos.to_string(),
                );
        */
        self.set_val(&PropDef::GuiPane1Pos.tostring(), pos.to_string());
    }

    pub fn store_gui_pane2_pos(&mut self, pos: i32) {
        /*
                self.set_section_key(
                    SECTION_GUI_RUNNER,
                    &PropDef::GuiPane2Pos.tostring(),
                    &pos.to_string(),
                );
        */
        self.set_val(&PropDef::GuiPane2Pos.tostring(), pos.to_string());
    }

    pub fn store_column_width(&mut self, col_nr: i32, width: i32) {
        let key = match col_nr {
            1 => PropDef::GuiCol1Width.tostring(),
            _ => panic!("unknown col_nr "),
        };
        //        self.set_section_key(SECTION_GUI_RUNNER, &key, &width.to_string());
        self.set_val(&key, width.to_string());
    }

    // #[deprecated]		// later
    pub fn get_conf(&self) -> Ini {
        let i2 = (*self.cconf).borrow().clone();
        i2
    }

    // #[deprecated]		// later
    pub fn conf_len(&self) -> usize {
        (*self.cconf).borrow().len()
    }

    #[deprecated]
    pub fn load_from_file(&mut self, filename: &str) {
        match ini::Ini::load_from_file(filename) {
            Ok(new_ini) => {
                if new_ini.len() > 2 {
                    let mode_debug =
                        // self.get_section_key_bool(&Self::section_name(), Self::CONF_MODE_DEBUG);
                        self.get_val_bool( Self::CONF_MODE_DEBUG);
                    (*self.cconf).replace(new_ini); //  unpraktisch !!!
                    (*self.cconf).borrow_mut().set_to(
                        Some(Self::section_name()),
                        Self::CONF_MODE_DEBUG.to_string(),
                        mode_debug.to_string(),
                    );
                }
                self.cconf_filename = filename.to_string();
            }
            Err(e) => {
                trace!("load_from_file {} {:?}", &filename, &e);
            }
        }
    }

    /// loads from preset available name
    #[deprecated] // later
    pub fn load_config_file(&mut self) {
        let fname = self.cconf_filename.clone();
        self.load_from_file(&fname);
    }

    #[deprecated] // later
    pub fn store_to_file(&self, filename: &str) -> Result<()> {
        (*self.cconf).borrow().write_to_file(filename)
    }

    // runs on timer trigger
    pub fn store_if_modified(&mut self) {
        if !*self.cconf_modified.borrow() {
            return;
        }
        let filename: &str = &self.cconf_filename;

/*
        match  {
            Ok(x) => {
                trace!("stored  \"{}\"  {:?}", &filename, x);
                self.cconf_modified.replace(false);
            }
            Err(e) => {
                error!("store_if_modified \"{}\" {:?}", &filename, e);
            }
        };
*/
let _r = self.store_user_conf(filename.to_string());
    }

    /// do not mark as dirty if the value was set before
    #[deprecated]
    pub fn set_section_key(&mut self, section: &str, key: &str, value: &str) {
        let mut cc = (*self.cconf).borrow_mut();
        let prev_value = cc.get_from(Some(section), key);
        if let Some(s) = prev_value {
            if s == value {
                return;
            }
        }
        cc.set_to(Some(section), key.to_string(), value.to_string());
        self.cconf_modified.replace(true);
    }

    #[deprecated]
    pub fn get_section_key(&self, section: &str, key: &str) -> Option<String> {
        let cc = (*self.cconf).borrow();
        if let Some(v) = cc.get_from(Some(section), key) {
            return Some(v.to_string());
        }
        None
    }

    #[deprecated]
    pub fn get_section_key_bool(&self, section: &str, key: &str) -> bool {
        let cc = (*self.cconf).borrow();

        if let Some(v) = cc.get_from(Some(section), key) {
            if let Ok(b) = v.parse::<bool>() {
                return b;
            }
        } // else {            trace!(                "get_section_key_bool({} {})   sec_vals={:?}",                section,                key,                cc.section(Some(section))            );        }
        false
    }

    #[deprecated]
    pub fn get_section_key_int(&self, section: &str, key: &str, defaultv: isize) -> isize {
        let cc = (*self.cconf).borrow();
        if let Some(v) = cc.get_from(Some(section), key) {
            if let Ok(i) = v.parse::<isize>() {
                return i;
            }
        }
        defaultv
    }

    pub fn debug_dump(&self, prefix: &str) {
        debug!(
            "{} DD-system= {:#?} ",
            prefix,
            (*self.system_config).borrow()
        );
        debug!("{} DD-user= {:#?} ", prefix, (*self.user_config).borrow());
    }

    /*
        pub fn set_conf_filename(&mut self, new_name: String) {
            self.cconf_filename = new_name;
        }
    */

    // #[deprecated] // later
    pub fn new_with_ini(ini_r: Rc<RefCell<Ini>>) -> ConfigManager {
        let filename = (*ini_r)
            .borrow()
            .section(Some(ConfigManager::section_name()))
            .unwrap()
            .get(ConfigManager::CONF_PATH_KEY)
            .unwrap()
            .to_string();
        ConfigManager {
            cconf_filename: filename,
            cconf: ini_r,
            ..ConfigManager::default()
        }
    }

    pub fn get_user_conf(&self) -> Rc<RefCell<HashMap<String, String>>> {
        self.user_config.clone()
    }

    pub fn load_user_conf(&self, filename: &String) -> bool {
        // self.cconf_filename
        if !std::path::Path::new(&filename).exists() {
            trace!(
                "load_subscriptions_pretty file {} not found. ",
                &self.cconf_filename
            );
            return false;
        }

        let r_string = std::fs::read_to_string(filename.clone());
        if r_string.is_err() {
            error!("load_user_conf  {:?}  {}", r_string.err(), filename);
            return false;
        }
        let lines = r_string.unwrap();
        let dec_r: serde_json::Result<HashMap<String, String>> = serde_json::from_str(&lines);
        if dec_r.is_err() {
            error!("serde_json:from_str {:?}   {:?} ", dec_r.err(), &filename);
            return false;
        }
        let userconf = dec_r.unwrap();
        let mut uc = (*self.user_config).borrow_mut();
        userconf.into_iter().for_each(|(k, v)| {
            uc.insert(k, v);
        });
        true
    }

    pub fn store_user_conf(&self, filename: String)  -> bool   {
        let r_file = std::fs::File::create(filename.clone());
        if r_file.is_err() {
            warn!("{:?} writing to {} ", r_file.err(), &filename);
            return false;
        }
        let outfile = r_file.unwrap();
        let bufwriter = BufWriter::new(outfile);
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
        let mut serializer = serde_json::Serializer::with_formatter(bufwriter, formatter);

        let writemap: &HashMap<String, String> = &*(*self.user_config).borrow();

        let r_ser = writemap.serialize(&mut serializer);
        if r_ser.is_err() {
            warn!("serializing into {} => {:?}", filename, r_ser.err());
			return false
        }
        debug!("written {} to {}", &writemap.len(), &filename);
		true
    }

    pub fn get_val(&self, key: &str) -> Option<String> {
        (*self.user_config)
            .borrow()
            .get(key)
            .map_or(None, |r| Some(r.clone()))
    }

    pub fn get_val_int(&self, key: &str) -> Option<isize> {
        if let Some(valstr) = (*self.user_config).borrow().get(key) {
            if let Ok(intval) = valstr.parse::<isize>() {
                return Some(intval);
            }
        }
        None
    }

    pub fn get_val_bool(&self, key: &str) -> bool {
        if let Some(valstr) = (*self.user_config).borrow().get(key) {
            if let Ok(b) = valstr.parse::<bool>() {
                return b;
            }
        }
        false
    }

    pub fn get_sys_val(&self, key: &str) -> Option<String> {
        (*self.system_config)
            .borrow()
            .get(key)
            .map_or(None, |r| Some(r.clone()))
    }

    pub fn set_system_config(&mut self, conf: Rc<RefCell<HashMap<String, String>>>) {
        // let mut sc = (*self.system_config).borrow_mut();        sc.clear();        sc.extend(conf);
        self.system_config = conf;
    }

    /// set user config
    pub fn set_val(&self, key: &str, val: String) {
        if let Some(existing) = (*self.user_config).borrow().get(&key.to_string()) {
            if *existing == val {
                return;
            }
        }
        (*self.user_config)
            .borrow_mut()
            .insert(key.to_string(), val);
        self.cconf_modified.replace(true);
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        ConfigManager {
            cconf: Rc::new(RefCell::new(Ini::new())),
            cconf_modified: RefCell::new(false),
            cconf_filename: String::from("default_config.ini"),
            user_config: Rc::new(RefCell::new(HashMap::new())),
            system_config: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl Buildable for ConfigManager {
    type Output = ConfigManager;

    #[allow(unreachable_code)]
    fn build(conf: Box<dyn BuildConfig>, appcontext: &AppContext) -> Self::Output {
        // panic!("don't use this, initialize manually ");
        let mut cm = ConfigManager::default();
        cm.cconf_filename = conf.get(&ConfigManager::CONF_PATH_KEY.to_string()).unwrap();
        cm.set_system_config(appcontext.get_system_config());
        let _r = cm.load_user_conf(&cm.cconf_filename);
        // exit on fail  here ?
        cm
    }

    fn section_name() -> String {
        String::from(ID_CONFIG)
    }
}

impl TimerReceiver for ConfigManager {
    fn trigger(&mut self, event: &TimerEvent) {
        match event {
            TimerEvent::Timer10s => {
                self.store_if_modified();
            }
            TimerEvent::Shutdown => {
                self.store_if_modified();
            }
            _ => (),
        }
    }
}

impl StartupWithAppContext for ConfigManager {
    fn startup(&mut self, _ac: &AppContext) {
        let timer_r = _ac.get_rc::<Timer>().unwrap();
        let configmanager_r = _ac.get_rc::<ConfigManager>().unwrap();
        {
            let mut t = (*timer_r).borrow_mut();
            t.register(&TimerEvent::Timer1s, configmanager_r.clone());
            t.register(&TimerEvent::Timer10s, configmanager_r.clone());
            t.register(&TimerEvent::Shutdown, configmanager_r);
        }
    }
}

//------------------------------------------------------

#[cfg(test)]
mod t {

    use super::*;

    #[test]
    fn configmanager_load_store() {
        setup();
        let cf_filename = "../target/configmanager_load_store.json";
        {
            let cm = ConfigManager::default();
            cm.set_val("Coffee", "3".to_string());
            cm.store_user_conf(cf_filename.to_string());
        }
        {
            let cm = ConfigManager::default();
            let _lr = cm.load_user_conf(&cf_filename.to_string());
            assert_eq!(cm.get_val("Coffee"), Some("3".to_string()));
        }
    }

    fn setup() {}
}
