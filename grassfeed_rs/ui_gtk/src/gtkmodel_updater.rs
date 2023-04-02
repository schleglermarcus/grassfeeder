#[cfg(feature = "legacy3gtk14")]
use webkit2gtk::traits::WebViewExt;
#[cfg(not(feature = "legacy3gtk14"))]
use webkit2gtk::WebViewExt;

use crate::iconloader::IconLoader;
use crate::GtkObjectsType;
use gtk::gdk::RGBA;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::glib;
use gtk::prelude::*;
use gtk::Label;
use gtk::ListStore;
use gtk::SortColumn;
use gtk::SortType;
use gtk::TreeIter;
use gtk::TreePath;
use gtk::TreeStore;
use gtk::TreeView;
use gtk::TreeViewColumn;
use gtk::Widget;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiTreeItem;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UISenderWrapper;
use gui_layer::abstract_ui::UIUpdaterMarkWidgetType;
use gui_layer::gui_values::PropDef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::From;
use std::sync::Arc;
use webkit2gtk::WebView;

pub struct GtkModelUpdaterInt {
    m_v_store: UIAdapterValueStoreType,
    g_o_a: GtkObjectsType,
    pixbufcache: RefCell<HashMap<String, Pixbuf>>,
    // ev_sender_w: Arc<dyn UISenderWrapper + Send + Sync + 'static>,
}

impl GtkModelUpdaterInt {
    pub fn new(
        g_m_v_s: UIAdapterValueStoreType,
        gtkobjects_a: GtkObjectsType,
        _ev_se_w: Arc<dyn UISenderWrapper + Send + Sync + 'static>,
    ) -> Self {
        GtkModelUpdaterInt {
            m_v_store: g_m_v_s,
            g_o_a: gtkobjects_a,
            pixbufcache: RefCell::new(HashMap::new()),
            // ev_sender_w: ev_se_w,
        }
    }

    fn is_tree_row_expanded(&self, idx: usize, gti: &GuiTreeItem) -> bool {
        (self.m_v_store)
            .read()
            .unwrap()
            .is_tree_row_expanded(idx, gti)
    }

    pub fn update_text_entry(&self, idx: u8) {
        let o_s = (self.m_v_store).read().unwrap().get_text_entry(idx);
        if let Some(content) = o_s {
            let g_o = (*self.g_o_a).read().unwrap();
            if let Some(entry) = g_o.get_text_entry(idx) {
                entry.set_text(&content);
                return;
            }
        }
        error!("update_text_entry({}) not found", idx);
    }

    ///  disconnects the view, expands the current focus again
    pub fn update_tree_model(&self, index: u8) {
        let g_o = (*self.g_o_a).read().unwrap();
        let tree_store: &TreeStore = g_o.get_tree_store(index as usize).unwrap();
        let tree_view: &TreeView = g_o.get_tree_view(index as usize).unwrap();
        let max_columns = g_o.get_tree_store_max_columns(index as usize) as usize;
        let view_option: Option<&ListStore> = None;
        tree_view.set_model(view_option);
        tree_store.clear();
        let path: Vec<i32> = vec![];
        let mut expand_paths: Vec<TreePath> = Vec::default();
        let gui_tree_root = (self.m_v_store).read().unwrap().get_tree_root();
        for (path_index, gti) in gui_tree_root.children.iter().enumerate() {
            let mut innerpath = path.clone();
            innerpath.push(path_index as i32);
            self.add_to_treestore(
                index as usize,
                tree_store,
                gti,
                None,
                innerpath,
                max_columns,
                &mut expand_paths,
            );
        }
        tree_view.set_model(Some(tree_store));
        expand_paths.iter().for_each(|t_path| {
            tree_view.expand_to_path(t_path);
        });
    }

    ///  Fills the columns, according to the guitreeitem's order
    ///  Is recursive
    /// later:  satisfy clippy  8/7
    #[allow(clippy::too_many_arguments)]
    fn add_to_treestore(
        &self,
        tree_idx: usize,
        tree_store: &TreeStore,
        gti: &GuiTreeItem,
        parent_iter: Option<&TreeIter>,
        path: Vec<i32>,
        max_columns: usize,
        expand_paths: &mut Vec<TreePath>,
    ) {
        let last_iter = tree_store.insert(parent_iter, -1);
        self.treestore_set_row(tree_store, gti, &last_iter, max_columns);
        if let Some(t_path) = tree_store.path(&last_iter) {
            if self.is_tree_row_expanded(tree_idx, gti) {
                expand_paths.push(t_path);
            }
        }
        for (path_index, child_gti) in gti.children.iter().enumerate() {
            let mut innerpath = path.clone();
            innerpath.push(path_index as i32);
            self.add_to_treestore(
                tree_idx,
                tree_store,
                child_gti,
                Some(&last_iter),
                innerpath,
                max_columns,
                expand_paths,
            );
        }
    }

