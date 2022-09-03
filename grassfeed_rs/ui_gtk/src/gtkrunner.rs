use crate::runner_internal::GtkRunnerInternal;
use crate::DialogDataDistributor;
use crate::GtkBuilderType;
use crate::GtkObjects;
use crate::IntCommands;
use flume::Receiver;
use flume::Sender;
use gtk::Application;
use gtk::Button;
use gtk::CellRendererSpinner;
use gtk::Dialog;
use gtk::Entry;
use gtk::Label;
use gtk::LinkButton;
use gtk::ListStore;
use gtk::Paned;
use gtk::ScrolledWindow;
use gtk::TextView;
use gtk::TreeStore;
use gtk::TreeView;
use gtk::TreeViewColumn;
use gtk::Window;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::GuiRunner;
use gui_layer::abstract_ui::ReceiverWrapper;
use gui_layer::abstract_ui::SenderWrapper;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::abstract_ui::UIUpdaterMarkWidgetType;
use gui_layer::gui_values::PropDef;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use webkit2gtk::WebContext;
use webkit2gtk::WebView;

const EVENT_QUEUE_SIZE: usize = 1000;
const INTERNAL_QUEUE_SIZE: usize = 600;
const INTERNAL_QUEUE_SEND_DURATION: Duration = Duration::from_millis(200);

pub struct GtkRunner {
    thread_gui_handle: Option<thread::JoinHandle<()>>,
    gui_event_sender: Sender<GuiEvents>,
    gui_event_receiver: Receiver<GuiEvents>,
    gui_command_sender: Sender<IntCommands>,
    gui_command_receiver: Receiver<IntCommands>,
    gtk_builder: GtkBuilderType,
    model_value_store: UIAdapterValueStoreType,
    ui_upd_adapter: Rc<RefCell<dyn UIUpdaterAdapter>>,
}

impl GtkRunner {
    pub fn new(builder: GtkBuilderType, m_v_st: UIAdapterValueStoreType) -> Self {
        let (ev_s, ev_r) = flume::bounded::<GuiEvents>(EVENT_QUEUE_SIZE);
        let (co_s, co_r) = flume::bounded::<IntCommands>(INTERNAL_QUEUE_SIZE);
        let co_se = co_s.clone();
        GtkRunner {
            thread_gui_handle: Option::None,
            gui_event_sender: ev_s,
            gui_event_receiver: ev_r,
            gui_command_sender: co_s,
            gui_command_receiver: co_r,
            gtk_builder: builder,
            model_value_store: m_v_st,
            ui_upd_adapter: Rc::new(RefCell::new(UIUpdaterAdapterImpl::new(co_se))),
        }
    }
}

impl GuiRunner for GtkRunner {
    fn init(&mut self) {
        let ev_se = self.gui_event_sender.clone();
        let ev_se2 = self.gui_event_sender.clone();
        let co_re2 = self.gui_command_receiver.clone();
        let builder_c = self.gtk_builder.clone();
        let win_title;
        let win_width;
        let win_height;
        let app_url;
        {
            let mvs = (*self.model_value_store).read().unwrap();
            win_title =
                mvs.get_gui_property_or(PropDef::GuiWindowTitle, "title-default".to_string());
            win_width = mvs.get_gui_int_or(PropDef::GuiWindowWidth, 200) as i32;
            win_height = mvs.get_gui_int_or(PropDef::GuiWindowHeight, 100) as i32;
            app_url = mvs.get_gui_property_or(PropDef::AppUrl, "some.app.url".to_string());
        }
        let m_v_st_c = self.model_value_store.clone();
        let child = thread::Builder::new()
            .name("TGUI".to_string())
            .spawn(move || {
                let mut runner_i = GtkRunnerInternal::new(ev_se);
                runner_i.init(&builder_c, win_title, win_width, win_height, app_url);
                ev_se2.send(GuiEvents::InternalStarted).unwrap();
                GtkRunnerInternal::add_timeout_loop(
                    co_re2.clone(),
                    runner_i.gtk_objects.clone(),
                    m_v_st_c,
                );
                if let Ok(cmd) = co_re2.recv() {
                    if cmd == IntCommands::START {
                        runner_i.run();
                    } else {
                        error!(
                            "GtkRunner Expected Intcommands:start, but got wrong cmd {:?}",
                            cmd
                        );
                    }
                }
            })
            .unwrap();
        self.thread_gui_handle = Some(child);
        if let Ok(ev) = self.gui_event_receiver.recv() {
            match ev {
                GuiEvents::InternalStarted => (),
                _ => error!("gtkrunner:init, got other event"),
            }
        }
    }

    fn start(&self) {
        self.gui_command_sender.send(IntCommands::START).unwrap();
    }

