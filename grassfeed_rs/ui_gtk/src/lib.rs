extern crate flume;
extern crate gdk_sys;
extern crate gtk;
extern crate webkit2gtk;
#[macro_use]
extern crate log;

pub mod dialogdatadistributor;
pub mod gtkmodel_updater;
pub mod gtkrunner;
pub mod iconloader;
pub mod keyboard_codes;
pub mod runner_internal;
pub mod ui_value_adapter;

use crate::dialogdatadistributor::DialogDataDistributor;
use crate::gtkrunner::CreateBrowserConfig;
use flume::Sender;
use gtk::Application;
use gtk::Window;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::UIUpdaterMarkWidgetType;
use std::sync::Arc;
use std::sync::RwLock;
use webkit2gtk::WebContext;
use webkit2gtk::WebView;

// warning: usage of an `Arc` that is not `Send` or `Sync`
//   the trait `Send` is not implemented for `RwLock<GtkObjectsImpl>`
pub type GtkObjectsType = Arc<RwLock<dyn GtkObjects>>;
pub type GtkBuilderType = Arc<Box<dyn GtkGuiBuilder + Send + Sync + 'static>>;

pub trait GtkGuiBuilder: 'static {
    fn build_gtk(
        &self,
        gui_event_sender: Sender<GuiEvents>,
        obj_a: GtkObjectsType,
        ddd: &mut DialogDataDistributor,
    );
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum UpdateListMode {
    None,
    FirstPart,
    MiddlePart,
    LastPart,
}

// order is important, upper items will be prioritized
#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum IntCommands {
    NONE,
    START,
    STOP,
    StoreImage(i32, String),
    UpdateTextEntry(u8),
    UpdateTreeModel(u8),
    ///  tree_index,   path
    UpdateTreeModelSingle(u8, Vec<u16>),

    ///  tree_index,   path
    UpdateTreeModelPartial(u8, Vec<u16>),

    UpdateListModel(u8),
    /// list_idx,  Detach/Attach View,  List-Start, Count.
    /// On too large lists, we use this paginated commands to split the update into several steps
    UpdateListModelPaginate(u8, UpdateListMode, usize, usize),

    ///  list_index,   list_position
    UpdateListModelSingle(u8, u32),
    ///  list_index,   list_positions
    UpdateListModelSome(u8, Vec<u32>),

    ///  list_index,   db-id, column,   scroll-pos
    ListSetCursor(u8, isize, u8, i8),
    ///  tree_index,   path
    TreeSetCursor(u8, Vec<u16>),

    UpdateTextView(u8),

    // webView_index
    UpdateWebView(u8),
    UpdateWebViewPlain(u8),

    UpdateLabel(u8),
    UpdateLabelMarkup(u8),
    UpdateDialog(u8),
    UpdateLinkButton(u8),
    ShowDialog(u8),
    ///  paned-index , position
    UpdatePanedPos(u8, i32),
    ///  type, effect marker number, used for renaming its ID, to match CSS definitions
    WidgetMark(UIUpdaterMarkWidgetType, u8, u8),
    ///  Type, index,
    GrabFocus(UIUpdaterMarkWidgetType, u8),
    UpdateWindowTitle,
    UpdateWindowIcon,
    ClipBoardSetText(String),
    // font size manual
    WebViewRemove(u8, Option<u8>),
    MemoryConserve(bool),
    TrayIconEnable(bool),
    UpdateWindowMinimized(bool, u32),
    ButtonSetSensitive(u8, bool),
}

pub type WebContentType = Option<Box<dyn Fn(CreateBrowserConfig) -> WebContext>>;

///  WebContext,   FontSizeManual, GuiEventSender
pub type CreateWebViewFnType =
    Option<Box<dyn Fn(&WebContext, Option<u8>, Sender<GuiEvents>) -> (WebView, WebView)>>;

pub trait GtkObjects {
    fn get_window(&self) -> Option<Window>;
    fn set_window(&mut self, w: &Window);
    fn get_application(&self) -> Option<Application>;
    fn set_application(&mut self, a: &Application);

    fn get_tree_store(&self, tree_index: usize) -> Option<&gtk::TreeStore>;
    fn set_tree_store(&mut self, idx: u8, ts: &gtk::TreeStore);

    fn get_tree_view(&self, tree_index: usize) -> Option<&gtk::TreeView>;
    fn set_tree_view(&mut self, idx: u8, tv: &gtk::TreeView);