    fn treestore_set_row(
        &self,
        tree_store: &TreeStore,
        gti: &GuiTreeItem,
        t_iter: &TreeIter,
        max_columns: usize,
    ) {
        let debuginfo = gti.a_values[1].str().unwrap();
        for c in 0..max_columns {
            match gti.a_values.get(c).unwrap() {
                AValue::AU32(u) => tree_store.set(t_iter, &[(c as u32, &u)]),
                AValue::AI32(i) => tree_store.set(t_iter, &[(c as u32, &i)]),
                AValue::ASTR(s) => tree_store.set(t_iter, &[(c as u32, &s)]),
                AValue::ABOOL(b) => tree_store.set(t_iter, &[(c as u32, &b)]),
                AValue::AIMG(s) => {
                    let contained = self.pixbufcache.borrow().contains_key(s);
                    if !contained {
                        let pb: Pixbuf = Self::icon_for_string(s, debuginfo.clone());
                        self.pixbufcache.borrow_mut().insert(s.clone(), pb);
                    }
                    match self.pixbufcache.borrow().get(s) {
                        Some(e_pb) => {
                            tree_store.set(t_iter, &[(c as u32, &e_pb)]);
                        }
                        None => {
                            error!("     treestore_set_row: {}  pixbuf was inserted, but is not there ", debuginfo);
                        }
                    }
                }
                AValue::None => (),
            }
        }
    }

    fn icon_for_string(s: &String, debug_info: String) -> gtk::gdk_pixbuf::Pixbuf {
        if s.is_empty() {
            debug!("tree inserting empty icon: no-data  ");
            return crate::iconloader::get_missing_icon();
        }
        let buf = IconLoader::decompress_string_to_vec(s);
        if buf.is_empty() {
            debug!("tree inserting empty icon: buf.len=0 {} ", s);
            return crate::iconloader::get_missing_icon();
        }
        match IconLoader::vec_to_pixbuf(&buf) {
            Ok(pb) => pb,
            Err(e) => {
                warn!(
                    "tree inserting empty icon: cannot convert  {:?} {} {:?} ",
                    e, debug_info, s
                );
                crate::iconloader::get_missing_icon()
            }
        }
    }

    ///  replaces a single line of the tree
    pub fn update_tree_model_single(&self, index: u8, path: Vec<u16>) {
        let max_columns;
        let tree_store: TreeStore;
        {
            let g_o = (*self.g_o_a).read().unwrap();
            max_columns = g_o.get_tree_store_max_columns(index as usize) as usize;
            tree_store = g_o.get_tree_store(index as usize).unwrap().clone();
        }
        let mut gti: &GuiTreeItem = &(self.m_v_store).read().unwrap().get_tree_root();
        for p_index in path.iter() {
            if *p_index as usize >= gti.children.len() {
                error!(
                    "update_tree_model_single: BadPath1 {:?}    Index:{}   #children={}",
                    &path,
                    *p_index,
                    gti.children.len()
                );
                return;
            }
            gti = &gti.children[*p_index as usize];
        }
        let path_cn = format!("{path:?}")
            .replace(['[', ']', ' '], "")
            .replace(',', ":");
        if let Some(iter) = tree_store.iter_from_string(&path_cn) {
            self.treestore_set_row(&tree_store, gti, &iter, max_columns);
        } else {
            error!(
                "update_tree_model_single: BadPath2 {:?} {:?} : TreeStore does not contain iter.  {:?}  ",
                &path, &path_cn, gti
            );
        }
    }

