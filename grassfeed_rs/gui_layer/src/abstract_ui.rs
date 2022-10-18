//  Non-gtk specific gui  abstraction

use crate::gui_values::PropDef;
use core::slice::Iter;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;

//  May not be "Send", for the gtk classes
pub trait GuiRunner {
    fn init(&mut self);
    fn start(&self);
    fn stop(&mut self);
    fn get_ui_updater(&self) -> Rc<RefCell<dyn UIUpdaterAdapter>>;

    fn get_event_receiver(&self) -> Rc<dyn ReceiverWrapper>;

    fn get_event_sender(&self) -> UiSenderWrapperType;
}

pub type UiSenderWrapperType = Arc<dyn UISenderWrapper + Send + Sync + 'static>;

// may not be  "Send"
pub trait GuiObjects {}

pub type UIAdapterValueStoreType = Arc<RwLock<dyn UIAdapterValueStore + Send + Sync + 'static>>;

pub trait ReceiverWrapper {
    /// Returns soon
    fn get_event_try(&self) -> GuiEvents;
    /// Blocks until an event comes
    fn get_event(&self) -> GuiEvents;
    /// waits only specified time
    fn get_event_timeout(&self, timeout_ms: u64) -> GuiEvents;
    /// returns the elenents in queue
    fn get_len(&self) -> usize;
}

pub trait UISenderWrapper {
    fn send(&self, ev: GuiEvents);
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Eq, Hash)]
pub enum GuiEvents {
    None,
    InternalStarted,
    AppWasAlreadyRunning,
    WinDelete,
    WindowSizeChanged(i32, i32),
    WindowThemeChanged(String),
    WindowIconified(bool),
    MenuFileQuit,
    MenuActivate(String),
    ButtonClicked(String),
    ///  paned-id,  position
    PanedMoved(i32, i32),
    ///  tree index,  tree position, db-id
    TreeRowActivated(i32, Vec<u16>, i32),
    ///  list index, list-position, db-id
    ListRowActivated(i32, i32, i32),
    ///  list index, list-position,  sort-column-nr, db-id
    ListCellClicked(i32, i32, i32, i32),
    ///  list index, list-position, db-id
    ListRowDoubleClicked(i32, i32, i32),
    /// identifier, payload
    DialogData(String, Vec<AValue>),
    /// single textfield editing:  identifier, payload
    DialogEditData(String, AValue),
    /// Tree-Index,  tree-repo-id, command
    TreeEvent(u8, i32, String),
    /// Tree-Nr, From-Path, To-Path
    TreeDragEvent(u8, Vec<u16>, Vec<u16>),
    /// Tree-Nr,  db-id,
    TreeCollapsed(u8, i32),
    /// Tree-Nr,  db-id,
    TreeExpanded(u8, i32),
    /// button-name
    ToolBarButton(String),
    /// button-name, is-active
    ToolBarToggle(String, bool),
    ///  column-id,  width
    ColumnWidth(i32, i32),
    /// list-id, List of Content-repo-id
    ListSelected(u8, Vec<i32>),
    /// list-id, action-name , List of ( Content-repo-id,   Gui-List-Positions)
    ListSelectedAction(u8, String, Vec<(i32, i32)>),
    /// list-id, column-id, ascending
    ListSortOrderChanged(u8, u8, bool),
    // Key-Code via gdk,   Unicode-Char
    KeyPressed(isize, Option<char>),
    /// index, new-text
    SearchEntryTextChanged(u8, String),
    Indicator(String),
}

impl Default for GuiEvents {
    fn default() -> GuiEvents {
        GuiEvents::None
    }
}

pub trait UIAdapterValueStore {
    fn memory_conserve(&mut self, active: bool);

    fn set_text_entry(&mut self, index: u8, newtext: String);
    fn get_text_entry(&self, index: u8) -> Option<String>;