    fn stop(&mut self) {
        self.gui_command_sender.send(IntCommands::STOP).unwrap();
        if let Some(h) = self.thread_gui_handle.take() {
            match h.join() {
                Ok(()) => {
                    // trace!("gtkrunner stop join ok ");
                }
                Err(e) => {
                    error!("gtkrunner stop join {:?}", e);
                }
            }
        }
    }

    fn get_event_receiver(&self) -> Rc<dyn ReceiverWrapper> {
        Rc::new(ReceiverWrapperImpl(self.gui_event_receiver.clone()))
    }

    fn get_event_sender(&self) -> Arc<dyn SenderWrapper + Send + Sync + 'static> {
        Arc::new(SenderWrapperImpl(self.gui_event_sender.clone()))
    }

    fn get_ui_updater(&self) -> Rc<RefCell<dyn UIUpdaterAdapter>> {
        self.ui_upd_adapter.clone()
    }
}

#[derive(Default)]
pub struct GtkObjectsImpl {
    pub window: Option<Window>,
    pub application: Option<Application>,
    pub buttons: Vec<Button>,
    pub tree_stores: Vec<TreeStore>,
    pub tree_views: Vec<TreeView>,
    pub tree_stores_max_columns: Vec<u8>,
    pub list_stores: Vec<ListStore>,
    pub list_stores_max_columns: Vec<u8>,
    pub list_views: Vec<TreeView>,
    pub text_views: Vec<TextView>,
    pub web_contexts: Vec<WebContext>,
    pub web_views: Vec<WebView>,
    pub text_entries: Vec<Entry>,
    pub c_r_spinner_w: Option<(CellRendererSpinner, TreeViewColumn)>,
    pub labels: Vec<Label>,
    pub dialogs: Vec<Dialog>,
    pub linkbuttons: Vec<LinkButton>,
    pub boxes: Vec<gtk::Box>,
    pub paneds: Vec<Paned>,
    pub scrolledwindows: Vec<ScrolledWindow>,
    dialogdata_dist: Option<DialogDataDistributor>,
}

/// may not be Send
impl GtkObjects for GtkObjectsImpl {
    fn get_window(&self) -> Option<Window> {
        self.window.clone()
    }
    fn set_window(&mut self, w: &Window) {
        self.window.replace(w.clone());
    }

    fn get_application(&self) -> Option<Application> {
        self.application.clone()
    }
    fn set_application(&mut self, a: &Application) {
        self.application.replace(a.clone());
    }

    fn get_tree_store(&self, index: usize) -> Option<&gtk::TreeStore> {
        self.tree_stores.get(index)
    }

    // fn add_tree_store(&mut self, ts: &gtk::TreeStore) {
    //     self.tree_stores.push(ts.clone());
    // }

    fn set_tree_store(&mut self, idx: u8, ts: &gtk::TreeStore) {
        if self.tree_stores.len() < idx as usize + 1 {
            self.tree_stores
                .resize(idx as usize + 1, TreeStore::new(&[glib::Type::BOOL]));
        }
        self.tree_stores[idx as usize] = ts.clone();
    }

    fn get_tree_view(&self, list_index: usize) -> Option<&gtk::TreeView> {
        self.tree_views.get(list_index)
    }
    fn set_tree_view(&mut self, idx: u8, tv: &gtk::TreeView) {
        if self.tree_views.len() < idx as usize + 1 {
            self.tree_views.resize(idx as usize + 1, TreeView::new());
        }
        self.tree_views[idx as usize] = tv.clone();
    }

    fn get_tree_store_max_columns(&self, index: usize) -> u8 {
        match self.tree_stores_max_columns.get(index) {
            Some(mc) => *mc,
            None => 0,
        }
    }
    fn set_tree_store_max_columns(&mut self, tree_index: usize, max_col: u8) {
        if self.tree_stores_max_columns.len() < tree_index + 1 {
            self.tree_stores_max_columns.resize(tree_index + 1, 0);
        }
        self.tree_stores_max_columns[tree_index] = max_col;
    }

    fn get_list_store(&self, list_index: usize) -> Option<&gtk::ListStore> {
        if self.list_stores.len() < list_index + 1 {
            error!("list not there yet: {}", list_index);
            return None;
        }
        self.list_stores.get(list_index)
    }

    fn set_list_store(&mut self, idx: u8, store: &gtk::ListStore) {
        if self.list_stores.len() < idx as usize + 1 {
            self.list_stores
                .resize(idx as usize + 1, ListStore::new(&[glib::Type::BOOL]));
        }
        self.list_stores[idx as usize] = store.clone();
    }

    fn get_list_store_max_columns(&self, list_index: usize) -> u8 {
        if self.list_stores_max_columns.len() < list_index + 1 {
            error!("list_stores_max_columns not there yet: {}", list_index);
            return 0;
        }
        self.list_stores_max_columns[list_index]
    }