    pub fn tree_set_cursor(&self, idx: u8, path: Vec<u16>) {
        let tree_store: TreeStore;
        let tree_view: TreeView;
        {
            let g_o = (*self.g_o_a).read().unwrap();
            tree_store = g_o.get_tree_store(idx as usize).unwrap().clone();
            let o_treeview = g_o.get_tree_view(idx as usize);
            if o_treeview.is_none() {
                error!("tree_set_cursor: no treeview!");
                return;
            }
            tree_view = o_treeview.unwrap().clone();
        }
        let path_cn = format!("{:?}", path)
            .replace(['[', ']', ' '], "")
            .replace(',', ":");
        if let Some(iter) = tree_store.iter_from_string(&path_cn) {
            if let Some(t_path) = tree_store.path(&iter) {
                let focus_column: Option<&TreeViewColumn> = None;
                tree_view.set_cursor(&t_path, focus_column, false);
            }
        } else {
            error!("tree_set_cursor: BadPath3 {:?} {:?}   ", &path, &path_cn);
        }
    }

    /// deconnects the list store,  refills it, reconnects it,   puts cursor back
    ///  Needs the same index for   ListStore  as for TreeView
    pub fn update_list_model(&self, list_index: u8) {
        let now = std::time::Instant::now();
        let g_o = (*self.g_o_a).read().unwrap();
        let o_list_store = g_o.get_list_store(list_index as usize);
        if o_list_store.is_none() {
            error!("update_list_model: liststore {} not found", list_index);
            return;
        }
        let list_store: &ListStore = o_list_store.unwrap();
        let o_list_view = g_o.get_tree_view(list_index as usize);
        if o_list_view.is_none() {
            error!("update_list_model: tree_view {} not found", list_index);
            return;
        }
        let list_view: &TreeView = o_list_view.unwrap();
        let maxcols: u32 = g_o.get_list_store_max_columns(list_index as usize) as u32;
        let empty_view_option: Option<&ListStore> = None;
        list_view.set_model(empty_view_option);
        let o_last_sort_column_id: Option<(SortColumn, SortType)> = list_store.sort_column_id();
        if o_last_sort_column_id.is_some() {
            list_store.set_unsorted();
        }
        list_store.clear();
        let mut num_lines = 0;
        for row in (self.m_v_store).read().unwrap().get_list_iter(list_index) {
            let append_iter = list_store.insert(-1);
            if row.len() < maxcols as usize {
                error!(
                    " update_list_model row shorter that columns #row:{}  columns:{}  SKIPPING",
                    row.len(),
                    maxcols
                );
                continue;
            }
            Self::put_into_store(list_store, &append_iter, maxcols, row, &self.pixbufcache);
            num_lines += 1;
        }
        if let Some((sort_col, sort_type)) = o_last_sort_column_id {
            list_store.set_sort_column_id(sort_col, sort_type);
        }
        list_view.set_model(Some(list_store));
        let elapsed = now.elapsed().as_millis();
        if elapsed > 200 {
            trace!(
                "update_list_model took {:?}ms #lines:{} ",
                elapsed,
                num_lines
            );
        }
    }

    fn put_into_store(
        list_store: &ListStore,
        iter: &TreeIter,
        maxcols: u32,
        row: &[AValue],
        pixbufcache: &RefCell<HashMap<String, Pixbuf>>,
    ) {
        for column in 0..maxcols {
            let o_column = row.get(column as usize);
            if o_column.is_none() {
                error!(
                    "put_into_store  row has no column {}  #row={}",
                    column,
                    row.len()
                );
                continue;
            }
            match o_column.unwrap() {
                AValue::ASTR(s) => {
                    list_store.set_value(iter, column, &glib::Value::from(&s));
                }
                AValue::AU32(u) => {
                    list_store.set_value(iter, column, &glib::Value::from(&u));
                }
                AValue::AI32(i) => {
                    list_store.set_value(iter, column, &glib::Value::from(&i));
                }
                AValue::ABOOL(b) => {
                    list_store.set_value(iter, column, &glib::Value::from(&b));
                }
                AValue::AIMG(s) => {
                    let contained = pixbufcache.borrow().contains_key(s);
                    if !contained {
                        let pb: Pixbuf = Self::icon_for_string(s, String::default());
                        pixbufcache.borrow_mut().insert(s.clone(), pb);
                    }
                    match pixbufcache.borrow().get(s) {
                        Some(e_pb) => {
                            list_store.set(iter, &[(column, &e_pb)]);
                        }
                        None => {
                            error!("list,put_into_store:   pixbuf was inserted, but is not there ");
                        }
                    }
                }
                AValue::None => (),
            }
        }
    }