    ///  insert a new  Tree-Item the given position by path
    fn insert_tree_item(&mut self, path: &[u16], treevalues: &[AValue]);
    fn get_tree_item(&self, path: &[u16]) -> Vec<AValue>;
    fn get_tree_root(&self) -> GuiTreeItem;
    ///  replaces a item
    fn replace_tree_item(&mut self, path: &[u16], treevalues: &[AValue]);
    fn clear_tree(&mut self, tree_index: u8);

    ///  insert a new list item
    fn insert_list_item(&mut self, list_index: u8, list_position: i32, values: &[AValue]);
    fn clear_list(&mut self, list_index: u8);
    fn get_list_item(&self, list_index: u8, list_position: i32) -> Option<Vec<AValue>>;
    fn get_list_iter(&self, _list_index: u8) -> Iter<Vec<AValue>>;

    fn set_text_view(&mut self, index: u8, newtext: String);
    fn get_text_view(&self, index: u8) -> Option<String>;

    fn set_web_view_text(&mut self, index: u8, newtext: String);
    fn get_web_view_text(&self, index: u8) -> Option<String>;

    fn set_spinner_active(&mut self, a: bool);
    fn is_spinner_active(&self) -> bool;

    fn set_tree_row_expand(&mut self, idx: usize, column: usize, bitmask: u32);
    fn get_tree_row_expand(&self, idx: usize) -> (usize, u32);
    fn is_tree_row_expanded(&self, idx: usize, gti: &GuiTreeItem) -> bool;

    fn set_label_text(&mut self, index: u8, newtext: String);
    fn get_label_text(&self, index: u8) -> Option<&String>;

    fn set_dialog_data(&mut self, idx: u8, values: &[AValue]);
    fn get_dialog_data(&self, idx: u8) -> Option<&Vec<AValue>>;

    fn set_gui_property(&mut self, name: PropDef, value: String);
    fn get_gui_property_or(&self, name: PropDef, default: String) -> String;
    fn get_gui_int_or(&self, name: PropDef, default: isize) -> isize;
    fn set_gui_properties(&mut self) -> HashMap<PropDef, String>;
    fn get_gui_bool(&self, name: PropDef) -> bool;

    ///  label,  url
    fn set_linkbutton_text(&mut self, index: u8, text_uri: (String, String));
    ///  label,  url
    fn get_linkbutton_text(&self, index: u8) -> Option<&(String, String)>;

    fn set_window_title(&mut self, t: String);
    fn get_window_title(&self) -> String;

    fn set_window_icon(&mut self, icon_compressed: String);

    /// gets and removes it. can be called only once
    fn get_window_icon(&mut self) -> String;

    fn set_label_tooltip(&mut self, index: u8, newtext: String);
    fn get_label_tooltip(&self, index: u8) -> Option<&String>;
}

#[derive(Default, Clone)]
pub struct GuiTreeItem {
    pub a_values: Vec<AValue>,
    pub children: Vec<GuiTreeItem>,
}

impl GuiTreeItem {
    pub fn new_named_(display_: &str) -> Self {
        let mut r = GuiTreeItem::default();
        r.a_values.push(AValue::ASTR(display_.to_string()));
        r
    }

    pub fn new_values(v: &[AValue]) -> Self {
        GuiTreeItem {
            a_values: v.to_owned(),
            ..Default::default()
        }
    }

    pub fn add(&mut self, gti: GuiTreeItem) {
        self.children.push(gti);
    }
}

impl std::fmt::Debug for GuiTreeItem {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("GTI")
            .field("V", &self.a_values)
            .field("#C", &self.children.len())
            .finish()
    }
}

pub trait UIUpdaterAdapter {
    fn update_tree(&self, tree_index: u8);
    fn update_tree_single(&self, tree_index: u8, path: &[u16]);

    fn update_list(&self, list_index: u8);
    fn update_list_single(&self, list_index: u8, list_position: u32);
    fn update_list_some(&self, list_index: u8, list_position: &[u32]);

