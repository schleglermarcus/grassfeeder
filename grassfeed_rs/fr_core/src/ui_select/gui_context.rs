use crate::config::configmanager::ConfigManager;
use crate::config::APPLICATION_NAME;
use crate::controller::sourcetree::TREE_STATUS_COLUMN;
use crate::ui_select::select;
use crate::util::string_truncate;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use gui_layer::abstract_ui::GuiRunner;
use gui_layer::abstract_ui::ReceiverWrapper;
use gui_layer::abstract_ui::SenderWrapper;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::gui_values::PropDef;
use gui_layer::gui_values::PROPDEF_ARRAY;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

const TITLE_LENGTH_MAX: usize = 20;

pub struct GuiContext {
    values_store_adapter: UIAdapterValueStoreType,
    updater_adapter: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_runner: Rc<RefCell<dyn GuiRunner>>,
    configmanager_r: Rc<RefCell<ConfigManager>>,
    application_name: String,
    window_title: String,
}

impl Buildable for GuiContext {
    type Output = GuiContext;

    #[allow(clippy::type_complexity)]
    fn build(conf: Box<dyn BuildConfig>, appcontext: &AppContext) -> Self {
        let configman = (*appcontext).get_rc::<ConfigManager>().unwrap();
        let mut initvalues: HashMap<PropDef, String> = HashMap::default();
        for pd in PROPDEF_ARRAY {
            // TODO check if we need both     conf, configmanager
            let mut o_val = conf.get(&pd.tostring());
            if o_val.is_none() {
                o_val = (*configman).borrow().get_val(&pd.to_string());
            }
            if o_val.is_none() {
                o_val = (*configman).borrow().get_sys_val(&pd.to_string());
            }
            if let Some(val) = o_val {
                initvalues.insert(pd, val);
            }
        }
        // trace!("gui_context:   initvals={:#?}", &initvalues);
        let (m_v_store_a, ui_updater, g_runner): (
            UIAdapterValueStoreType,
            Rc<RefCell<dyn UIUpdaterAdapter>>,
            Rc<RefCell<dyn GuiRunner>>,
        ) = select::ui_select::init_gui(initvalues);
        (*m_v_store_a)
            .write()
            .unwrap()
            .set_tree_row_expand(0, TREE_STATUS_COLUMN, 1);
        GuiContext {
            values_store_adapter: m_v_store_a,
            updater_adapter: ui_updater,
            gui_runner: g_runner,
            configmanager_r: configman,
            application_name: APPLICATION_NAME.to_string(),
            window_title: String::default(),
        }
    }

    fn section_name() -> String {
        String::from("window")
    }
}

impl GuiContext {
    pub const CONF_RCS_VERSION: &'static str = "rcs_version";

    pub fn get_receiver_wrapper(&self) -> Rc<dyn ReceiverWrapper> {
        (*self.gui_runner).borrow().get_event_receiver()
    }
    pub fn get_sender_wrapper(&self) -> Arc<dyn SenderWrapper + Send + Sync + 'static> {
        (*self.gui_runner).borrow().get_event_sender()
    }

    pub fn get_values_adapter(&self) -> UIAdapterValueStoreType {
        self.values_store_adapter.clone()
    }
    pub fn get_updater_adapter(&self) -> Rc<RefCell<dyn UIUpdaterAdapter>> {
        self.updater_adapter.clone()
    }
    pub fn get_gui_runner(&self) -> Rc<RefCell<dyn GuiRunner>> {
        self.gui_runner.clone()
    }

    pub fn start(&self) {
        info!("GuiContext::start()  --> ");
        (*self.gui_runner).borrow().start();
    }

    pub fn stop(&self) {
        (*self.gui_runner).borrow_mut().stop();
    }

    pub fn set_conf_fontsize_manual_enable(&self, e: bool) {
        (*self.values_store_adapter)
            .write()
            .unwrap()
            .set_gui_property(PropDef::GuiFontSizeManualEnable, e.to_string());
        (*self.configmanager_r)
            .borrow()
            .set_val(&PropDef::GuiFontSizeManualEnable.to_string(), e.to_string());
    }

    pub fn set_conf_fontsize_manual(&self, s: i32) {
        (*self.values_store_adapter)
            .write()
            .unwrap()
            .set_gui_property(PropDef::GuiFontSizeManual, s.to_string());
        (*self.configmanager_r)
            .borrow()
            .set_val(&PropDef::GuiFontSizeManual.to_string(), s.to_string());
    }

    pub fn set_window_title(&mut self, current_title: String) {
        let mut t = string_truncate(current_title, TITLE_LENGTH_MAX);
        t = t.trim().to_string();
        let wtitle = if t.is_empty() {
            self.application_name.clone()
        } else {
            format!("{} - {}", t, self.application_name)
        };
        self.window_title = wtitle.clone();
        (*self.values_store_adapter)
            .write()
            .unwrap()
            .set_window_title(wtitle);
        (*self.updater_adapter).borrow().update_window_title();
    }

}

impl StartupWithAppContext for GuiContext {
    fn startup(&mut self, _ac: &AppContext) {}
}
