mod logger_config;

use fr_core::config::init_system::combine_config_path;
use fr_core::config::init_system::GrassFeederConfig;
use fr_core::controller::guiprocessor::GuiProcessor;
use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::timer::ITimer;
use fr_core::timer::Timer;
use fr_core::ui_select::gui_context::GuiContext;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiEvents;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::Ordering;

// 1: Storing the pane position  into file on shutown
// 2:  Detect circular dependency among context objects that prevent freeing them
#[test]
fn shutdown_store_ini() {
    setup();
    {
        let folder = "../target/db_shutdown".to_string();
        let gf_conf = GrassFeederConfig {
            path_config: folder.clone(),
            path_cache: folder.clone(),
            debug_mode: true,
            version: "db_shutdown".to_string(),
        };
        let appcontext = fr_core::config::init_system::start(gf_conf);
        let gp_r = appcontext.get_rc::<GuiProcessor>().unwrap();
        let gtk_rw_r = appcontext.get_rc::<GuiContext>().unwrap();
        let event_sender = (*gtk_rw_r).borrow().get_sender_wrapper();
        let timer_r: Rc<RefCell<dyn ITimer>> = appcontext.get_rc::<Timer>().unwrap();
        let pane_pos = 192;
        let _r = event_sender.send(GuiEvents::PanedMoved(0, pane_pos));
        let _r = event_sender.send(GuiEvents::WinDelete);
        (*gp_r).borrow_mut().process_event();
        (*gp_r).borrow_mut().process_jobs();
        (*timer_r).borrow_mut().main_loop();
        let conf_filename = combine_config_path(&folder);
        let lines = std::fs::read_to_string(conf_filename).unwrap();
        let dec_r: serde_json::Result<HashMap<String, String>> = serde_json::from_str(&lines);
        let initvalues = dec_r.unwrap();
        let r = initvalues.get("GuiPane1Pos");
        assert!(r.is_some());
        assert!(pane_pos.to_string().eq(r.unwrap()));
        // debug!("GuiPane1Pos={:?}", r);
    }
    let inuse = fr_core::ui_select::gui_context::IN_USE.load(Ordering::Relaxed);
    assert_eq!(inuse, false);
}

#[test]
fn add_folder_and_feed() {
    setup();
    let gf_conf = GrassFeederConfig {
        path_config: "../target/db_feedsource_add".to_string(),
        path_cache: "../target/db_feedsource_add".to_string(),
        debug_mode: true,
        version: "add_folder_and_feed".to_string(),
    };
    let appcontext = fr_core::config::init_system::start(gf_conf);
    let subs_r: Rc<RefCell<dyn ISubscriptionRepo>> =
        appcontext.get_rc::<SubscriptionRepo>().unwrap();
    (*subs_r).borrow().scrub_all_subscriptions();
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
    // for e in &entries {        trace!("SUB={:?}", e);    }
    assert_eq!(entries.len(), 2); // 0 default entries: scrubbed after startup, one folder, one regular entry
}

// ------------------------------------

#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(
            logger_config::QuietFlags::Db as u64 | logger_config::QuietFlags::Config as u64,
        );
    });
    if false {
        trace!("");
    }
}