    fn update_text_view(&self, nr: u8);
    fn update_text_entry(&self, nr: u8);
    fn update_label(&self, nr: u8);
    fn update_label_markup(&self, nr: u8);
    fn update_linkbutton(&self, nr: u8);
    fn update_dialog(&self, nr: u8);
    fn show_dialog(&self, nr: u8);
    fn update_paned_pos(&self, nr: u8, pos: i32);
    fn widget_mark(&self, typ: UIUpdaterMarkWidgetType, sw_idx: u8, mark: u8);
    fn grab_focus(&self, typ: UIUpdaterMarkWidgetType, sw_idx: u8);
    //  list-idx,     db-id: -1 for no cursor,        select column for db-id
    fn list_set_cursor(&self, list_index: u8, db_id: isize, column: u8);
    fn update_window_title(&self);
    fn update_window_icon(&self);
    fn update_web_view(&self, nr: u8);
    fn update_web_view_plain(&self, nr: u8);
    fn web_view_remove(&self, fontsizemanual: Option<u8>);

    fn clipboard_set_text(&self, s: String);
    fn memory_conserve(&self, act: bool);

    fn update_systray_indicator(&self, enable: bool);
}

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd, Hash, Clone)]
pub enum UIUpdaterMarkWidgetType {
    ScrolledWindow,
    Box,
    TreeView,
    WebView,
}

pub enum WebViewUpdateResult {
    Ok,
    NeedRestartView,
}

//  Values Wrapper as intermediate for  glib::values
#[derive(Serialize, Deserialize, PartialEq, Clone, Eq, Hash)]
pub enum AValue {
    None,
    AU32(u32),
    AI32(i32),
    ASTR(String),
    AIMG(String),
    ABOOL(bool),
}

impl std::fmt::Debug for AValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut r = f.debug_struct("").finish();
        match &self {
            AValue::AU32(n) => r = f.debug_struct("U32:").field("", n).finish(),
            AValue::AI32(n) => r = f.debug_struct("I32:").field("", n).finish(),
            AValue::ASTR(s) => r = f.debug_struct("STR:").field("", s).finish(),
            AValue::AIMG(s) => r = f.debug_struct("IMG:#").field("", &s.len()).finish(),
            AValue::ABOOL(b) => r = f.debug_struct("B:#").field("", b).finish(),
            _ => {}
        };
        r
    }
}

impl AValue {
    pub fn str(&self) -> Option<String> {
        match &self {
            AValue::AI32(i) => Some(i.to_string()),
            AValue::AU32(i) => Some(i.to_string()),
            AValue::ASTR(s) => Some(s.clone()),
            AValue::AIMG(s) => Some(s.clone()),
            AValue::ABOOL(b) => Some((*b).to_string()),
            _ => None,
        }
    }
    pub fn int(&self) -> Option<i32> {
        match &self {
            AValue::AI32(i) => Some(*i),
            _ => None,
        }
    }
    pub fn uint(&self) -> Option<u32> {
        match &self {
            AValue::AU32(u) => Some(*u),
            _ => None,
        }
    }
    pub fn boo(&self) -> bool {
        match &self {
            AValue::ABOOL(b) => *b,
            _ => false,
        }
    }
}

pub struct TreeRowExpand {}
impl TreeRowExpand {
    pub fn is_expanded(gti: &GuiTreeItem, col_bitmask: (usize, u32)) -> bool {
        if let AValue::AU32(v) = gti.a_values[col_bitmask.0] {
            if v & col_bitmask.1 == col_bitmask.1 {
                return true;
            }
        }
        false
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum KeyCodes {
    Nothing = 0,
    Tab,
    ShiftTab,
    Space,
    Escape,
    Enter,
    Delete,
    CursorUp,
    CursorDown,
    CursorRight,
    CursorLeft,
    F1,
    F2,
    F3,
    F4,
    Key_A,
    Key_a,
    Key_B,
    Key_b,
    Key_N,
    Key_n,
    Key_s,
    Key_v,
    Key_x,
}
