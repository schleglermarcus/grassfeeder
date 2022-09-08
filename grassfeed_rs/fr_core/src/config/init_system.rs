use crate::config::configmanager::ConfigManager;
use crate::controller::browserpane::BrowserPane;
use crate::controller::contentdownloader::Downloader;
use crate::controller::contentlist::FeedContents;
use crate::controller::guiprocessor::GuiProcessor;
use crate::controller::sourcetree::SourceTreeController;
use crate::db::icon_repo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo;
use crate::opml::opmlreader::OpmlReader;
use crate::timer::ITimer;
use crate::timer::Timer;
use crate::ui_select::gui_context::GuiContext;
use context::appcontext::AppContext;
use gui_layer::gui_values::PropDef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct GrassFeederConfig {
    pub path_config: String,
    pub path_cache: String,
    pub debug_mode: bool,
    pub version: String,
}

pub fn start(conf: GrassFeederConfig) -> AppContext {
    let systemconf = create_system_config(&conf);
    let mut appcontext = AppContext::new(systemconf);
    appcontext.build::<ConfigManager>();
    let configmanager_r: Rc<RefCell<ConfigManager>> = appcontext.get_rc::<ConfigManager>().unwrap();
    appcontext.set_user_conf((*configmanager_r).borrow().get_user_conf());

    appcontext.build::<Timer>();
    appcontext.build::<GuiContext>();
    appcontext.build::<subscription_repo::SubscriptionRepo>();
    appcontext.build::<icon_repo::IconRepo>();
    appcontext.build::<MessagesRepo>();
    appcontext.build::<OpmlReader>();
    appcontext.build::<Downloader>();
    appcontext.build::<SourceTreeController>();
    appcontext.build::<BrowserPane>();
    appcontext.build::<FeedContents>();
    appcontext.build::<GuiProcessor>();
    appcontext.startup();
    appcontext
}

pub fn run(appcontext: &AppContext) {
    let timer_r: Rc<RefCell<dyn ITimer>> = appcontext.get_rc::<Timer>().unwrap();
    (*timer_r).borrow_mut().main_loop();
}

pub fn create_system_config(gf_conf: &GrassFeederConfig) -> HashMap<String, String> {
    check_or_create_folder(&gf_conf.path_config);
    check_or_create_folder(&gf_conf.path_cache);

    let mut ret: HashMap<String, String> = HashMap::default();
    ret.insert(
        icon_repo::KEY_FOLDERNAME.to_string(),
        gf_conf.path_config.clone(),
    );

    ret.insert(
        subscription_repo::KEY_FOLDERNAME.to_string(),
        gf_conf.path_config.clone(),
    );

    ret.insert(
        ConfigManager::CONF_PATH_KEY.to_string(),
        format!("{}/config.json", &gf_conf.path_config),
    );
    ret.insert(
        ConfigManager::CONF_MODE_DEBUG.to_string(),
        gf_conf.debug_mode.to_string(),
    );
    ret.insert(
        GuiContext::CONF_RCS_VERSION.to_string(),
        gf_conf.version.clone(),
    ); // TODO choose one
    ret.insert(PropDef::AppRcsVersion.tostring(), gf_conf.version.clone());
    ret.insert(
        MessagesRepo::CONF_DB_KEY_FOLDER.to_string(),
        format!("{}", &gf_conf.path_config),
    );
    ret.insert(
        PropDef::BrowserDir.tostring(),
        format!("{}/browser", &gf_conf.path_cache),
    );

    /*
        mod_ini.set_to(
            Some(GuiContext::section_name()),
            PropDef::GuiWindowWidth.tostring(),
            "400".to_string(),
        );
        mod_ini.set_to(
            Some(GuiContext::section_name()),
            PropDef::GuiWindowHeight.tostring(),
            "200".to_string(),
        );
        mod_ini.set_to(
            Some(GuiContext::section_name()),
            PropDef::GuiPane1Pos.tostring(),
            "150".to_string(),
        );
        mod_ini.set_to(
            Some(GuiContext::section_name()),
            PropDef::GuiPane2Pos.tostring(),
            "300".to_string(),
        );
        mod_ini.set_to(
            Some(GuiContext::section_name()),
            PropDef::GuiWindowTitle.tostring(),
            "app:default:to do".to_string(),
        );
        mod_ini.set_to(
            Some(BrowserPane::section_name()),
            PropDef::BrowserBackgroundLevel.tostring(),
            "200".to_string(),
        );
        mod_ini.set_to(
            Some(FeedContents::section_name()),
            PropDef::GuiList0SortColumn.tostring(),
            "0".to_string(),
        );
        mod_ini.set_to(
            Some(FeedContents::section_name()),
            PropDef::GuiList0SortAscending.tostring(),
            "true".to_string(),
        );
    */
    ret
}

fn check_or_create_folder(path: &String) {
    let mut dir_exists = false;
    if let Ok(metadata) = std::fs::metadata(&path) {
        dir_exists = metadata.is_dir();
    }
    if !dir_exists {
        if let Err(e) = std::fs::create_dir_all(&path) {
            error!("creating folder {} {:?}", &path, &e);
        }
    }
}
