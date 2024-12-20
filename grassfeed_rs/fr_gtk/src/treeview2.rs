use crate::cell_data_func::*;
use crate::util::DragState;
use crate::util::EvSenderWrapper;
use crate::util::DIALOG_ICON_SIZE;
use crate::util::MOUSE_BUTTON_RIGHT;
use flume::Sender;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::TreeModelExt;
use gtk::prelude::TreeViewColumnExt;
use gtk::prelude::TreeViewExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::CellRendererPixbuf;
use gtk::CellRendererSpinner;
use gtk::CellRendererText;
use gtk::Menu;
use gtk::MenuItem;
use gtk::TreeStore;
use gtk::TreeView;
use gtk::TreeViewColumn;
use gtk::TreeViewGridLines;
use gui_layer::abstract_ui::GuiEvents;
use resources::id::*;
use rust_i18n;
use rust_i18n::t;
use std::rc::Rc;
use std::sync::RwLock;
use ui_gtk::dialogdatadistributor::DialogDataDistributor;
use ui_gtk::GtkObjectsType;

const TREEVIEW_NAME: &str = "TREEVIEW1";
const TV_ID: u8 = 0;
const COL1WIDE_WIDTH: i32 = 80;
const COL1NARROW_WIDTH: i32 = 44;

// Later: check with gtk4:  if we don't fix the width of tree column1, the gtk system crashes on moving the pane-1

pub fn create_tree_store() -> (TreeStore, usize) {
    let tree_store_types: &[gtk::glib::Type] = &[
        gtk::gdk_pixbuf::Pixbuf::static_type(), // 0: feed-icon
        String::static_type(),                  // 1: Feed-Source-Name
        String::static_type(),                  // 2: unread-column  text
        Pixbuf::static_type(),                  // 3: status-icon
        bool::static_type(),                    // 4: is-Folder
        u32::static_type(),                     // 5: db-id
        u32::static_type(),                     // 6: num-unread
        u32::static_type(),                     // 7: status
        String::static_type(),                  // 8: ToolTip	== TREE0_COL_TOOLTIP
        bool::static_type(),                    // 9: Spinner active, visible
        bool::static_type(),                    // 10: Status Icon Visible
        bool::static_type(),                    // 11: unread-text visible
    ];
    let tree_store = TreeStore::new(tree_store_types);
    (tree_store, tree_store_types.len())
}