    fn set_list_store_max_columns(&mut self, list_index: usize, mc: u8) {
        if self.list_stores_max_columns.len() < list_index + 1 {
            self.list_stores_max_columns.resize(list_index + 1, 0);
        }
        self.list_stores_max_columns[list_index] = mc;
    }

    // fn get_list_view(&self, list_index: usize) -> Option<&gtk::TreeView> {
    //     self.list_views.get(list_index)
    // }

    fn get_text_view(&self, index: usize) -> Option<&gtk::TextView> {
        self.text_views.get(index)
    }
    fn add_text_view(&mut self, tv: &gtk::TextView) {
        self.text_views.push(tv.clone());
    }

    fn get_web_view(&self, index: u8) -> Option<&WebView> {
        self.web_views.get(index as usize)
    }
    fn add_web_view(&mut self, wv: &WebView) {
        self.web_views.push(wv.clone());
    }

    fn get_web_context(&self, index: u8) -> Option<&WebContext> {
        self.web_contexts.get(index as usize)
    }
    fn add_web_context(&mut self, wc: &WebContext) {
        self.web_contexts.push(wc.clone());
    }

    fn get_text_entry(&self, index: u8) -> Option<&Entry> {
        self.text_entries.get(index as usize)
    }
    fn add_text_entry(&mut self, e: &gtk::Entry) {
        self.text_entries.push(e.clone());
    }
    fn set_text_entry(&mut self, idx: u8, e: &gtk::Entry) {
        if self.text_entries.len() < idx as usize + 1 {
            self.text_entries.resize(idx as usize + 1, Entry::new());
        }
        self.text_entries[idx as usize] = e.clone();
    }

    fn get_buttons(&self) -> Vec<gtk::Button> {
        self.buttons.clone()
    }
    fn add_button(&mut self, e: &gtk::Button) {
        self.buttons.push(e.clone());
    }

    fn get_spinner_w(&self) -> Option<(gtk::CellRendererSpinner, gtk::TreeViewColumn)> {
        self.c_r_spinner_w.clone()
    }
    fn set_spinner_w(&mut self, widgets: (gtk::CellRendererSpinner, gtk::TreeViewColumn)) {
        self.c_r_spinner_w.replace(widgets);
    }

    fn get_label(&self, idx: u8) -> Option<&gtk::Label> {
        self.labels.get(idx as usize)
    }

    fn add_label(&mut self, l: &gtk::Label) {
        self.labels.push(l.clone());
    }

    fn set_label(&mut self, idx: u8, l: &gtk::Label) {
        if self.labels.len() < idx as usize + 1 {
            self.labels.resize(idx as usize + 1, Label::new(None));
        }
        self.labels[idx as usize] = l.clone();
    }

    fn get_dialog(&self, idx: u8) -> Option<&gtk::Dialog> {
        if self.dialogs.len() < idx as usize + 1 {
            error!("dialog not there yet: {}", idx);
            return None;
        }
        self.dialogs.get(idx as usize)
    }

    fn set_dialog(&mut self, idx: u8, d: &gtk::Dialog) {
        if self.dialogs.len() < idx as usize + 1 {
            self.dialogs.resize(idx as usize + 1, Dialog::new());
        }
        self.dialogs[idx as usize] = d.clone();
    }

    fn set_dddist(&mut self, ddd: DialogDataDistributor) {
        self.dialogdata_dist = Some(ddd);
    }

    fn get_dddist(&self) -> &Option<DialogDataDistributor> {
        &self.dialogdata_dist
    }

    fn get_linkbutton(&self, idx: u8) -> Option<&LinkButton> {
        if self.linkbuttons.len() < idx as usize + 1 {
            error!("linkbutton not there yet: {}", idx);
            return None;
        }
        self.linkbuttons.get(idx as usize)
    }

    fn add_linkbutton(&mut self, e: &LinkButton) {
        self.linkbuttons.push(e.clone());
    }
    fn set_linkbutton(&mut self, idx: u8, l: &LinkButton) {
        if self.linkbuttons.len() < idx as usize + 1 {
            self.linkbuttons
                .resize(idx as usize + 1, LinkButton::new(""));
        }
        self.linkbuttons[idx as usize] = l.clone();
    }

    fn get_box(&self, idx: u8) -> Option<&gtk::Box> {
        if self.boxes.len() < idx as usize + 1 {
            error!("box not there yet: {}", idx);
            return None;
        }
        self.boxes.get(idx as usize)
    }
    fn set_box(&mut self, idx: u8, b: &gtk::Box) {
        if self.boxes.len() < idx as usize + 1 {
            self.boxes.resize(
                idx as usize + 1,
                gtk::Box::new(gtk::Orientation::Horizontal, 0),
            );
        }
        self.boxes[idx as usize] = b.clone();
    }

