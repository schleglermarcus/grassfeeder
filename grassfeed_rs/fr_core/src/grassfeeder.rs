use crate::config;
use crate::config::configmanager::ConfigManager;
use crate::config::prepare_ini::GrassFeederConfig;
use crate::controller::browserpane::BrowserPane;
use crate::controller::contentdownloader::Downloader;
use crate::controller::contentlist::FeedContents;
use crate::controller::guiprocessor::GuiProcessor;
use crate::controller::sourcetree::SourceTreeController;
use crate::db::icon_repo::IconRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::opml::opmlreader::OpmlReader;
use crate::timer::ITimer;
use crate::timer::Timer;
use crate::ui_select::gui_context::GuiContext;
use config::prepare_ini::prepare_config_by_path;
use context::appcontext::AppContext;
use ini::Ini;
use std::cell::RefCell;
use std::rc::Rc;

pub fn start(conf: GrassFeederConfig) -> AppContext {
    let ini_r: Rc<RefCell<Ini>> = Rc::new(RefCell::new(prepare_config_by_path(&conf)));

    let mut cm = ConfigManager::new_with_ini(ini_r.clone());

    cm.load_config_file();

    let mut appcontext = AppContext::new_with_ini(ini_r.clone());

        appcontext.store_ini(Rc::new(RefCell::new(cm.get_conf())));

    appcontext.store_obj(Rc::new(RefCell::new(cm)));
    appcontext.build::<Timer>();
    appcontext.build::<GuiContext>();
    appcontext.build::<SubscriptionRepo>();
    appcontext.build::<IconRepo>();
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

//