// Store:	Feed-Icon	Feed-Source-Name	Number-items	Status-Icon		Is-Folder	DB-ID	num-unread
// View:	Feed-Icon	Feed-Source-Name	Number-items	Status-Icon
//
// tree_store.connect_row_deleted(); Does not give a usable path, useless for drag recognition
//
// https://developer-old.gnome.org/pygtk/stable/class-gtkcellrenderer.html
// https://docs.gtk.org/gtk4/class.CellRendererText.html
//
//  CellRenderer Attributes:   see  gtk3 / cell_renderer_text.rs
//  https://github.com/gtk-rs/gtk3-rs/blob/master/gtk/src/auto/cell_renderer_text.rs
pub fn create_treeview(
    g_ev_se: Sender<GuiEvents>,
    drag_state: Rc<RwLock<DragState>>,
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) -> TreeView {
    let (tree_store, num_store_types) = create_tree_store();
    let treeview1 = TreeView::new();
    treeview1.set_widget_name(TREEVIEW_NAME);
    treeview1.set_model(Some(&tree_store));
    treeview1.set_grid_lines(TreeViewGridLines::Vertical);
    treeview1.set_headers_visible(false);
    treeview1.set_reorderable(true);
    treeview1.set_tooltip_column(TREE0_COL_TOOLTIP);
    treeview1.set_width_request(100);
    treeview1.set_enable_search(false);
    let cellrenderer_spinner = CellRendererSpinner::new();
    cellrenderer_spinner.set_active(true);
    let tree0column1 = TreeViewColumn::new();
    {
        let col = TreeViewColumn::new();
        let cellrendpixbuf = CellRendererPixbuf::new();
        CellLayoutExt::pack_start(&col, &cellrendpixbuf, false);
        CellLayoutExt::add_attribute(&col, &cellrendpixbuf, "gicon", 0_i32);
        let cellrendtext = CellRendererText::new();
        CellLayoutExt::pack_start(&col, &cellrendtext, true);
        CellLayoutExt::add_attribute(&col, &cellrendtext, "text", 1_i32); // display-name
        TreeViewColumnExt::set_cell_data_func(
            &col,
            &cellrendtext,
            Some(Box::new(BoldFunction::<TreeBoldDiscr>::tree_switch_bold)),
        );
        col.set_expand(true); // needed
        col.set_max_width(DIALOG_ICON_SIZE * 2); // needed
        col.set_min_width(DIALOG_ICON_SIZE); //  help with  maintaining minimum width
        treeview1.append_column(&col);
    }
    {
        let col = tree0column1.clone();
        let cellrendtext = CellRendererText::new();
        CellLayoutExt::pack_start(&col, &cellrendtext, false);
        CellLayoutExt::add_attribute(&col, &cellrendtext, "text", 2_i32); // unread-text
        CellLayoutExt::add_attribute(&col, &cellrendtext, "visible", 11_i32); //  unread-text visible
        TreeViewColumnExt::set_cell_data_func(
            &col,
            &cellrendtext,
            Some(Box::new(BoldFunction::<TreeBoldDiscr>::tree_switch_bold)),
        );
        let cellrendpixbuf = CellRendererPixbuf::new();
        CellLayoutExt::pack_end(&col, &cellrendpixbuf, false);
        CellLayoutExt::add_attribute(&col, &cellrendpixbuf, "gicon", 3_i32);
        gtk::prelude::CellLayoutExt::add_attribute(&col, &cellrendpixbuf, "visible", 10_i32);
        gtk::prelude::CellLayoutExt::pack_end(&col, &cellrenderer_spinner, false);
        gtk::prelude::CellLayoutExt::add_attribute(&col, &cellrenderer_spinner, "active", 9_i32);
        gtk::prelude::CellLayoutExt::add_attribute(&col, &cellrenderer_spinner, "visible", 9_i32);
        col.set_fixed_width(9); // If we don't fix the width, the gtk system crashes on moving the pane-1
        treeview1.append_column(&col);
    }
    let drag_s7 = drag_state.clone();
    let esw = EvSenderWrapper(g_ev_se.clone());
    let g_o_a_c = gtk_obj_a.clone();
    let connect_cursor_changed_handler = move |treeview: &TreeView| {
        let (o_tp, _tree_view_column) = treeview.cursor();
        if let Some(mut treepath) = o_tp {
            let tree_blocked = (*g_o_a_c).read().unwrap().get_block_tree_updates(TV_ID);
            let in_drag = (*drag_s7).read().unwrap().block_row_activated();
            if !in_drag {
                if tree_blocked {
                    // debug!(                        " in-drag:{in_drag}   tree-blocked:{tree_blocked}  TreeRowActivated  {:?} ",                        treepath.indices_with_depth()                    );
                } else {
                    let mut repo_id: i32 = -1;
                    let selection = treeview.selection();
                    if let Some((model, iter)) = selection.selected() {
                        repo_id =
                            model.value(&iter, TREE0_COL_REPO_ID).get::<u32>().unwrap() as i32;
                    }
                    let indices = treepath.indices_with_depth();
                    let ind_u16: Vec<u16> = indices.iter().map(|v| *v as u16).collect::<Vec<u16>>();
                    esw.sendw(GuiEvents::TreeCursorChanged(TV_ID, ind_u16, repo_id));
                }
            }
        }
    };
    treeview1.connect_cursor_changed(connect_cursor_changed_handler);
    let ev_se_3 = g_ev_se.clone();
    treeview1.connect_button_press_event(move |p_tv, ev_but| {
        let mut source_repo_id: i32 = -1;
        let (posx, posy) = ev_but.position();
        let treeview: gtk::TreeView = p_tv.clone().dynamic_cast::<gtk::TreeView>().unwrap();
        let mut is_folder: bool = false;
        if let Some((Some(t_path), _o_tvc, _x, _y)) = treeview.path_at_pos(posx as i32, posy as i32)
        {
            if let Some(t_model) = treeview.model() {
                let t_iter = t_model.iter(&t_path).unwrap();
                source_repo_id = t_model
                    .value(&t_iter, TREE0_COL_REPO_ID)
                    .get::<u32>()
                    .unwrap() as i32;

                is_folder = t_model
                    .value(&t_iter, TREE0_COL_ISFOLDER)
                    .get::<bool>()
                    .unwrap();
            }
        }
        if ev_but.button() == MOUSE_BUTTON_RIGHT {
            show_context_menu_source(ev_but.button(), source_repo_id, ev_se_3.clone(), is_folder);
        }
        gtk::Inhibit(false)
    });
    // Drag Events:
    let drag_s4 = drag_state.clone();
    tree_store.connect_row_inserted(move |_t_model, t_path, _t_iter| {
        let in_drag = (*drag_s4).read().unwrap().drag_start_path.is_some();
        if in_drag {
            let indices = t_path
                .indices()
                .iter()
                .map(|a| *a as u16)
                .collect::<Vec<u16>>();
            let mut w_state = (*drag_s4).write().unwrap();
            if w_state.inserted.is_none() {
                w_state.inserted.replace(indices);
            }
        }
    });
    let drag_s2 = drag_state.clone();
    treeview1.connect_drag_begin(move |_t_view, _drag_context| {
        let (o_t_path, _) = _t_view.cursor();

        if let Some(t_path) = o_t_path {
            (*drag_s2).write().unwrap().drag_start_path = Some(t_path);
            let _makeitempty = (*drag_s2).write().unwrap().inserted.take();
        }
    });
    let drag_s3 = drag_state.clone();
    let esw = EvSenderWrapper(g_ev_se.clone());
    treeview1.connect_drag_end(move |_t_view, _drag_context| {
        let r_state = (*drag_s3).read().unwrap();
        if r_state.drag_start_path.is_some()
            && r_state.inserted.is_some()
            && r_state.deleted.is_some()
        {
            drop(r_state);
            let mut w_state = (*drag_s3).write().unwrap();
            let inserted = w_state.inserted.take().unwrap();
            let deleted = w_state.deleted.take().unwrap();
            let start_path = w_state.drag_start_path.take().unwrap();
            drop(w_state);
            if inserted != deleted {
                debug!("Dragged   {:?} ==> {:?}  End  ", &deleted, &inserted);
                esw.sendw(GuiEvents::TreeDragEvent(TV_ID, deleted, inserted));
            }
            let focus_column: Option<&TreeViewColumn> = None;
            _t_view.set_cursor(&start_path, focus_column, false);
        }
    });
    let drag_s1 = drag_state;
    treeview1.connect_drag_data_get(move |_t_view, _dragcontext, _sel_data, _i1, _i2| {
        let in_drag = (*drag_s1).read().unwrap().drag_start_path.is_some();
        if in_drag {
            let (o_t_path, _) = _t_view.cursor();
            if let Some(t_path) = o_t_path {
                let indices = t_path
                    .indices()
                    .iter()
                    .map(|a| *a as u16)
                    .collect::<Vec<u16>>();
                (*drag_s1).write().unwrap().deleted.replace(indices);
            }
        }
    });
    let t_v_1c = treeview1.clone();
    tree_store.connect_row_has_child_toggled(move |_t_model, t_path, t_iter| {
        if let Some(t_model) = t_v_1c.model() {
            let status = t_model
                .value(t_iter, TREE0_COL_STATUS)
                .get::<u32>()
                .unwrap();
            if status & TREE0_COL_STATUS_EXPANDED > 0 {
                let _row_existed = t_v_1c.expand_row(t_path, false);
            }
        }
    });
    let esw = EvSenderWrapper(g_ev_se.clone());
    treeview1.connect_row_expanded(move |t_view, t_iter, _t_path| {
        if let Some(model) = t_view.model() {
            let repo_id = model.value(t_iter, TREE0_COL_REPO_ID).get::<u32>().unwrap() as i32;
            esw.sendw(GuiEvents::TreeExpanded(TV_ID, repo_id));
        }
    });

    let esw = EvSenderWrapper(g_ev_se.clone());
    treeview1.connect_row_collapsed(move |t_view, t_iter, _t_path| {
        if let Some(model) = t_view.model() {
            let repo_id = model.value(t_iter, TREE0_COL_REPO_ID).get::<u32>().unwrap() as i32;
            esw.sendw(GuiEvents::TreeCollapsed(TV_ID, repo_id));
        }
    });
    let esw = EvSenderWrapper(g_ev_se);
    treeview1.connect_row_activated(move |t_view, t_path, _tv_col| {
        if let Some(model) = t_view.model() {
            if let Some(t_iter) = model.iter(t_path) {
                let db_id = model
                    .value(&t_iter, TREE0_COL_REPO_ID)
                    .get::<u32>()
                    .unwrap() as i32;
                esw.sendw(GuiEvents::TreeDoubleClick(TV_ID, db_id));
            }
        }
    });
    {
        let mut ret = (*gtk_obj_a).write().unwrap();
        ret.set_tree_store(TREEVIEW0, &tree_store);
        ret.set_tree_view(TREEVIEW0, &treeview1);
        ret.set_tree_store_max_columns(TV_ID, num_store_types as u8);
        if let Some(col1) = treeview1.column(1) {
            ret.set_spinner_w((cellrenderer_spinner, col1));
        }
    }
    ddd.set_dialog_distribute(DIALOG_TREE0COL1, move |dialogdata| {
        let mut col0width = COL1NARROW_WIDTH;
        if dialogdata.first().unwrap().boo() {
            col0width = COL1WIDE_WIDTH;
        }
        tree0column1.set_fixed_width(col0width);
    });
    treeview1
}

