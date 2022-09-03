mod logger_config;

use context::appcontext::AppContext;
use fr_core::config::configmanager::ConfigManager;
use fr_core::config::prepare_ini::prepare_config_by_path;
use fr_core::config::prepare_ini::GrassFeederConfig;
use fr_core::controller::browserpane::BrowserPane;
use fr_core::controller::contentdownloader::Downloader;
use fr_core::controller::contentlist::FeedContents;
use fr_core::controller::guiprocessor::GuiProcessor;
use fr_core::controller::sourcetree::SourceTreeController;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::timer::Timer;
use fr_core::ui_select::gui_context::GuiContext;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiEvents;
use std::cell::RefCell;
use std::rc::Rc;

//   TODO  use other  resource for  shutdown test

/*
// Storing the pane position  into file on shutown
// #[ignore]
#[test]
fn shutdown_store_ini() {
    {
        setup();
        let ini_r = Rc::new(RefCell::new(prepare_config_by_path(
            "../target/db_shutdown".to_string(),
            "../target/db_timer_uninit".to_string(),
        )));
        let mut appcontext = AppContext::new_with_ini(ini_r.clone());
        let mut cm = ConfigManager::new_with_ini(ini_r);
        cm.load_config_file();
        appcontext.store_ini(Rc::new(RefCell::new(cm.get_conf())));
        appcontext.store_obj(Rc::new(RefCell::new(cm)));
        appcontext.build::<Timer>();
        appcontext.build::<GuiContext>();
        appcontext.build::<SubscriptionRepo>();
        appcontext.build::<IconRepo>();
        appcontext.build::<FeedContentRepo>();
        appcontext.build::<MessagesRepo>();
        appcontext.build::<Downloader>();
        appcontext.build::<SourceTreeController>();
        appcontext.build::<BrowserPane>();
        appcontext.build::<FeedContents>();
        appcontext.build::<GuiProcessor>();
        let conf_name = "../target/config_shutdowntest.ini".to_string();
        let config_r = appcontext.get_rc::<ConfigManager>().unwrap();
        (*config_r)
            .borrow_mut()
            .set_conf_filename(conf_name.clone());
        let gp_r = appcontext.get_rc::<GuiProcessor>().unwrap();
        let gtk_rw_r = appcontext.get_rc::<GuiContext>().unwrap();
        let event_sender = (*gtk_rw_r).borrow().get_sender_wrapper();

        let timer_r: Rc<RefCell<dyn ITimer>> = appcontext.get_rc::<Timer>().unwrap();
        appcontext.startup();
        let pane_pos = 192;
        let _r = event_sender.send(GuiEvents::PanedMoved(0, pane_pos));
        let _r = event_sender.send(GuiEvents::WinDelete);
        (*gp_r).borrow_mut().process_event();
        (*gp_r).borrow_mut().process_jobs();
        (*timer_r).borrow_mut().main_loop();
        let ini2 = ini::Ini::load_from_file(conf_name.clone()).unwrap();
        let ppos_str = ini2.get_from(Some("window"), "GuiPane1Pos").unwrap();
        assert_eq!(ppos_str.to_string(), pane_pos.to_string());
    }

    let inuse = context::appcontext::AppCo fr_core::db::feed_content_repo::IN_USE.load(Ordering::Relaxed);
    assert_eq!(inuse, false);
    debug!("shutdown_store_ini - outer ");
}


*/

// #[ignore]
#[test]
fn add_folder_and_feed() {
    setup();

    let gfc = GrassFeederConfig {
        path_config: "../target/db_feedsource_add".to_string(),
        path_cache: "../target/db_feedsource_add".to_string(),
        debug_mode: true,
    };
    // "../target/db_feedsource_add".to_string(),        "../target/db_feedsource_add".to_string(),
    let ini_r = Rc::new(RefCell::new(prepare_config_by_path(&gfc)));
    let mut appcontext = AppContext::new_with_ini(ini_r.clone());
    let mut cm = ConfigManager::new_with_ini(ini_r);
    cm.load_config_file();
    appcontext.store_ini(Rc::new(RefCell::new(cm.get_conf())));
    appcontext.store_obj(Rc::new(RefCell::new(cm)));
    appcontext.build::<Timer>();
    appcontext.build::<GuiContext>();
    appcontext.build::<MessagesRepo>();
    appcontext.build::<SubscriptionRepo>();
    appcontext.build::<IconRepo>();
    appcontext.build::<Downloader>();
    appcontext.build::<SourceTreeController>();
    appcontext.build::<BrowserPane>();
    appcontext.build::<FeedContents>();
    appcontext.build::<GuiProcessor>();
    appcontext.startup();
    let subs_r: Rc<RefCell<dyn ISubscriptionRepo>> =
        appcontext.get_rc::<SubscriptionRepo>().unwrap();
    let gui_c_r = appcontext.get_rc::<GuiContext>().unwrap();
    let event_sender = (*gui_c_r).borrow().get_sender_wrapper();

    let msg_r_r = appcontext.get_rc::<MessagesRepo>().unwrap();
    let _r = (*msg_r_r).borrow().insert(&MessageRow::default());

    let mut payload: Vec<AValue> = Vec::default();
    payload.push(AValue::ASTR("folder2".to_string()));
    let _r = event_sender.send(GuiEvents::DialogData("new-folder".to_string(), payload));
    let mut payload: Vec<AValue> = Vec::default();
    payload.push(AValue::ASTR("tests/data/gui_proc_rss2_v1.rss".to_string()));
    payload.push(AValue::ASTR("name-proc2".to_string()));
    let gp_r = appcontext.get_rc::<GuiProcessor>().unwrap();
    let _r = event_sender.send(GuiEvents::DialogData("new-feedsource".to_string(), payload));
    for _a in 0..2 {
        let mut gp = (*gp_r).borrow_mut();
        gp.process_event();
        gp.process_jobs();
    }
    let entries = (*(subs_r.borrow_mut())).get_all_entries();
    // for e in &entries {        debug!("SUB={:?}", e);    }
    assert_eq!(entries.len(), 4); // 2 default entries, one folder, one regular entry
    if false {
        trace!("");
    }
}

// ------------------------------------

#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_logger();
    });
}
