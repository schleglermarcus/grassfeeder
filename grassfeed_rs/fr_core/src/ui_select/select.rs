#[cfg(feature = "ui_gtk")]
pub mod ui_select {
    use fr_gtk::gtk_object_tree::GtkObjectTree;
    use gui_layer::abstract_ui::GuiRunner;
    use gui_layer::abstract_ui::KeyCodes;
    use gui_layer::abstract_ui::UIAdapterValueStoreType;
    use gui_layer::abstract_ui::UIUpdaterAdapter;
    use gui_layer::gui_values::PropDef;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::RwLock;
    use ui_gtk::gtkrunner::GtkRunner;
    use ui_gtk::ui_value_adapter::ModelValueStoreImpl;
    use ui_gtk::GtkBuilderType;
    // use fr_gtk::gdk_sys;

    pub fn init_gui(
        initvalues: HashMap<PropDef, String>,
    ) -> (
        UIAdapterValueStoreType,
        Rc<RefCell<dyn UIUpdaterAdapter>>,
        Rc<RefCell<dyn GuiRunner>>,
    ) {
        let mut g_o = GtkObjectTree::default();
        let mut m_v_store = ModelValueStoreImpl::new();
        g_o.initvalues = initvalues.clone();
        m_v_store.properties = initvalues;
        let m_v_store_a: UIAdapterValueStoreType = Arc::new(RwLock::new(m_v_store));
        let gtk_o2a: GtkBuilderType = Arc::new(Box::new(g_o));
        let mut runner = GtkRunner::new(gtk_o2a, m_v_store_a.clone());
        let ui_updater: Rc<RefCell<dyn UIUpdaterAdapter>> = runner.get_ui_updater();
        runner.init();
        let r_runner = Rc::new(RefCell::new(runner));
        (m_v_store_a, ui_updater, r_runner)
    }

    pub fn from_gdk_sys(code: isize) -> KeyCodes {
        ui_gtk::keyboard_codes::from_gdk_sys(code)
    }

    /*
        // https://gtk-rs.org/gtk3-rs/stable/latest/docs/gdk_sys/index.html
        #[deprecated]
        #[allow(unused_variables)]
        pub fn from_isize(code: isize) -> KeyCodes {
            match code as i32 {
                gdk_sys::GDK_KEY_Tab => KeyCodes::Tab,
                gdk_sys::GDK_KEY_ISO_Left_Tab => KeyCodes::ShiftTab,
                gdk_sys::GDK_KEY_KP_Tab => KeyCodes::Tab,
                gdk_sys::GDK_KEY_space => KeyCodes::Space,
                gdk_sys::GDK_KEY_Escape => KeyCodes::Escape,
                gdk_sys::GDK_KEY_KP_Enter | gdk_sys::GDK_KEY_ISO_Enter | gdk_sys::GDK_KEY_Return => {
                    KeyCodes::Enter
                }
                gdk_sys::GDK_KEY_F1 => KeyCodes::F1,
                gdk_sys::GDK_KEY_F2 => KeyCodes::F2,
                gdk_sys::GDK_KEY_F3 => KeyCodes::F3,
                gdk_sys::GDK_KEY_F4 => KeyCodes::F4,
                gdk_sys::GDK_KEY_Up => KeyCodes::CursorUp,
                gdk_sys::GDK_KEY_Down => KeyCodes::CursorDown,
                gdk_sys::GDK_KEY_Right => KeyCodes::CursorRight,
                gdk_sys::GDK_KEY_Left => KeyCodes::CursorLeft,
                gdk_sys::GDK_KEY_A => KeyCodes::Key_A,
                gdk_sys::GDK_KEY_a => KeyCodes::Key_a,
                gdk_sys::GDK_KEY_B => KeyCodes::Key_B,
                gdk_sys::GDK_KEY_b => KeyCodes::Key_b,
                gdk_sys::GDK_KEY_N => KeyCodes::Key_N,
                gdk_sys::GDK_KEY_n => KeyCodes::Key_n,
                gdk_sys::GDK_KEY_s => KeyCodes::Key_s,
                gdk_sys::GDK_KEY_v => KeyCodes::Key_v,
                gdk_sys::GDK_KEY_x => KeyCodes::Key_x,

                _ => KeyCodes::Nothing,
            }
        }

    */
}

#[cfg(not(feature = "ui_gtk"))]
pub mod ui_select {

    use crate::ui_select::uimock::UIMock;
    use gui_layer::abstract_ui::GuiRunner;
    use gui_layer::abstract_ui::KeyCodes;
    use gui_layer::abstract_ui::UIAdapterValueStoreType;
    use gui_layer::abstract_ui::UIUpdaterAdapter;
    use gui_layer::gui_values::PropDef;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[allow(clippy::type_complexity)]
    pub fn init_gui(
        _initvalues: HashMap<PropDef, String>,
    ) -> (
        UIAdapterValueStoreType,
        Rc<RefCell<dyn UIUpdaterAdapter>>,
        Rc<RefCell<dyn GuiRunner>>,
    ) {
        let mock = UIMock::new();
        info!("Using UI MOCK");
        (mock.val_sto(), mock.upd_adp(), mock.guirunner())
    }

    pub fn from_isize(code: isize) -> KeyCodes {
        match code {
            65289 => KeyCodes::Tab,
            _ => KeyCodes::Nothing,
        }
    }

	
    #[allow(dead_code)]
    pub fn from_gdk_sys(_code: isize) -> KeyCodes {
        KeyCodes::Nothing
    }
}
