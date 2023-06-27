use core::slice::Iter;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiTreeItem;
use gui_layer::abstract_ui::TreeRowExpand;
use gui_layer::abstract_ui::UIAdapterValueStore;
use gui_layer::gui_values::PropDef;
use std::collections::HashMap;

#[derive(Default)]
pub struct ModelValueStoreImpl {
    pub gui_tree_root: GuiTreeItem,
    pub gui_list: Vec<Vec<AValue>>, //  rows of  items
    pub gui_text_views: Vec<String>,
    pub gui_web_view_texts: Vec<String>,
    pub gui_text_entry_content: Vec<String>,
    pub gui_spinner_active: bool,
    pub gui_label_texts: Vec<String>,
    pub gui_linkbutton: Vec<(String, String)>,
    pub expand_data: Vec<(usize, u32)>,
    pub dialog_data: Vec<Vec<AValue>>, //  One set for each dialog
    pub properties: HashMap<PropDef, String>,
    pub window_title: String,
    pub window_icon: String,
    pub gui_label_tooltips: Vec<String>,
}

impl ModelValueStoreImpl {
    pub fn new() -> Self {
        ModelValueStoreImpl {
            gui_tree_root: GuiTreeItem::new_named_("+"),
            ..Default::default()
        }
    }

    fn get_tree_add_node_for_path(&mut self, path: &[u16]) -> Option<(&mut GuiTreeItem, u16)> {
        assert!(!path.is_empty());
        let mut add_node = &mut self.gui_tree_root;
        let (last_path_pos, reduced_path): (&u16, &[u16]) = path.split_last().unwrap();
        for p_index in reduced_path.iter() {
            if *p_index as usize >= add_node.children.len() {
                error!(
                    "get_tree_add_node_for_path_1  {path:?}  {}>={}  Skipping  node={:?}  ",
                    *p_index,
                    add_node.children.len(),
                    &add_node.a_values
                );
                return None;
            }
            add_node = &mut add_node.children[*p_index as usize];
        }
        if *last_path_pos > add_node.children.len() as u16 {
            error!(  "get_tree_add_node_for_path_2 {path:?}  lastpos={} >= children.len:{:?} add_node_values={:?} ",
                *last_path_pos,                add_node.children.len(),				&add_node.a_values            );
            return None;
        }
        Some((add_node, *last_path_pos))
    }

    fn get_tree_add_node_for_path_ro(&self, path: &[u16]) -> Option<&GuiTreeItem> {
        assert!(!path.is_empty());
        let mut add_node = &self.gui_tree_root;
        let (last_path_pos, reduced_path): (&u16, &[u16]) = path.split_last().unwrap();
        for p_index in reduced_path.iter() {
            if *p_index as usize >= add_node.children.len() {
                error!("BadPath_R3 {:?}  Skipping  ", &path);
                return None;
            }
            add_node = &add_node.children[*p_index as usize];
        }
        if *last_path_pos > add_node.children.len() as u16 {
            error!("BadPath_R4 {:?} last:{:?} ", &path, add_node.children.len(),);
            return None;
        }
        Some(add_node)
    }

    #[allow(dead_code)]
    fn debug_dump_tree(&self, ident: &str) {
        ModelValueStoreImpl::dump_elements_r(&self.gui_tree_root, &[], ident);
        debug!("\\----------------------------/ {}", ident);
    }

    #[allow(dead_code)]
    fn dump_elements_r(node: &GuiTreeItem, path: &[u16], ident: &str) {
        trace!(
            "DUMP {}:\t{:?}\t{:?}\t{:?}",
            ident,
            &path,
            node.a_values.get(5),
            node.a_values.get(1)
        );
        node.children.iter().enumerate().for_each(|(i, n)| {
            let mut n_path = path.to_vec();
            n_path.push(i as u16);
            ModelValueStoreImpl::dump_elements_r(n, n_path.as_slice(), ident);
        });
    }
}

impl UIAdapterValueStore for ModelValueStoreImpl {
    fn memory_conserve(&mut self, active: bool) {
        if active {
            self.gui_list.clear();
        }
    }

    fn set_text_entry(&mut self, idx: u8, newtext: String) {
        if (self.gui_text_entry_content.len() as u8) <= idx {
            self.gui_text_entry_content
                .resize(idx as usize + 1, "".to_string());
        }
        self.gui_text_entry_content[idx as usize] = newtext;
    }