    pub fn update_list_model_single(&self, list_index: u8, list_position: u32) {
        let o_row = (self.m_v_store)
            .read()
            .unwrap()
            .get_list_item(list_index, list_position as i32);
        if let Some(row) = o_row {
            let g_o = (*self.g_o_a).read().unwrap();
            let list_store: &ListStore = g_o.get_list_store(list_index as usize).unwrap();
            let maxcols = g_o.get_list_store_max_columns(list_index as usize) as u32;
            let gpath = gtk::TreePath::from_indicesv(&[list_position as i32]);
            let iter = list_store.iter(&gpath).unwrap();
            if row.len() < maxcols as usize {
                error!(
                    "update_list_model_single row shorter that columns #row:{}  columns:{}  SKIPPING",
                    row.len(),
                    maxcols
                );
                return;
            }
            Self::put_into_store(list_store, &iter, maxcols, &row, &self.pixbufcache);
        }
    }

    pub fn update_list_model_some(&self, list_index: u8, list_positions: Vec<u32>) {
        let g_o = (*self.g_o_a).read().unwrap();
        let list_store: &ListStore = g_o.get_list_store(list_index as usize).unwrap();
        let maxcols = g_o.get_list_store_max_columns(list_index as usize) as u32;
        for list_pos in list_positions {
            let o_row = self
                .m_v_store
                .read()
                .unwrap()
                .get_list_item(list_index, list_pos as i32);
            if let Some(row) = o_row {
                let gpath = gtk::TreePath::from_indicesv(&[list_pos as i32]);
                if let Some(iter) = list_store.iter(&gpath) {
                    if row.len() < maxcols as usize {
                        error!(
		                    "update_list_model_some row shorter that columns #row:{}  columns:{}  SKIPPING",
		                    row.len(),
		                    maxcols
		                );
                        continue;
                    }
                    Self::put_into_store(list_store, &iter, maxcols, &row, &self.pixbufcache);
                }
            }
        }
    }

    pub fn update_text_view(&self, text_view_index: u8) {
        let g_o = (*self.g_o_a).read().unwrap();
        if let Some(textview) = g_o.get_text_view(text_view_index) {
            let o_tv = (self.m_v_store)
                .read()
                .unwrap()
                .get_text_view(text_view_index);
            if let Some(newtext) = o_tv {
                if let Some(buffer) = textview.buffer() {
                    buffer.set_text(newtext.as_str());
                }
            }
        } else {
            error!("update_text_view({}) not found", text_view_index);
        }
    }

    // This contains a workaround for:  WebView hangs occasionally on some feed contents.
    // return false if webView hangs
    pub fn update_web_view(&self) -> bool {
        let webviewtext_index = 0;
        let g_o = (*self.g_o_a).read().unwrap();
        if let Some(webview) = g_o.get_web_view() {
            if webview.is_loading() {
                webview.stop_loading();
                std::thread::sleep(std::time::Duration::from_millis(3));
            }
            if webview.is_loading() {
                let isresponsive = webview.is_web_process_responsive();
                if !isresponsive {
                    warn!("WebView is still loading, not responsive !   ");
                    return false;
                }
            }
        } else {
            error!("update_web_view: NO VIEW! ");
            return false;
        }
        if let Some(webview) = g_o.get_web_view() {
            let store = (self.m_v_store).read().unwrap();
            let o_wv_t = store.get_web_view_text(webviewtext_index);
            if let Some(text) = o_wv_t {
                if webview.is_loading() {
                    webview.stop_loading();
                    std::thread::sleep(std::time::Duration::from_millis(3));
                }
                let bright_int = store.get_gui_int_or(PropDef::BrowserBackgroundLevel, 50);
                set_brightness(bright_int, &webview);
                webview.load_html(&text, None);
                let browser_zoom_pc = store.get_gui_int_or(PropDef::BrowserZoomPercent, 100);
                webview.set_zoom_level(browser_zoom_pc as f64 / 100.0);
            }
        }
        true
    }

    pub fn update_web_view_plain(&self) {
        if let Some(webview) = (*self.g_o_a).read().unwrap().get_web_view() {
            let store = (self.m_v_store).read().unwrap();
            let o_wv_t = store.get_web_view_text(0);
            if let Some(text) = o_wv_t {
                let bright_int = store.get_gui_int_or(PropDef::BrowserBackgroundLevel, 50);
                set_brightness(bright_int, &webview);
                webview.load_plain_text(&text);
            }
        }
    }

