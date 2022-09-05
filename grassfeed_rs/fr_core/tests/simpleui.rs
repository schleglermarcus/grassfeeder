mod logger_config;

// mod uimock;
// use crate::ui_select::uimock;

#[cfg(feature = "ui-gtk")]
mod ui_select {
    use fr_gtk::hellobutton::HelloButtonUI;
    use gui_layer::abstract_ui::GuiRunner;
    use gui_layer::abstract_ui::ReceiverWrapper;
    use gui_layer::abstract_ui::UIAdapterValueStoreType;
    use gui_layer::abstract_ui::UIUpdaterAdapter;
    use resources::windowconfig::GtkWindowConfig;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::RwLock;
    use ui_gtk::gtkrunner::GtkRunner;
    use ui_gtk::ui_value_adapter::ModelValueStoreImpl;
    use ui_gtk::GtkBuilderType;

    pub fn init_gui_stuff() -> (
        Rc<dyn ReceiverWrapper>,
        UIAdapterValueStoreType,
        Rc<RefCell<dyn UIUpdaterAdapter>>,
    ) {
        let m_v_st = ModelValueStoreImpl::new();
        let m_v_st_a: UIAdapterValueStoreType = Arc::new(RwLock::new(m_v_st));

        let win_conf = GtkWindowConfig::default();
        let gtk_o2a: GtkBuilderType = Arc::new(Box::new(HelloButtonUI {}));
        let mut runner = GtkRunner::new(gtk_o2a, win_conf, m_v_st_a.clone());
        let ui_updater: Rc<RefCell<dyn UIUpdaterAdapter>> = runner.get_ui_updater();
        let ev_rec_w: Rc<dyn ReceiverWrapper> = runner.get_event_receiver();

        runner.init();
        runner.start();

        (ev_rec_w, m_v_st_a, ui_updater)
    }
}

#[cfg(not(feature = "ui-gtk"))]
mod ui_select {
    use fr_core::ui_select::uimock::UIMock;
    use gui_layer::abstract_ui::ReceiverWrapper;
    use gui_layer::abstract_ui::UIAdapterValueStoreType;
    use gui_layer::abstract_ui::UIUpdaterAdapter;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub fn init_gui_stuff() -> (
        Rc<dyn ReceiverWrapper>,
        UIAdapterValueStoreType,
        Rc<RefCell<dyn UIUpdaterAdapter>>,
    ) {
        let mock = UIMock::new(
		);
        (mock.rec_wr(), mock.val_sto(), mock.upd_adp())
    }
}

use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiEvents;

// #[ignore]
#[test]
fn roundtrip_with_text_entry() {
    setup();
    let payload = "Hello2".to_string();
    let (ev_rec_w, m_v_store_a, ui_updater) = ui_select::init_gui_stuff();
    m_v_store_a
        .write()
        .unwrap()
        .set_text_entry(0, payload.clone());
    (*ui_updater).borrow().update_text_entry(0);

    let mut expected: Vec<GuiEvents> = Vec::default();
    expected.push(GuiEvents::DialogEditData(
        "e".to_string(),
        AValue::ASTR(payload.clone()),
    ));
    expected.push(GuiEvents::None);
    //  GTK takes time to fire up !
    for exp_ev in expected {
        let ev = ev_rec_w.get_event_timeout(500);
        // debug!("checking: exp={:?}  == {:?}", &exp_ev, &ev);
        assert_eq!(&ev, &exp_ev);
    }
}

// ------------------------------------
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
		let _r = logger_config::setup_fern_logger(0);
    });
}
