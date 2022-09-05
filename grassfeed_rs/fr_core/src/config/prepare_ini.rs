use crate::config::configmanager::ConfigManager;
use crate::controller::browserpane::BrowserPane;
use crate::controller::contentlist::FeedContents;
use crate::db::icon_repo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo;
use crate::ui_select::gui_context::GuiContext;
use context::Buildable;
use gui_layer::gui_values::PropDef;
use ini::Ini;
use std::fs;

pub struct GrassFeederConfig {
    pub path_config: String,
    pub path_cache: String,
    pub debug_mode: bool,
    pub version: String,
}

fn check_or_create_folder(path: &String) {
    let mut dir_exists = false;
    if let Ok(metadata) = fs::metadata(&path) {
        dir_exists = metadata.is_dir();
    }
    if !dir_exists {
        if let Err(e) = fs::create_dir_all(&path) {
            error!("creating folder {} {:?}", &path, &e);
        }
    }
}

pub fn prepare_config_by_path(gf_conf: &GrassFeederConfig) -> Ini {
    check_or_create_folder(&gf_conf.path_config);
    check_or_create_folder(&gf_conf.path_cache);

    let mut mod_ini = Ini::new();
    mod_ini.set_to(
        Some(icon_repo::IconRepo::section_name()),
        icon_repo::KEY_FOLDERNAME.to_string(),
        gf_conf.path_config.clone(),
    );
    mod_ini.set_to(
        Some(subscription_repo::SubscriptionRepo::section_name()),
        subscription_repo::KEY_FOLDERNAME.to_string(),
        gf_conf.path_config.clone(),
    );
    mod_ini.set_to(
        Some(ConfigManager::section_name()),
        ConfigManager::CONF_PATH_KEY.to_string(),
        format!("{}/config.ini", &gf_conf.path_config),
    );
    mod_ini.set_to(
        Some(ConfigManager::section_name()),
        ConfigManager::CONF_MODE_DEBUG.to_string(),
        gf_conf.debug_mode.to_string(),
    );
    /*
        mod_ini.set_to(
            Some(GuiContext::section_name()),
            GuiContext::CONF_RCS_VERSION.to_string(),
            gf_conf.version.clone(),
        );
    */
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
        PropDef::BrowserDir.tostring(),
        format!("{}/browser", &gf_conf.path_cache),
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
    mod_ini.set_to(
        Some(GuiContext::section_name()),
        PropDef::AppRcsVersion.tostring(),
        gf_conf.version.clone(),
    );
    debug!(
        "INI: {}={:?}",
        GuiContext::section_name(),
        mod_ini.section(Some(GuiContext::section_name()))
    );

    mod_ini.set_to(
        Some(MessagesRepo::section_name()),
        MessagesRepo::CONF_DB_KEY.to_string(),
        format!("{}/messages.db", &gf_conf.path_config),
    );

    mod_ini
}
