use core::slice::Iter;
use flume::Receiver;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::GuiRunner;
use gui_layer::abstract_ui::GuiTreeItem;
use gui_layer::abstract_ui::ReceiverWrapper;
use gui_layer::abstract_ui::TreeRowExpand;
use gui_layer::abstract_ui::UIAdapterValueStore;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UISenderWrapper;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::abstract_ui::UIUpdaterMarkWidgetType;
use gui_layer::gui_values::PropDef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

#[allow(unused_imports)]
use log::*;

#[derive(Clone)]
#[allow(dead_code)]
pub struct UIMock {
    pub ada_val_sto_a: UIAdapterValueStoreType,
    upd_ada: Rc<RefCell<UpdAda>>,
    pub gui_runner: Rc<RefCell<dyn GuiRunner>>,
    pub event_receiver: Rc<dyn ReceiverWrapper>,
    pub event_sender: Arc<dyn UISenderWrapper>,
}

#[allow(dead_code)]
impl UIMock {
    pub fn new() -> Self {
        let (ev_s, ev_r) = flume::bounded::<GuiEvents>(10);
        let r_ev_re: Rc<dyn ReceiverWrapper> = Rc::new(ReceiverWrapperImpl(ev_r));
        let ev_se_wr = Arc::new(SenderWrapperImpl(ev_s));
        let sto_a = Arc::new(RwLock::new(AdValSto::default()));
        let upd_adapter = UpdAda::new(sto_a.clone(), ev_se_wr.clone());
        let upd_adapter_r = Rc::new(RefCell::new(upd_adapter));
        let mockrunner = MockRunner::new(upd_adapter_r.clone(), r_ev_re.clone(), ev_se_wr.clone());
        UIMock {
            ada_val_sto_a: sto_a,
            upd_ada: upd_adapter_r,
            gui_runner: Rc::new(RefCell::new(mockrunner)),
            event_receiver: r_ev_re,
            event_sender: ev_se_wr,
        }
    }

    pub fn rec_wr(&self) -> Rc<dyn ReceiverWrapper> {
        (*self.gui_runner).borrow().get_event_receiver()
    }

    pub fn val_sto(&self) -> UIAdapterValueStoreType {
        self.ada_val_sto_a.clone()
    }
    pub fn upd_adp(&self) -> Rc<RefCell<dyn UIUpdaterAdapter>> {
        self.upd_ada.clone()
    }
    pub fn guirunner(&self) -> Rc<RefCell<dyn GuiRunner>> {
        self.gui_runner.clone()
    }
}

impl Default for UIMock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Clone)]
struct AdValSto {
    pub text_entry_text: String,
    pub list0: Vec<Vec<AValue>>,
    pub expand_data: Vec<(usize, u32)>,
}
impl AdValSto {}

impl UIAdapterValueStore for AdValSto {
    fn set_text_entry(&mut self, _index: u8, nt: String) {
        self.text_entry_text = nt;
    }
    fn get_text_entry(&self, _index: u8) -> Option<String> {
        Some(self.text_entry_text.clone())
    }

    ///  insert a new  Tree-Item the given position by path
    fn insert_tree_item(&mut self, _path: &[u16], _treevalues: &[AValue]) {}
    fn get_tree_item(&self, _path: &[u16]) -> Vec<AValue> {
        Vec::default()
    }
    fn get_tree_root(&self) -> GuiTreeItem {
        GuiTreeItem::default()
    }

    fn replace_tree_item(&mut self, _path: &[u16], _treevalues: &[AValue]) {}
    fn clear_tree(&mut self, _tree_index: u8) {}

    ///  insert a new     list item
    fn insert_list_item(&mut self, _list_index: u8, _list_position: i32, _values: &[AValue]) {}
    fn clear_list(&mut self, _list_index: u8) {}
    fn get_list_item(&self, _list_index: u8, _list_position: i32) -> Option<Vec<AValue>> {
        None
    }

    fn get_list_iter(&self, _list_index: u8) -> Iter<Vec<AValue>> {
        self.list0.iter()
    }