    fn get_paned(&self, idx: u8) -> Option<&gtk::Paned> {
        if self.paneds.len() < idx as usize + 1 {
            error!("paned not there yet: {}", idx);
            return None;
        }
        self.paneds.get(idx as usize)
    }

    fn set_paned(&mut self, idx: u8, p: &gtk::Paned) {
        if self.paneds.len() < idx as usize + 1 {
            self.paneds.resize(idx as usize + 1, gtk::Paned::default());
        }
        self.paneds[idx as usize] = p.clone();
    }

    fn get_scrolledwindow(&self, idx: u8) -> Option<&ScrolledWindow> {
        if self.scrolledwindows.len() < idx as usize + 1 {
            error!("scrolledwindow not there yet: {}", idx);
            return None;
        }
        self.scrolledwindows.get(idx as usize)
    }

    fn set_scrolledwindow(&mut self, idx: u8, p: &ScrolledWindow) {
        if self.scrolledwindows.len() < idx as usize + 1 {
            self.scrolledwindows
                .resize(idx as usize + 1, ScrolledWindow::default());
        }
        self.scrolledwindows[idx as usize] = p.clone();
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
impl SenderWrapper for SenderWrapperImpl {
    fn send(&self, ev: GuiEvents) {
        let _r = self.0.send(ev);
    }
}

pub struct UIUpdaterAdapterImpl {
    g_command_sender: Sender<IntCommands>,
}

impl UIUpdaterAdapterImpl {
    pub fn new(co_se: Sender<IntCommands>) -> UIUpdaterAdapterImpl {
        UIUpdaterAdapterImpl {
            g_command_sender: co_se,
        }
    }

    pub fn send_to_int(&self, ic: &IntCommands) {
        let r = self
            .g_command_sender
            .send_timeout(ic.clone(), INTERNAL_QUEUE_SEND_DURATION);
        if let Err(ref e) = r {
            warn!(
                "send_int: queue full, skipped {:?}  {:?}  cap={:?}",
                &ic,
                &e,
                self.g_command_sender.capacity()
            );
        }
    }
}

impl UIUpdaterAdapter for UIUpdaterAdapterImpl {
    fn update_text_entry(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateTextEntry(nr));
    }

    fn update_tree(&self, tree_index: u8) {
        self.send_to_int(&IntCommands::UpdateTreeModel(tree_index));
    }
    fn update_tree_single(&self, tree_index: u8, path: &[u16]) {
        self.send_to_int(&IntCommands::UpdateTreeModelSingle(
            tree_index,
            path.to_vec(),
        ));
    }

    fn update_list(&self, list_index: u8) {
        self.send_to_int(&IntCommands::UpdateListModel(list_index));
    }
    fn update_list_single(&self, list_index: u8, list_position: u32) {
        self.send_to_int(&IntCommands::UpdateListModelSingle(
            list_index,
            list_position,
        ));
    }
    fn update_list_some(&self, list_index: u8, list_position: &[u32]) {
        self.send_to_int(&IntCommands::UpdateListModelSome(
            list_index,
            list_position.to_vec(),
        ));
    }
    fn update_text_view(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateTextView(nr));
    }

    fn update_web_view(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateWebView(nr));
    }

    fn update_label(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateLabel(nr));
    }

    fn update_label_markup(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateLabelMarkup(nr));
    }

    fn update_dialog(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateDialog(nr));
    }

    fn show_dialog(&self, nr: u8) {
        self.send_to_int(&IntCommands::ShowDialog(nr));
    }
    fn update_linkbutton(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateLinkButton(nr));
    }

    fn update_paned_pos(&self, nr: u8, pos: i32) {
        self.send_to_int(&IntCommands::UpdatePanedPos(nr, pos));
    }

    fn list_set_cursor(&self, list_index: u8, db_id: isize, column: u8) {
        self.send_to_int(&IntCommands::ListSetCursor(list_index, db_id, column));
    }

    fn widget_mark(&self, typ: UIUpdaterMarkWidgetType, sw_idx: u8, mark: u8) {
        self.send_to_int(&IntCommands::WidgetMark(typ, sw_idx, mark));
    }

    fn grab_focus(&self, typ: UIUpdaterMarkWidgetType, sw_idx: u8) {
        self.send_to_int(&IntCommands::GrabFocus(typ, sw_idx));
    }

    fn update_window_title(&self) {
        self.send_to_int(&IntCommands::UpdateWindowTitle);
    }

    fn update_window_icon(&self) {
        self.send_to_int(&IntCommands::UpdateWindowIcon);
    }
} //