    pub fn update_label(&self, idx: u8) {
        let g_o = (*self.g_o_a).read().unwrap();
        let label: &Label = g_o.get_label(idx).unwrap();

        let valstore = (self.m_v_store).read().unwrap();
        if let Some(newtext) = valstore.get_label_text(idx) {
            label.set_text(newtext);
        }
    }

    pub fn update_label_markup(&self, idx: u8) {
        let g_o = (*self.g_o_a).read().unwrap();
        let label: &Label = g_o.get_label(idx).unwrap();
        let valstore = (self.m_v_store).read().unwrap();
        if let Some(newtext) = valstore.get_label_text(idx) {
            label.set_markup(newtext);
        }
        if let Some(ttt) = valstore.get_label_tooltip(idx) {
            if ttt.is_empty() {
                label.set_tooltip_text(None);
            } else {
                label.set_tooltip_text(Some(ttt));
            }
        }
    }

    pub fn update_dialog(&self, idx: u8) {
        if let Some(dd) = (self.m_v_store).read().unwrap().get_dialog_data(idx) {
            let gtk_objects = (*self.g_o_a).read().unwrap();
            if let Some(dddist) = gtk_objects.get_dddist() {
                dddist.dialog_distribute(idx, dd);
            }
        }
    }

    pub fn show_dialog(&self, idx: u8) {
        let gtk_objects = (*self.g_o_a).read().unwrap();
        if let Some(r_dia) = gtk_objects.get_dialog(idx) {
            r_dia.show_all();
        } else {
            warn!("GMU: show dialog{} not found", idx);
        }
    }

    pub fn update_linkbutton(&self, idx: u8) {
        if let Some((label, url)) = (self.m_v_store).read().unwrap().get_linkbutton_text(idx) {
            let gtk_objects = (*self.g_o_a).read().unwrap();
            if let Some(linkbutton) = gtk_objects.get_linkbutton(idx) {
                linkbutton.set_label(label);
                linkbutton.set_uri(url);
            }
        }
    }

    pub fn update_paned_pos(&self, idx: u8, pos: i32) {
        if let Some(paned) = (*self.g_o_a).read().unwrap().get_paned(idx) {
            paned.set_position(pos);
        }
    }

    //  unavailable db-id:   remove focus.
    //  Tried to:  disconnect, reconnect ->  cursor is gone
    pub fn list_set_cursor(&self, idx: u8, db_id: isize, column: u8, scroll_pos: i8) {
        let g_o = (*self.g_o_a).read().unwrap();
        let o_treestore = g_o.get_list_store(idx as usize);
        if o_treestore.is_none() {
            warn!("list_set_cursor: no treestore idx={} ", idx);
            return;
        }
        let treestore = o_treestore.unwrap();
        let o_treeview = g_o.get_tree_view(idx as usize);
        if o_treeview.is_none() {
            warn!("list_set_cursor: no treeview!");
            return;
        }
        let treeview = o_treeview.unwrap();
        let mut matching_path: Option<TreePath> = None;
        if let Some(iter) = treestore.iter_first() {
            loop {
                let val = treestore.value(&iter, column as i32);
                if let Ok(iter_db_id) = val.get::<u32>() {
                    if iter_db_id as isize == db_id {
                        matching_path = treestore.path(&iter);
                    }
                }
                if !treestore.iter_next(&iter) {
                    break;
                }
            }
        }
        if let Some(t_path) = matching_path {
            // trace!(                "list_set_cursor :  {:?} scroll: {}",                &t_path.indices(),                scroll_pos            );
            let focus_column: Option<&TreeViewColumn> = None;
            (*treeview).set_cursor(&t_path, focus_column, false);
            if scroll_pos >= 0 {
                (*treeview).scroll_to_cell(
                    Some(&t_path),
                    focus_column,
                    true,
                    (scroll_pos as f32) / 100.0,
                    0.0,
                );
            }
        }
    }

