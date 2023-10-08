use crate::runner_internal::GtkRunnerInternal;
use crate::CreateWebViewFnType;
use crate::DialogDataDistributor;
use crate::GtkBuilderType;
use crate::GtkObjects;
use crate::IntCommands;
use crate::WebContentType;
use flume::Receiver;
use flume::Sender;
use gtk::prelude::BoxExt;
use gtk::prelude::ContainerExt;
use gtk::prelude::WidgetExt;
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
use gtk::SearchEntry;
use gtk::TextView;
use gtk::ToolButton;
use gtk::TreeStore;
use gtk::TreeView;
use gtk::TreeViewColumn;
use gtk::Widget;
use gtk::Window;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::GuiRunner;
use gui_layer::abstract_ui::ReceiverWrapper;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UISenderWrapper;
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
const INTERNAL_QUEUE_SIZE: usize = 2000;
const INTERNAL_QUEUE_SEND_DURATION: Duration = Duration::from_millis(200);
const NUM_WEBVIEWS: usize = 2;
const NUM_TREES: usize = 2;

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
        let co_se2 = self.gui_command_sender.clone();
        let builder_c = self.gtk_builder.clone();
        // let ev_se_wc = self.get_event_sender();
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
                let initsuccess =
                    runner_i.init(&builder_c, win_title, win_width, win_height, app_url);
                if !initsuccess {
                    let _r = co_se2.send(IntCommands::STOP);
                    let _r = ev_se2.send(GuiEvents::AppWasAlreadyRunning);
                    return;
                }
                ev_se2.send(GuiEvents::InternalStarted).unwrap(); // co_se2.clone(),
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
                GuiEvents::AppWasAlreadyRunning => {
                    let _r = self.gui_command_sender.send(IntCommands::STOP);
                }
                _ => error!("gtkrunner:init, got other event {:?}", &ev),
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

    fn get_event_sender(&self) -> Arc<dyn UISenderWrapper + Send + Sync + 'static> {
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
    pub text_entries: Vec<Entry>,
    pub c_r_spinner_w: Option<(CellRendererSpinner, TreeViewColumn)>,
    pub labels: Vec<Label>,
    pub dialogs: Vec<Dialog>,
    pub linkbuttons: Vec<LinkButton>,
    pub boxes: Vec<gtk::Box>,
    pub paneds: Vec<Paned>,
    pub scrolledwindows: Vec<ScrolledWindow>,
    dialogdata_dist: Option<DialogDataDistributor>,
    pub web_context: RefCell<Option<WebContext>>, // allow only one browser in the application
    pub web_views: RefCell<[Option<WebView>; NUM_WEBVIEWS]>,
    create_webcontext_fn: WebContentType,
    create_webview_fn: CreateWebViewFnType,
    browser_config: CreateBrowserConfig,
    pub searchentries: Vec<SearchEntry>,
    gui_event_sender: Option<Sender<GuiEvents>>,
    pub toolbuttons: Vec<ToolButton>,
    pub tree_update_block: [bool; NUM_TREES],
}

impl GtkObjectsImpl {
    fn check_or_create_browser(&self) {
        if self.web_context.borrow().is_none() {
            if self.create_webcontext_fn.is_none() {
                warn!("cannot create webContext, no create function here!");
                return;
            }
            if let Some(create_fn) = &self.create_webcontext_fn {
                let w_context = (create_fn)(self.browser_config.clone());
                self.web_context.borrow_mut().replace(w_context);
            }
        }
        if self.web_views.borrow()[0].is_some() {
            return;
        }
        if self.create_webview_fn.is_none() {
            warn!("cannot create WebView, no create function here!");
            return;
        }
        let o_dest_box = self.get_box(self.browser_config.attach_box_index);
        if o_dest_box.is_none() {
            warn!("should not create browser, no gtk-box to attach to !");
            return;
        }
        let dest_box = o_dest_box.unwrap();
        if let Some(ev_se) = &self.gui_event_sender {
            if let Some(create_fn) = &self.create_webview_fn {
                let (w_view1, w_view2) = (create_fn)(
                    self.web_context.borrow().as_ref().unwrap(),
                    self.browser_config.font_size_manual,
                    ev_se.clone(),
                );
                dest_box.pack_start(&w_view1, true, true, 10);
                w_view1.show();
                self.web_views.borrow_mut()[0].replace(w_view1);
                self.web_views.borrow_mut()[1].replace(w_view2);
            }
        } else {
            error!("gtkrunner:  event sender not here !! ");
        }
    }