    fn set_text_view(&mut self, _index: u8, _newtext: String) {}
    fn get_text_view(&self, _index: u8) -> Option<String> {
        None
    }

    fn set_web_view_text(&mut self, _index: u8, _newtext: String) {}
    fn get_web_view_text(&self, _index: u8) -> Option<String> {
        None
    }

    fn set_spinner_active(&mut self, _a: bool) {}
    fn is_spinner_active(&self) -> bool {
        false
    }

    fn set_tree_row_expand(&mut self, idx: usize, column: usize, bitmask: u32) {
        while self.expand_data.len() < (idx + 1) {
            self.expand_data.push((0, 0));
        }
        self.expand_data[idx] = (column, bitmask);
    }
    fn get_tree_row_expand(&self, idx: usize) -> (usize, u32) {
        self.expand_data[idx]
    }
    fn is_tree_row_expanded(&self, idx: usize, gti: &GuiTreeItem) -> bool {
        if idx < self.expand_data.len() {
            return TreeRowExpand::is_expanded(gti, self.expand_data[idx]);
        }
        false
    }

    fn set_label_text(&mut self, _index: u8, _newtext: String) {}
    fn get_label_text(&self, _index: u8) -> Option<&String> {
        None
    }

    fn set_dialog_data(&mut self, _idx: u8, _values: &[AValue]) {
        // trace!("mock: set_dialog_data  {:?}", _values);
    }
    fn get_dialog_data(&self, _idx: u8) -> Option<&Vec<AValue>> {
        None
    }

    fn set_gui_property(&mut self, _name: PropDef, _value: String) {
        unimplemented!()
    }
    fn get_gui_property_or(&self, _name: PropDef, default: String) -> String {
        default
    }
    fn get_gui_int_or(&self, _name: PropDef, default: isize) -> isize {
        default
    }
    fn set_gui_properties(&mut self) -> HashMap<PropDef, String> {
        HashMap::<PropDef, String>::default()
    }
    fn get_gui_bool(&self, _name: PropDef) -> bool {
        false
    }

    fn set_window_title(&mut self, _t: String) {}
    fn get_window_title(&self) -> String {
        String::default()
    }
    fn set_linkbutton_text(&mut self, _index: u8, _text_uri: (String, String)) {}

    fn get_linkbutton_text(&self, _index: u8) -> Option<&(String, String)> {
        None
    }

    fn set_window_icon(&mut self, _icon_compressed: String) {}
    fn get_window_icon(&mut self) -> String {
        String::default()
    }

    fn set_label_tooltip(&mut self, _index: u8, _newtext: String) {}
    fn get_label_tooltip(&self, _index: u8) -> Option<&String> {
        None
    }

    fn get_list_length(&self, _list_index: u8) -> usize {
        0
    }

    fn get_window_minimized(&self) -> bool {
        false
    }

    fn set_window_minimized(&mut self, _active: bool) {}
}

// #[derive(Default)]
struct UpdAda {
    ada_val_sto_a: UIAdapterValueStoreType,
    r_event_sender: Arc<dyn UISenderWrapper>,
}
impl UpdAda {
    fn new(sto: UIAdapterValueStoreType, r_se: Arc<dyn UISenderWrapper>) -> Self {
        UpdAda {
            ada_val_sto_a: sto,
            r_event_sender: r_se,
        }
    }
}