    fn get_text_entry(&self, idx: u8) -> Option<String> {
        if (idx as usize) < self.gui_text_entry_content.len() {
            Some(self.gui_text_entry_content[idx as usize].clone())
        } else {
            None
        }
    }

    ///  Pushes an element at the bottom of the  path's children list.
    ///  * path cannot  be an empty array.   Need to assign a definite path position where the new element shall go to
    ///    replace = false -->  insert the element.      true :  put it in place
    fn insert_tree_item(&mut self, path: &[u16], treevalues: &[AValue]) {
        assert!(!treevalues.is_empty());
        if let Some((ref mut add_node, last_path_pos)) = self.get_tree_add_node_for_path(path) {
            add_node
                .children
                .insert(last_path_pos as usize, GuiTreeItem::new_values(treevalues));
        } else {
            error!(
                "insert_tree_item: BadPath {:?}  Skipping {:?} ",
                &path, &treevalues
            );
        }
    }

    fn get_tree_item(&self, path: &[u16]) -> Vec<AValue> {
        if let Some(add_node) = self.get_tree_add_node_for_path_ro(path) {
            return add_node.a_values.clone();
        }
        warn!("get_tree_item: no entries for path {:?}", path);
        Vec::default()
    }

    fn get_tree_root(&self) -> GuiTreeItem {
        self.gui_tree_root.clone()
    }

    ///  Replaces an element
    ///  * path cannot  be an empty array.   Need to assign a definite path position where the new element shall go to
    fn replace_tree_item(&mut self, path: &[u16], treevalues: &[AValue]) {
        if treevalues.is_empty() {
            error!(" replace_tree_item : treevalues is empty ");
            return;
        }
        if path.is_empty() {
            error!(" replace_tree_item : path is empty ");
            return;
        }
        if let Some((ref mut add_node, last_path_pos)) = self.get_tree_add_node_for_path(path) {
            let mut new_child = GuiTreeItem::new_values(treevalues);
            if add_node.children.len() < (last_path_pos + 1) as usize {
                add_node
                    .children
                    .resize((last_path_pos + 1) as usize, new_child.clone());
            }
            new_child.children = (add_node).children[last_path_pos as usize].children.clone();
            (add_node).children[last_path_pos as usize] = new_child;
        } else {
            error!(
                "replace_tree_item: BadPath {:?}  Skipping {:?} ",
                &path, &treevalues
            );
            self.debug_dump_tree("REPL");
        }
    }

    fn clear_tree(&mut self, _tree_index: u8) {
        assert!(_tree_index == 0); // later: use list of trees
        self.gui_tree_root.a_values.clear();
        self.gui_tree_root.children.clear();
    }

    ///   insert a new   GuiListEntry  does not handle multiple lists yet.
    //  Later:  use list index
    fn insert_list_item(&mut self, _list_index: u8, list_position: i32, values: &[AValue]) {
        while self.gui_list.len() <= list_position as usize {
            self.gui_list.push(Vec::default());
        }
        self.gui_list[list_position as usize] = values.to_owned();
    }

    //  Later: use list index
    fn clear_list(&mut self, _list_index: u8) {
        self.gui_list.clear();
    }

    ///   GuiListEntry  does not handle multiple lists yet
    fn get_list_item(&self, _list_index: u8, list_position: i32) -> Option<Vec<AValue>> {
        if self.gui_list.len() <= list_position as usize {
            error!(
                "get_list_item:  requested index {} , list has only  {} ",
                list_position,
                self.gui_list.len()
            );
            return None;
        }
        let o_val = self.gui_list.get(list_position as usize);
        if let Some(gle) = o_val {
            return Some(gle.clone());
        }
        None
    }

    fn get_list_iter(&self, _list_index: u8) -> Iter<Vec<AValue>> {
        self.gui_list.iter()
    }

    fn set_text_view(&mut self, text_view_index: u8, newtext: String) {
        if (self.gui_text_views.len() as u8) <= text_view_index {
            self.gui_text_views
                .resize(text_view_index as usize + 1, "".to_string());
        }
        self.gui_text_views[text_view_index as usize] = newtext;
    }

    fn get_text_view(&self, index: u8) -> Option<String> {
        self.gui_text_views.get(index as usize).cloned()
    }