    #[cfg(not(feature = "legacy3gtk14"))]
    fn paned_default() -> Paned {
        gtk::Paned::default()
    }

    #[cfg(feature = "legacy3gtk14")]
    fn paned_default() -> Paned {
        gtk::Paned::new(gtk::Orientation::Horizontal)
    }

    #[cfg(not(feature = "legacy3gtk14"))]
    fn scrolledwindow_default() -> ScrolledWindow {
        gtk::ScrolledWindow::default()
    }

    #[cfg(feature = "legacy3gtk14")]
    fn scrolledwindow_default() -> ScrolledWindow {
        const NONE_ADJ: Option<&gtk::Adjustment> = None;
        gtk::ScrolledWindow::new(NONE_ADJ, NONE_ADJ)
    }

    // impl GtkObjectsImpl
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

    fn set_tree_store(&mut self, idx: u8, ts: &gtk::TreeStore) {
        if self.tree_stores.len() < idx as usize + 1 {
            self.tree_stores
                .resize(idx as usize + 1, TreeStore::new(&[gtk::glib::Type::BOOL]));
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
                .resize(idx as usize + 1, ListStore::new(&[gtk::glib::Type::BOOL]));
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

    fn get_text_view(&self, index: u8) -> Option<&gtk::TextView> {
        self.text_views.get(index as usize)
    }

    fn set_text_view(&mut self, list_index: u8, tv: &gtk::TextView) {
        if self.text_views.len() < list_index as usize + 1 {
            self.text_views
                .resize(list_index as usize + 1, TextView::new());
        }
        self.text_views[list_index as usize] = tv.clone();
    }

    fn get_web_view(&self, idx: u8) -> Option<WebView> {
        self.check_or_create_browser();
        self.web_views.borrow()[idx as usize].clone()
    }

    // Later: determine which one goes to the gtk-box
    fn set_web_view(&mut self, idx: u8, o_wv: Option<WebView>, font_size_man: Option<u8>) {
        self.browser_config.font_size_manual = font_size_man;
        // trace!(            " set_web_view {}  webView:{:?}  fontsize:{:?} ",            idx,            o_wv,            font_size_man        );
        match o_wv {
            None => {
                let o_dest_box = self
                    .boxes
                    .get(self.browser_config.attach_box_index as usize);
                if o_dest_box.is_none() {
                    error!("set_web_view:None - Box index not found !");
                    return;
                }
                let dest_box = o_dest_box.unwrap();
                if self.web_views.borrow()[idx as usize].is_some() {
                    dest_box.remove(self.web_views.borrow()[idx as usize].as_ref().unwrap());
                }
                self.web_views.borrow_mut()[idx as usize] = None;
            }
            Some(wv) => {
                trace!("runner:setting webView");
                let _r = self.web_views.borrow_mut()[idx as usize].replace(wv);
            }
        };
    }

    fn get_web_context(&self) -> Option<WebContext> {
        self.check_or_create_browser();
        self.web_context.borrow().clone()
    }

    fn set_web_context(&mut self, o_wc: Option<WebContext>) {
        match o_wc {
            None => self.web_context = RefCell::new(None),
            Some(wc) => {
                let _r = self.web_context.borrow_mut().replace(wc);
            }
        };
    }

    fn get_text_entry(&self, index: u8) -> Option<&Entry> {
        self.text_entries.get(index as usize)
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
    fn set_button(&mut self, idx: u8, e: &gtk::Button) {
        if self.buttons.len() < idx as usize + 1 {
            self.buttons.resize(idx as usize + 1, Button::new());
        }
        self.buttons[idx as usize] = e.clone();
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
            self.paneds.resize(idx as usize + 1, Self::paned_default());
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
                .resize(idx as usize + 1, Self::scrolledwindow_default());
        }
        self.scrolledwindows[idx as usize] = p.clone();
    }

    fn set_create_webcontext_fn(
        &mut self,
        cb_fn: Option<Box<dyn Fn(CreateBrowserConfig) -> WebContext>>,
        browser_folder: &str,
        a_box_index: u8,
        browser_clear_cache: bool,
        font_size_man: Option<u8>,
    ) {
        self.create_webcontext_fn = cb_fn;
        self.browser_config = CreateBrowserConfig {
            browser_dir: browser_folder.to_string(),
            attach_box_index: a_box_index,
            startup_clear_cache: browser_clear_cache,
            font_size_manual: font_size_man,
        };
    }

    fn set_create_webview_fn(
        &mut self,
        cb_fn: Option<
            Box<dyn Fn(&WebContext, Option<u8>, Sender<GuiEvents>) -> (WebView, WebView)>,
        >,
    ) {
        self.create_webview_fn = cb_fn;
    }

    fn get_searchentry(&self, idx: u8) -> Option<&SearchEntry> {
        if self.searchentries.len() < idx as usize + 1 {
            error!("scrolledwindow not there yet: {}", idx);
            return None;
        }
        self.searchentries.get(idx as usize)
    }

    fn set_searchentry(&mut self, idx: u8, p: &SearchEntry) {
        if self.searchentries.len() < idx as usize + 1 {
            self.searchentries
                .resize(idx as usize + 1, SearchEntry::default());
        }
        self.searchentries[idx as usize] = p.clone();
    }

    fn set_gui_event_sender(&mut self, ev_se: Sender<GuiEvents>) {
        self.gui_event_sender = Some(ev_se);
    }

    fn get_gui_event_sender(&mut self) -> Option<Sender<GuiEvents>> {
        self.gui_event_sender.clone()
    }

    fn get_toolbutton(&self, idx: u8) -> Option<&ToolButton> {
        if self.toolbuttons.len() < idx as usize + 1 {
            error!("ToolButton not there yet: {}", idx);
            return None;
        }
        self.toolbuttons.get(idx as usize)
    }

    // fn add_linkbutton(&mut self, e: &LinkButton) {
    //     self.linkbuttons.push(e.clone());
    // }
    fn set_toolbutton(&mut self, idx: u8, l: &ToolButton) {
        if self.toolbuttons.len() < idx as usize + 1 {
            let nowidget: Option<&Widget> = None;
            self.toolbuttons
                .resize(idx as usize + 1, ToolButton::new(nowidget, None));
        }
        self.toolbuttons[idx as usize] = l.clone();
    }

    fn set_block_tree_updates(&mut self, idx: u8, block: bool) {
        self.tree_update_block[idx as usize] = block;
    }

    fn get_block_tree_updates(&self, idx: u8) -> bool {
        self.tree_update_block[idx as usize]
    }
}

#[derive(Default, Clone, Debug)]
pub struct CreateBrowserConfig {
    pub attach_box_index: u8,
    pub browser_dir: String,
    pub startup_clear_cache: bool,
    pub font_size_manual: Option<u8>,
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

    fn update_tree_partial(&self, tree_index: u8, path: &[u16]) {
        self.send_to_int(&IntCommands::UpdateTreeModelPartial(
            tree_index,
            path.to_vec(),
        ));
    }

    fn update_list(&self, list_idx: u8) {
        self.send_to_int(&IntCommands::UpdateListModel(list_idx));
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
    fn update_web_view_plain(&self, nr: u8) {
        self.send_to_int(&IntCommands::UpdateWebViewPlain(nr));
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

    fn list_set_cursor(&self, list_index: u8, db_id: isize, column: u8, scroll_pos: i8) {
        self.send_to_int(&IntCommands::ListSetCursor(
            list_index, db_id, column, scroll_pos,
        ));
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

    fn clipboard_set_text(&self, s: String) {
        self.send_to_int(&IntCommands::ClipBoardSetText(s));
    }

    fn web_view_remove(&self, idx: u8, fs_man: Option<u8>) {
        self.send_to_int(&IntCommands::WebViewRemove(idx, fs_man));
    }

    fn memory_conserve(&self, act: bool) {
        self.send_to_int(&IntCommands::MemoryConserve(act));
    }

    fn update_window_minimized(&self, mini: bool, ev_time: u32) {
        self.send_to_int(&IntCommands::UpdateWindowMinimized(mini, ev_time));
    }

    fn tree_set_cursor(&self, tree_idx: u8, path: Vec<u16>) {
        self.send_to_int(&IntCommands::TreeSetCursor(tree_idx, path));
    }

    fn store_image(&self, idx: i32, img: String) {
        self.send_to_int(&IntCommands::StoreImage(idx, img));
    }

    fn toolbutton_set_sensitive(&self, idx: u8, sens: bool) {
        self.send_to_int(&IntCommands::ButtonSetSensitive(idx, sens));
    }
} //