impl UIUpdaterAdapter for UpdAda {
    fn update_tree(&self, _tree_index: u8) {}
    fn update_tree_single(&self, _tree_index: u8, _path: &[u16]) {}
    fn update_tree_partial(&self, _tree_index: u8, _path: &[u16]) {}
    fn tree_set_cursor(&self, _list_index: u8, _path: Vec<u16>) {}
    fn update_list(&self, _list_index: u8) {}
    fn update_list_single(&self, _list_index: u8, _list_position: u32) {}
    fn update_list_some(&self, _list_index: u8, _list_position: &[u32]) {}
    fn update_text_view(&self, _nr: u8) {}
    fn update_text_entry(&self, _nr: u8) {
        if let Some(te) = (*self.ada_val_sto_a).read().unwrap().get_text_entry(_nr) {
            (*self.r_event_sender)
                .send(GuiEvents::DialogEditData("e".to_string(), AValue::ASTR(te)));
        }
    }
    fn update_label(&self, _nr: u8) {
        unimplemented!()
    }
    fn update_label_markup(&self, _nr: u8) {}
    fn update_dialog(&self, _nr: u8) {
        // trace!("mock: update_dialog {}", _nr);
    }
    fn show_dialog(&self, _nr: u8) {
        unimplemented!()
    }
    fn update_linkbutton(&self, _nr: u8) {
        unimplemented!()
    }
    fn update_paned_pos(&self, _nr: u8, _pos: i32) {}
    fn widget_mark(&self, _typ: UIUpdaterMarkWidgetType, _sw_idx: u8, _mark: u8) {}
    fn grab_focus(&self, _typ: UIUpdaterMarkWidgetType, _idx: u8) {}
    fn list_set_cursor(&self, _list_index: u8, _db_id: isize, _column: u8, _scrollpos: i8) {}
    fn update_window_title(&self) {}
    fn update_window_icon(&self) {}
    fn update_web_view(&self, _nr: u8) {}
    fn update_web_view_plain(&self, _nr: u8) {}
    fn clipboard_set_text(&self, _s: String) {}
    fn web_view_remove(&self, _idx: u8, _fs_man: Option<u8>) {}

    fn memory_conserve(&self, _act: bool) {}
    // fn update_systray_indicator(&self, _enable: bool) {}
    fn update_window_minimized(&self, _mini: bool, _ev_time: u32) {}
    fn store_image(&self, _idx: i32, _img: String) {}

    fn toolbutton_set_sensitive(&self, _idx: u8, _sens: bool) {
        unimplemented!()
    }
    fn button_set_sensitive(&self, _idx: u8, _sens: bool) {
        unimplemented!()
    }
}

struct MockRunner {
    up_ad: Rc<RefCell<UpdAda>>,
    event_receiver: Rc<dyn ReceiverWrapper>,
    event_sender: Arc<dyn UISenderWrapper + Send + Sync + 'static>,
}
impl MockRunner {
    fn new(
        up_ada: Rc<RefCell<UpdAda>>,
        r_ev_re: Rc<dyn ReceiverWrapper>,
        r_ev_se: Arc<dyn UISenderWrapper + Send + Sync + 'static>,
    ) -> Self {
        MockRunner {
            up_ad: up_ada,
            event_receiver: r_ev_re,
            event_sender: r_ev_se,
        }
    }
}
impl GuiRunner for MockRunner {
    fn init(&mut self) {}
    fn start(&self) {}
    fn stop(&mut self) {}
    fn get_event_receiver(&self) -> Rc<dyn ReceiverWrapper> {
        self.event_receiver.clone()
    }
    fn get_event_sender(&self) -> Arc<dyn UISenderWrapper + Send + Sync + 'static> {
        self.event_sender.clone()
    }
    fn get_ui_updater(&self) -> Rc<RefCell<dyn UIUpdaterAdapter>> {
        self.up_ad.clone()
    }
}

struct ReceiverWrapperImpl(Receiver<GuiEvents>);

impl ReceiverWrapper for ReceiverWrapperImpl {
    fn get_event_try(&self) -> GuiEvents {
        if let Ok(e) = self.0.try_recv() {
            return e;
        }
        GuiEvents::None
    }

    fn get_event(&self) -> GuiEvents {
        if let Ok(e) = self.0.recv() {
            return e;
        }
        GuiEvents::None
    }

    fn get_event_timeout(&self, timeout_ms: u64) -> GuiEvents {
        if let Ok(e) = self.0.recv_timeout(Duration::from_millis(timeout_ms)) {
            return e;
        }
        GuiEvents::None
    }

    fn get_len(&self) -> usize {
        self.0.len()
    }
}

struct SenderWrapperImpl(Sender<GuiEvents>);
impl UISenderWrapper for SenderWrapperImpl {
    fn send(&self, ev: GuiEvents) {
        let _r = self.0.send(ev);
    }
}