    fn get_tree_store_max_columns(&self, tree_index: usize) -> u8;
    fn set_tree_store_max_columns(&mut self, tree_index: usize, max_col: u8);

    fn get_list_store(&self, list_index: usize) -> Option<&gtk::ListStore>;
    fn set_list_store(&mut self, idx: u8, store: &gtk::ListStore);

    fn get_list_store_max_columns(&self, list_index: usize) -> u8;
    fn set_list_store_max_columns(&mut self, list_index: usize, mc: u8);

    fn get_text_view(&self, list_index: u8) -> Option<&gtk::TextView>;
    fn set_text_view(&mut self, list_index: u8, tv: &gtk::TextView);

    fn get_web_view(&self, idx: u8) -> Option<WebView>;
    fn set_web_view(&mut self, idx: u8, wv: Option<WebView>, font_size_man: Option<u8>);

    fn get_web_context(&self) -> Option<WebContext>;
    fn set_web_context(&mut self, wc: Option<WebContext>);

    fn get_text_entry(&self, idx: u8) -> Option<&gtk::Entry>;
    fn set_text_entry(&mut self, idx: u8, e: &gtk::Entry);

    fn get_buttons(&self) -> Vec<gtk::Button>;
    fn add_button(&mut self, e: &gtk::Button);

    fn get_spinner_w(&self) -> Option<(gtk::CellRendererSpinner, gtk::TreeViewColumn)>;
    fn set_spinner_w(&mut self, widgets: (gtk::CellRendererSpinner, gtk::TreeViewColumn));

    fn get_label(&self, idx: u8) -> Option<&gtk::Label>;
    fn add_label(&mut self, e: &gtk::Label);
    fn set_label(&mut self, idx: u8, e: &gtk::Label);

    fn get_dialog(&self, idx: u8) -> Option<&gtk::Dialog>;
    fn set_dialog(&mut self, idx: u8, d: &gtk::Dialog);

    fn set_dddist(&mut self, ddd: DialogDataDistributor);
    fn get_dddist(&self) -> &Option<DialogDataDistributor>;

    fn get_linkbutton(&self, idx: u8) -> Option<&gtk::LinkButton>;
    fn add_linkbutton(&mut self, e: &gtk::LinkButton);
    fn set_linkbutton(&mut self, idx: u8, e: &gtk::LinkButton);

    fn get_box(&self, idx: u8) -> Option<&gtk::Box>;
    fn set_box(&mut self, idx: u8, e: &gtk::Box);

    fn get_paned(&self, idx: u8) -> Option<&gtk::Paned>;
    fn set_paned(&mut self, idx: u8, e: &gtk::Paned);

    fn get_scrolledwindow(&self, idx: u8) -> Option<&gtk::ScrolledWindow>;
    fn set_scrolledwindow(&mut self, idx: u8, p: &gtk::ScrolledWindow);

    fn set_create_webcontext_fn(
        &mut self,
        cb_fn: Option<Box<dyn Fn(CreateBrowserConfig) -> WebContext>>,
        browser_dir: &str,
        a_box_index: u8,
        browser_clear_cache: bool,
        font_size_man: Option<u8>,
    );

    fn set_create_webview_fn(&mut self, cb_fn: CreateWebViewFnType);

    fn set_searchentry(&mut self, idx: u8, e: &gtk::SearchEntry);
    fn get_searchentry(&self, idx: u8) -> Option<&gtk::SearchEntry>;

    fn set_gui_event_sender(&mut self, ev_se: Sender<GuiEvents>);
    fn get_gui_event_sender(&mut self) -> Option<Sender<GuiEvents>>;

    fn get_toolbutton(&self, idx: u8) -> Option<&gtk::ToolButton>;
    fn set_toolbutton(&mut self, idx: u8, e: &gtk::ToolButton);

    fn set_block_tree_updates(&mut self, idx: u8, block: bool);
    fn get_block_tree_updates(&self, idx: u8) -> bool;
}

#[derive(Clone, Debug)]
pub struct GtkWindowConfig {
    pub title: String,
    pub default_width: i32,
    pub default_height: i32,
    pub show_menubar: bool,
}

impl Default for GtkWindowConfig {
    fn default() -> Self {
        GtkWindowConfig {
            title: String::from("default title"),
            default_width: 50,
            default_height: 50,
            show_menubar: false,
        }
    }
}
