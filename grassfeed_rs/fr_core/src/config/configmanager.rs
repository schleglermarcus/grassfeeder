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
use std::cell::RefCell;
use std::io::Result;
use std::rc::Rc;

const ID_CONFIG: &str = "config";
const SECTION_GUI_RUNNER: &str = "window";

pub struct ConfigManager {
    cconf: Rc<RefCell<Ini>>,
    cconf_modified: bool,
    cconf_filename: String,
}

impl ConfigManager {
    pub const CONF_PATH_KEY: &'static str = "conf_filename";
    pub const CONF_MODE_DEBUG: &'static str = "mode_debug";

    pub fn store_window_size(&mut self, width: i32, height: i32) {
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
    }

    pub fn store_gui_pane1_pos(&mut self, pos: i32) {
        self.set_section_key(
            SECTION_GUI_RUNNER,
            &PropDef::GuiPane1Pos.tostring(),
            &pos.to_string(),
        );
    }

    pub fn store_gui_pane2_pos(&mut self, pos: i32) {
        self.set_section_key(
            SECTION_GUI_RUNNER,
            &PropDef::GuiPane2Pos.tostring(),
            &pos.to_string(),
        );
    }

    pub fn store_column_width(&mut self, col_nr: i32, width: i32) {
        let key = match col_nr {
            1 => PropDef::GuiCol1Width.tostring(),
            _ => panic!("unknown col_nr "),
        };
        self.set_section_key(SECTION_GUI_RUNNER, &key, &width.to_string());
    }

    pub fn get_conf(&self) -> Ini {
        let i2 = (*self.cconf).borrow().clone();
        i2
    }

    pub fn conf_len(&self) -> usize {
        (*self.cconf).borrow().len()
    }

    pub fn load_from_file(&mut self, filename: &str) {
        match ini::Ini::load_from_file(filename) {
            Ok(new_ini) => {

                if new_ini.len() > 2 {

                    let mode_debug =
                        self.get_section_key_bool(&Self::section_name(), Self::CONF_MODE_DEBUG);

                    (*self.cconf).replace(new_ini);	//  unpraktisch !!!

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
    pub fn load_config_file(&mut self) {
        let fname = self.cconf_filename.clone();
        self.load_from_file(&fname);
    }

    pub fn store_to_file(&self, filename: &str) -> Result<()> {
        (*self.cconf).borrow().write_to_file(filename)
    }

    // runs on timer trigger
    pub fn store_if_modified(&mut self) {
        if !self.cconf_modified {
            return;
        }
        let filename: &str = &self.cconf_filename;
        match self.store_to_file(filename) {
            Ok(x) => {
                trace!("stored  \"{}\"  {:?}", &filename, x);
                self.cconf_modified = false;
            }
            Err(e) => {
                error!("store_if_modified \"{}\" {:?}", &filename, e);
            }
        };
    }

    /// do not mark as dirty if the value was set before
    pub fn set_section_key(&mut self, section: &str, key: &str, value: &str) {
        let mut cc = (*self.cconf).borrow_mut();
        let prev_value = cc.get_from(Some(section), key);
        if let Some(s) = prev_value {
            if s == value {
                return;
            }
        }
        cc.set_to(Some(section), key.to_string(), value.to_string());
        self.cconf_modified = true;
    }

    pub fn get_section_key(&self, section: &str, key: &str) -> Option<String> {
        let cc = (*self.cconf).borrow();
        if let Some(v) = cc.get_from(Some(section), key) {
            return Some(v.to_string());
        }
        None
    }

    pub fn get_section_key_bool(&self, section: &str, key: &str) -> bool {
        let cc = (*self.cconf).borrow();

        if let Some(v) = cc.get_from(Some(section), key) {
            if let Ok(b) = v.parse::<bool>() {
                return b;
            }
        } // else {            trace!(                "get_section_key_bool({} {})   sec_vals={:?}",                section,                key,                cc.section(Some(section))            );        }
        false
    }

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
        let conf = (*self.cconf).borrow();
        let sections: Vec<&str> = conf.sections().map(|o| o.unwrap()).collect();
        let sec_count = &sections.len();
        for s in sections {
            debug!("{} {} : {:?}", &prefix, s, conf.section(Some(s)));
        }
        debug!(
            "{} file={} mod={} #sections={}",
            &prefix, self.cconf_filename, self.cconf_modified, sec_count
        );
    }

    pub fn set_conf_filename(&mut self, new_name: String) {
        self.cconf_filename = new_name;
    }

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
}

impl Default for ConfigManager {
    fn default() -> Self {
        ConfigManager {
            cconf: Rc::new(RefCell::new(Ini::new())),
            cconf_modified: false,
            cconf_filename: String::from("default_config.ini"),
        }
    }
}

impl Buildable for ConfigManager {
    type Output = ConfigManager;

    #[allow(unreachable_code)]
    fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        panic!("don't use this, initialize manually ");
        ConfigManager::default()
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
mod configmanager_test {}