fn show_context_menu_source(
    ev_button: u32,
    subscription_id: i32,
    g_ev_se: Sender<GuiEvents>,
    is_folder: bool,
) {
    let menu: gtk::Menu = Menu::new();
    let mi_addfeed = MenuItem::with_label(&t!("CM_SUB_ADD_FEED"));
    let esw = EvSenderWrapper(g_ev_se.clone());
    mi_addfeed.connect_activate(move |_menuiten| {
        esw.sendw(GuiEvents::TreeEvent(
            TV_ID,
            subscription_id,
            "new-subscription-dialog".to_string(),
        ));
    });

    let mi_afo = MenuItem::with_label(&t!("CM_SUB_ADD_FOLDER"));
    let esw = EvSenderWrapper(g_ev_se.clone());
    mi_afo.connect_activate(move |_menuiten| {
        esw.sendw(GuiEvents::TreeEvent(
            TV_ID,
            subscription_id,
            "new-folder-dialog".to_string(),
        ));
    });
    let mi_update = MenuItem::with_label(&t!("CM_SUB_UPDATE"));
    let esw = EvSenderWrapper(g_ev_se.clone());
    mi_update.connect_activate(move |_menuiten| {
        esw.sendw(GuiEvents::TreeEvent(
            TV_ID,
            subscription_id,
            "feedsource-update".to_string(),
        ));
    });

    let mi_mark_all = MenuItem::with_label(&t!("CM_SUB_MARK_AS_READ"));
    let esw = EvSenderWrapper(g_ev_se.clone());
    mi_mark_all.connect_activate(move |_menuiten| {
        esw.sendw(GuiEvents::TreeEvent(
            TV_ID,
            subscription_id,
            "feedsource-mark-as-read".to_string(),
        ));
    });
    let mi_edit = MenuItem::with_label(&t!("CM_SUB_EDIT"));
    let esw = EvSenderWrapper(g_ev_se.clone());
    mi_edit.connect_activate(move |_menuiten| {
        esw.sendw(GuiEvents::TreeEvent(
            TV_ID,
            subscription_id,
            "feedsource-edit-dialog".to_string(),
        ));
    });
    let esw = EvSenderWrapper(g_ev_se.clone());
    let mi_del = MenuItem::with_label(&t!("CM_SUB_DELETE"));
    mi_del.connect_activate(move |_menuiten| {
        esw.sendw(GuiEvents::TreeEvent(
            TV_ID,
            subscription_id,
            "feedsource-delete-dialog".to_string(),
        ));
    });

    let esw = EvSenderWrapper(g_ev_se);
    let mi_stats = MenuItem::with_label(&t!("CM_SUBS_STATISTICS"));
    mi_stats.connect_activate(move |_menuiten| {
        esw.sendw(GuiEvents::TreeEvent(
            TV_ID,
            subscription_id,
            "subscription-statistics-dialog".to_string(),
        ));
    });
    // trace!("add  RM {} {} ", subscription_id, is_folder);
    if subscription_id >= 0 {
        menu.append(&mi_mark_all);
        menu.append(&mi_update);
        menu.append(&mi_edit);
        if !is_folder {
            menu.append(&mi_stats);
        }
        menu.append(&mi_del);
    }
    menu.append(&mi_addfeed);
    menu.append(&mi_afo);
    menu.show_all();
    let c_ev_time = gtk::current_event_time();
    menu.popup_easy(ev_button, c_ev_time);
}