    pub fn widget_mark(&self, typ: UIUpdaterMarkWidgetType, idx: u8, mark: u8) {
        let name: &str = match typ {
            UIUpdaterMarkWidgetType::ScrolledWindow => "scrolledwindow_",
            UIUpdaterMarkWidgetType::Box => "box_",
            UIUpdaterMarkWidgetType::TreeView => "treeview_",
            UIUpdaterMarkWidgetType::WebView => "webview_",
        };
        let widget_name = if mark > 0 {
            format!("{name}{idx}_{mark}")
        } else {
            format!("{name}{idx}")
        };
        if let Some(widget) = self.widget_for_typ(typ, idx) {
            widget.set_widget_name(widget_name.as_str());
            widget.queue_draw();
        }
    }

    pub fn grab_focus(&self, typ: UIUpdaterMarkWidgetType, idx: u8) {
        if let Some(widget) = self.widget_for_typ(typ, idx) {
            widget.grab_focus();
        }
    }

    fn widget_for_typ(&self, typ: UIUpdaterMarkWidgetType, idx: u8) -> Option<Widget> {
        let g_o = (*self.g_o_a).read().unwrap();
        match typ {
            UIUpdaterMarkWidgetType::ScrolledWindow => {
                if let Some(sw) = g_o.get_scrolledwindow(idx) {
                    return Some(sw.clone().upcast::<Widget>());
                }
            }
            UIUpdaterMarkWidgetType::Box => {
                if let Some(boxx) = g_o.get_box(idx) {
                    return Some(boxx.clone().upcast::<Widget>());
                }
            }
            UIUpdaterMarkWidgetType::TreeView => {
                if let Some(tv) = g_o.get_tree_view(idx as usize) {
                    return Some(tv.clone().upcast::<Widget>());
                }
            }
            UIUpdaterMarkWidgetType::WebView => {
                if let Some(wv) = g_o.get_web_view() {
                    return Some(wv.upcast::<Widget>());
                }
            }
        }
        None
    }

    pub fn update_window_title(&self) {
        if let Some(window) = (*self.g_o_a).read().unwrap().get_window() {
            let title = (self.m_v_store).read().unwrap().get_window_title();
            window.set_title(&title);
        }
    }

    pub fn update_window_icon(&self) {
        if let Some(window) = (*self.g_o_a).read().unwrap().get_window() {
            let icon_str = (self.m_v_store).write().unwrap().get_window_icon();
            let contained = self.pixbufcache.borrow().contains_key(&icon_str);
            if !contained {
                let pb: Pixbuf = Self::icon_for_string(&icon_str, "window_icon".to_string());
                self.pixbufcache.borrow_mut().insert(icon_str.clone(), pb);
            }
            match self.pixbufcache.borrow().get(&icon_str) {
                Some(e_pb) => {
                    window.set_icon(Some(e_pb));
                }
                None => {
                    panic!("update_window_icon: pixbuf was inserted, but is not there ");
                }
            }
        }
    }

    pub fn memory_conserve(&self, active: bool) {
        if active {
            self.pixbufcache.borrow_mut().clear();
            (*self.g_o_a).write().unwrap().set_web_view(None, None);
        }
        (self.m_v_store).write().unwrap().memory_conserve(active);
    }

    pub fn update_window_minimized(&self, minimized: bool, _ev_time: u32) {
        if let Some(window) = (*self.g_o_a).read().unwrap().get_window() {
            if minimized {
                window.hide();
            } else {
                window.show();
                let _r = window.is_resizable();
                // trace!("win visible1 true {}", _r);
                window.set_visible(true);
                std::thread::sleep(std::time::Duration::from_millis(10));
                window.present();
                std::thread::sleep(std::time::Duration::from_millis(10));
                window.deiconify();
                std::thread::sleep(std::time::Duration::from_millis(10));
                window.set_visible(true);
            }
        }
    }
} // GtkModelUpdaterInt

/// outsourcing this due to  the gtk 0.14  incompatible api
#[cfg(not(feature = "legacy3gtk14"))]
pub fn set_brightness(bright: isize, webview: &WebView) {
    let bright: f64 = bright as f64 / 255.0;
    let c_bg = RGBA::new(bright, bright, bright, 1.0);
    webview.set_background_color(&c_bg);
}

/// No Background setting available  for the old version
// https://docs.rs/webkit2gtk/0.14.0/webkit2gtk/struct.WebView.html
// https://github.com/gtk-rs/gtk3-rs/blob/0.14.3/gdk/src/rgba.rs
#[cfg(feature = "legacy3gtk14")]
pub fn set_brightness(_bright: isize, _webview: &WebView) {}
