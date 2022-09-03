#[macro_use]
extern crate log;
extern crate gtk;

pub mod dialogdatadistributor;
pub mod gtkmodel_updater;
pub mod gtkrunner;
pub mod iconloader;
pub mod runner_internal;
pub mod ui_value_adapter;

use crate::dialogdatadistributor::DialogDataDistributor;
use flume::Sender;
use gtk::Application;
use gtk::Window;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::UIUpdaterMarkWidgetType;
use std::sync::Arc;
use std::sync::RwLock;
use webkit2gtk::WebContext;
use webkit2gtk::WebView;

pub type GtkObjectsType = Arc<RwLock<dyn GtkObjects>>;
pub type GtkBuilderType = Arc<Box<dyn GtkGuiBuilder + Send + Sync + 'static>>;

pub trait GtkGuiBuilder: 'static {
    fn build_gtk(
        &self,
        gui_event_sender: Sender<GuiEvents>,
        obj_a: GtkObjectsType,
        ddd: &mut DialogDataDistributor,
    );

    // fn create_browser(&self, obj_a: GtkObjectsType);
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum IntCommands {
    NONE,
    START,
    STOP,
    UpdateTextEntry(u8),
    UpdateTreeModel(u8),
    ///  tree_index,   path
    UpdateTreeModelSingle(u8, Vec<u16>),
    UpdateListModel(u8),
    ///  list_index,   list_position
    UpdateListModelSingle(u8, u32),
    UpdateListModelSome(u8, Vec<u32>),
    UpdateTextView(u8),
    UpdateWebView(u8),
    UpdateLabel(u8),
    UpdateLabelMarkup(u8),
    UpdateDialog(u8),
    UpdateLinkButton(u8),
    ShowDialog(u8),
    ///  paned-index , position
    UpdatePanedPos(u8, i32),
    ///  list_index,   db-id, column
    ListSetCursor(u8, isize, u8),
    ///  type, effect marker number, used for renaming its ID, to match CSS definitions
    WidgetMark(UIUpdaterMarkWidgetType, u8, u8),
    ///  Type, index,
    GrabFocus(UIUpdaterMarkWidgetType, u8),
    UpdateWindowTitle,
    UpdateWindowIcon,
}

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

    fn get_text_view(&self, list_index: usize) -> Option<&gtk::TextView>;
    fn add_text_view(&mut self, tv: &gtk::TextView);

    fn get_web_view(&self, idx: u8) -> Option<&WebView>;
    fn add_web_view(&mut self, wv: &WebView);
    fn get_web_context(&self, idx: u8) -> Option<&WebContext>;
    fn add_web_context(&mut self, wc: &WebContext);

    fn get_text_entry(&self, idx: u8) -> Option<&gtk::Entry>;
    fn add_text_entry(&mut self, e: &gtk::Entry);
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