    fn set_web_view_text(&mut self, text_view_index: u8, newtext: String) {
        if (self.gui_web_view_texts.len() as u8) <= text_view_index {
            self.gui_web_view_texts
                .resize(text_view_index as usize + 1, "".to_string());
        }
        self.gui_web_view_texts[text_view_index as usize] = newtext;
    }

    fn get_web_view_text(&self, index: u8) -> Option<String> {
        self.gui_web_view_texts.get(index as usize).cloned()
    }

    fn set_spinner_active(&mut self, a: bool) {
        self.gui_spinner_active = a;
    }

    fn is_spinner_active(&self) -> bool {
        self.gui_spinner_active
    }

    fn set_tree_row_expand(&mut self, idx: usize, column: usize, bitmask: u32) {
        if self.expand_data.len() < (idx + 1) {
            self.expand_data.resize(idx + 1, (0, 0));
        }
        self.expand_data[idx] = (column, bitmask);
    }

    fn get_tree_row_expand(&self, idx: usize) -> (usize, u32) {
        self.expand_data[idx]
    }

    fn is_tree_row_expanded(&self, idx: usize, gti: &GuiTreeItem) -> bool {
        if self.expand_data.len() < (idx + 1) {
            warn!("is_tree_row_expanded  {} : no expand set! ", idx);
            return false;
        }
        TreeRowExpand::is_expanded(gti, self.expand_data[idx])
    }

    fn set_label_text(&mut self, index: u8, newtext: String) {
        if (self.gui_label_texts.len() as u8) <= index {
            self.gui_label_texts
                .resize(index as usize + 1, "".to_string());
        }
        self.gui_label_texts[index as usize] = newtext;
    }

    fn get_label_text(&self, index: u8) -> Option<&String> {
        self.gui_label_texts.get(index as usize)
    }

    fn set_dialog_data(&mut self, idx: u8, values: &[AValue]) {
        if (self.dialog_data.len() as u8) <= idx {
            self.dialog_data
                .resize(idx as usize + 1, Vec::<AValue>::default());
        }
        self.dialog_data[idx as usize] = values.to_owned();
    }

    fn get_dialog_data(&self, idx: u8) -> Option<&Vec<AValue>> {
        self.dialog_data.get(idx as usize)
    }

    fn set_gui_property(&mut self, name: PropDef, value: String) {
        self.properties.insert(name, value);
    }
    fn get_gui_property_or(&self, name: PropDef, default: String) -> String {
        self.properties.get(&name).unwrap_or(&default).to_string()
    }
    fn get_gui_int_or(&self, name: PropDef, default: isize) -> isize {
        match self
            .properties
            .get(&name)
            .map(|s| s.parse::<isize>().unwrap_or(default))
        {
            Some(n) => n,
            None => default,
        }
    }
    fn get_gui_bool(&self, name: PropDef) -> bool {
        self.properties
            .get(&name)
            .map(|s| s.parse::<bool>().unwrap_or(false))
            .unwrap_or(false)
    }

    fn set_gui_properties(&mut self) -> HashMap<PropDef, String> {
        self.properties.clone()
    }

    fn set_window_title(&mut self, t: String) {
        self.window_title = t;
    }

    fn get_window_title(&self) -> String {
        self.window_title.clone()
    }

    fn set_linkbutton_text(&mut self, index: u8, text_uri: (String, String)) {
        if (self.gui_linkbutton.len() as u8) <= index {
            self.gui_linkbutton
                .resize(index as usize + 1, (String::default(), String::default()));
        }
        self.gui_linkbutton[index as usize] = text_uri;
    }

    fn get_linkbutton_text(&self, index: u8) -> Option<&(String, String)> {
        self.gui_linkbutton.get(index as usize)
    }

    fn set_window_icon(&mut self, icon_compressed: String) {
        self.window_icon = icon_compressed;
    }
    fn get_window_icon(&mut self) -> String {
        let s = self.window_icon.clone();
        self.window_icon.clear();
        s
    }

    fn set_label_tooltip(&mut self, index: u8, newtext: String) {
        if (self.gui_label_tooltips.len() as u8) <= index {
            self.gui_label_tooltips
                .resize(index as usize + 1, String::default());
        }
        self.gui_label_tooltips[index as usize] = newtext;
    }
    fn get_label_tooltip(&self, index: u8) -> Option<&String> {
        self.gui_label_tooltips.get(index as usize)
    }

    fn get_list_length(&self, _list_index: u8) -> usize {
        self.gui_list.len()
    }
}
