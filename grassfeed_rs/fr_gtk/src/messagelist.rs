use crate::cell_data_func::*;
use crate::gtk_object_tree::GLOB_CACHE;
use crate::util::EvSenderCache;
use crate::util::EvSenderWrapper;
use crate::util::MOUSE_BUTTON_LEFT;
use crate::util::MOUSE_BUTTON_RIGHT;
use flume::Sender;
use gdk::EventButton;
use glib::types::Type;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::GtkMenuItemExt;
use gtk::prelude::TreeModelExt;
use gtk::prelude::TreeViewColumnExt;
use gtk::prelude::TreeViewExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::CellRendererPixbuf;
use gtk::CellRendererText;
use gtk::ListStore;
use gtk::Menu;
use gtk::MenuItem;
use gtk::SortType;
use gtk::TreeIter;
use gtk::TreeModel;
use gtk::TreeView;
use gtk::TreeViewColumn;
use gui_layer::abstract_ui::GuiEvents;
use resources::id::*;
use rust_i18n::t;
use ui_gtk::GtkObjectsType;

pub fn create_listview(
    g_ev_se: Sender<GuiEvents>,
    col1width: i32,
    gtk_obj_a: GtkObjectsType,
    sort_column: i32,
    sort_ascending: bool,
) -> TreeView {
    const TYPESTRING_TEXT: &str = "text";
    let content_tree_view = TreeView::new();
    content_tree_view.set_headers_visible(true);
    content_tree_view.set_tooltip_column(6);
    content_tree_view
        .selection()
        .set_mode(gtk::SelectionMode::Multiple);
    content_tree_view.set_activate_on_single_click(false);
    content_tree_view.set_enable_search(false);
    content_tree_view.set_widget_name("msg_list");
    content_tree_view.set_margin_top(2);
    let liststoretypes = &[
        Pixbuf::static_type(), // 0: Fav / feed icon
        Type::STRING,          // title
        Type::STRING,          // date
        Pixbuf::static_type(), // status icon
        u32::static_type(),    // is unread
        u32::static_type(),    // 5 : db-id
        Type::STRING,          // tooltip
    ];
    let title_column: TreeViewColumn;
    let date_column: TreeViewColumn;
    {
        let col = TreeViewColumn::new();
        let cellrendpixbuf = CellRendererPixbuf::new();
        col.pack_start(&cellrendpixbuf, false);
        col.add_attribute(&cellrendpixbuf, "gicon", 0_i32);
        col.set_title("Fav");
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        col.set_fixed_width(25);
        col.set_expand(false);
        col.set_sort_column_id(LIST0_COL_FAVICON);
        content_tree_view.append_column(&col);
    }
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        col.pack_start(&cellrendtext, true);
        col.add_attribute(&cellrendtext, TYPESTRING_TEXT, 1);
        col.set_visible(true);
        col.set_title(&t!("MSGLIST_TOP_TITLE"));
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        col.set_resizable(true);
        col.set_expand(true);
        col.set_min_width(10);
        col.set_max_width(1000);
        col.set_sort_column_id(LIST0_COL_DISPLAYTEXT);
        col.set_sort_indicator(true);
        TreeViewColumnExt::set_cell_data_func(
            &col,
            &cellrendtext,
            Some(Box::new(BoldFunction::<ListBoldDiscr>::tree_switch_bold)),
        );
        content_tree_view.append_column(&col);
        col.connect_resizable_notify(|_col| {
            info!("List resizable ");
        });
        let esw = EvSenderWrapper(g_ev_se.clone());
        col.connect_width_notify(move |col| {
            let new_width: i32 = col.width();
            if new_width != GLOB_CACHE.with(|glob| glob.borrow().col0w) {
                GLOB_CACHE.with(|glob| {
                    glob.borrow_mut().col0w = new_width;
                });
                esw.sendw(GuiEvents::ColumnWidth(1, new_width));
            }
        });
        col.set_fixed_width(col1width);
        title_column = col;
    }
    {
        let col = TreeViewColumn::new(); // is-read
        let cellrendpixbuf = CellRendererPixbuf::new();
        col.pack_end(&cellrendpixbuf, false);
        col.add_attribute(&cellrendpixbuf, "gicon", 3_i32);
        col.set_title("R");
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        col.set_expand(false);
        col.set_min_width(10);
        col.set_max_width(20);
        col.set_sort_column_id(LIST0_COL_ISREAD);
        col.set_resizable(false);
        content_tree_view.append_column(&col);
    }
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        col.pack_end(&cellrendtext, false);
        col.add_attribute(&cellrendtext, TYPESTRING_TEXT, 2);
        col.set_visible(true);
        col.set_title(&t!("MSGLIST_TOP_DATE"));
        col.set_sizing(gtk::TreeViewColumnSizing::GrowOnly);
        col.set_min_width(10);
        col.set_resizable(true);
        col.set_expand(false);
        col.set_sort_column_id(LIST0_COL_TIMESTAMP);
        col.set_sort_indicator(true);
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        TreeViewColumnExt::set_cell_data_func(
            &col,
            &cellrendtext,
            Some(Box::new(BoldFunction::<ListBoldDiscr>::tree_switch_bold)),
        );
        content_tree_view.append_column(&col);
        date_column = col;
    }
    let list_store = ListStore::new(liststoretypes);
    content_tree_view.set_model(Some(&list_store));

    let esw = EvSenderWrapper(g_ev_se.clone());
    content_tree_view.connect_cursor_changed(move |tree_view| {
        let sele = tree_view.selection();
        if tree_view.model().is_none() {
            return;
        }
        if sele.count_selected_rows() != 1 {
            return; // either no row selected, then it's a application focus setting, or too many - then it's a selection range
        }
        let t_model: TreeModel = tree_view.model().unwrap();
        let mut list_pos = -1;
        let mut o_t_iter: Option<TreeIter> = None;
        let (o_tpath, _o_tv_col) = tree_view.cursor();
        if let Some(t_path) = o_tpath {
            let indices = t_path.indices();
            list_pos = indices[0];
            o_t_iter = t_model.iter(&t_path);
        }
        let mut repo_id: i32 = -1;
        if let Some(t_iter) = o_t_iter {
            repo_id = t_model
                .value(&t_iter, LIST0_COL_MSG_ID as i32)
                .get::<u32>()
                .unwrap() as i32;
        }
        // trace!("LIST cursor_changed {}  {:?}  ", list_pos, repo_id);
        if list_pos >= 0 && repo_id > 0 {
            esw.sendw(GuiEvents::ListRowActivated(0, list_pos, repo_id));
        }
    });
    let esw = EvSenderWrapper(g_ev_se.clone());
    content_tree_view
        .selection()
        .connect_changed(move |t_selection| {
            let n_rows = t_selection.count_selected_rows();
            if n_rows <= 0 {
                return;
            }
            let (tp_list, t_model) = t_selection.selected_rows();
            let id_list = tp_list
                .iter()
                .filter_map(|tp| {
                    if let Some(t_iter) = t_model.iter(tp) {
                        if let Ok(val) =
                            t_model.value(&t_iter, LIST0_COL_MSG_ID as i32).get::<u32>()
                        {
                            return Some(val as i32);
                        }
                    }
                    None
                })
                .collect::<Vec<i32>>();
            // trace!("LIST changed multiple   #rows={}  {:?}  ", n_rows, &id_list);
            esw.sendw(GuiEvents::ListSelected(0, id_list));
        });

    content_tree_view.connect_selection_notify_event(|_tv, ev_sel| {
        debug!("LIST  _selection_notify_event  {:?}", ev_sel);
        Inhibit(false)
    });

    let esw = EvSenderWrapper(g_ev_se.clone());
    content_tree_view.connect_row_activated(move |t_view, t_path, _tv_column| {
        let t_model = t_view.model().unwrap();
        let t_iter = t_model.iter(t_path).unwrap();
        let repo_id = t_model
            .value(&t_iter, LIST0_COL_MSG_ID as i32)
            .get::<u32>()
            .unwrap() as i32;
        let list_pos = t_path.indices()[0];
        // trace!(            "row_activated, double click repoid: {} {}",            repo_id,            list_pos        );
        esw.sendw(GuiEvents::ListRowDoubleClicked(0, list_pos, repo_id));
    });

    let ev_se_3 = g_ev_se.clone();
    let esw = EvSenderWrapper(g_ev_se.clone());
    let gtk_obj_ac = gtk_obj_a.clone();
    content_tree_view.connect_button_press_event(
        move |p_tv: &TreeView, eventbutton: &EventButton| {
            let treeview: gtk::TreeView = p_tv.clone().dynamic_cast::<gtk::TreeView>().unwrap();
            let mut repo_id: i32 = -1;
            let button_num = eventbutton.button();
            let (posx, posy) = eventbutton.position();
            if button_num == MOUSE_BUTTON_LEFT {
                if let Some((o_tree_path, o_column, _x, _y)) =
                    treeview.path_at_pos(posx as i32, posy as i32)
                {
                    if let Some(ref t_path) = o_tree_path {
                        let t_model = treeview.model().unwrap();
                        let t_iter = t_model.iter(t_path).unwrap();
                        repo_id = t_model
                            .value(&t_iter, LIST0_COL_MSG_ID as i32)
                            .get::<u32>()
                            .unwrap() as i32;
                    }
                    if let Some(tvc) = o_column {
                        let mut list_pos = -1;
                        if let Some(tp) = o_tree_path {
                            let indices = tp.indices();
                            list_pos = indices[0];
                        }
                        // debug!(                            "button-left list_pos={:?}   SC={}",                            list_pos,                            tvc.sort_column_id()                        );
                        if tvc.sort_column_id() == LIST0_COL_ISREAD
                            || tvc.sort_column_id() == LIST0_COL_FAVICON
                        {
                            esw.sendw(GuiEvents::ListCellClicked(
                                0,
                                list_pos,
                                tvc.sort_column_id(),
                                repo_id,
                            ));
                            return gtk::Inhibit(true); // do  not propagate
                        }
                    }
                }
            }
            if button_num == MOUSE_BUTTON_RIGHT {
                let (tp_list, t_model) = treeview.selection().selected_rows();
                let mut repoid_listpos: Vec<(i32, i32)> = Vec::default();
                if !tp_list.is_empty() {
                    for t_path in tp_list {
                        if let Some(t_iter) = t_model.iter(&t_path) {
                            let repo_id = t_model
                                .value(&t_iter, LIST0_COL_MSG_ID as i32)
                                .get::<u32>()
                                .unwrap() as i32;
                            repoid_listpos.push((repo_id, t_path.indices()[0]));
                        }
                    }
                }
                show_context_menu_message(
                    button_num,
                    ev_se_3.clone(),
                    gtk_obj_ac.clone(),
                    &repoid_listpos,
                );
                return gtk::Inhibit(true); // do  not propagate
            }
            gtk::Inhibit(false) // do propagate
        },
    );
    let targets = vec![
        gtk::TargetEntry::new("STRING", gtk::TargetFlags::OTHER_APP, 0),
        gtk::TargetEntry::new("text/plain", gtk::TargetFlags::OTHER_APP, 0),
        gtk::TargetEntry::new("text/html", gtk::TargetFlags::OTHER_APP, 0),
    ];
    content_tree_view.drag_dest_set(gtk::DestDefaults::ALL, &targets, gdk::DragAction::LINK);
    let esw = EvSenderWrapper(g_ev_se.clone());
    content_tree_view.connect_drag_data_received(
        move |_tv, _dragcontext, _x, _y, selectiondata, _info, _timestamp| {
            if let Some(gstri) = selectiondata.text() {
                debug!("DDR: SEL.text {:?} ", gstri.to_string());
                esw.sendw(GuiEvents::DragDropUrlReceived(gstri.to_string()));
            }
        },
    );

    match sort_column {
        1 => set_sort_indicator(&title_column, sort_column, sort_ascending),
        2 => set_sort_indicator(&date_column, sort_column, sort_ascending),
        _ => (),
    };
    set_column_notifier(&title_column, g_ev_se.clone());
    set_column_notifier(&date_column, g_ev_se);
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_tree_view(TREEVIEW1, &content_tree_view);
    ret.set_list_store(TREEVIEW1, &list_store);
    ret.set_list_store_max_columns(TREEVIEW1 as usize, liststoretypes.len() as u8);
    content_tree_view
}

fn show_context_menu_message(
    ev_button: u32,
    g_ev_se: Sender<GuiEvents>,
    _gtk_obj_a: GtkObjectsType,
    repoid_listpos: &Vec<(i32, i32)>,
) {
    let mi_mark_read = MenuItem::with_label(&t!("CM_MSG_MARK_AS_READ"));
    let esc = EvSenderCache(
        g_ev_se.clone(),
        GuiEvents::ListSelectedAction(0, "mark-as-read".to_string(), repoid_listpos.clone()),
    );
    mi_mark_read.connect_activate(move |_menuiten| {
        esc.send();
    });
    let mi_mark_unread = MenuItem::with_label(&t!("CM_MSG_MARK_AS_UNREAD"));
    let esc = EvSenderCache(
        g_ev_se.clone(),
        GuiEvents::ListSelectedAction(0, "mark-as-unread".to_string(), repoid_listpos.clone()),
    );
    mi_mark_unread.connect_activate(move |_menuiten| {
        esc.send();
    });
    let mi_open_browser = MenuItem::with_label(&t!("CM_MSG_OPEN_IN_BROWSER"));
    let esc = EvSenderCache(
        g_ev_se.clone(),
        GuiEvents::ListSelectedAction(0, "open-in-browser".to_string(), repoid_listpos.clone()),
    );
    mi_open_browser.connect_activate(move |_menuiten| {
        esc.send();
    });

    let mi_delete = MenuItem::with_label(&t!("CM_MSG_DELETE"));
    let esc = EvSenderCache(
        g_ev_se.clone(),
        GuiEvents::ListSelectedAction(0, "messages-delete".to_string(), repoid_listpos.clone()),
    );
    mi_delete.connect_activate(move |_menuiten| {
        esc.send();
    });

    let mi_copy_link = MenuItem::with_label(&t!("CM_MSG_COPY_LINK_CLIPBOARD"));
    if repoid_listpos.len() == 1 {
        let esc = EvSenderCache(
            g_ev_se.clone(),
            GuiEvents::ListSelectedAction(
                0,
                "message-copy-link".to_string(),
                repoid_listpos.clone(),
            ),
        );
        mi_copy_link.connect_activate(move |_menuiten| {
            esc.send();
        });
    }

    let mi_mark_favorite = MenuItem::with_label(&t!("CM_MSG_MARK_FAVORITE"));
    let esc = EvSenderCache(
        g_ev_se.clone(),
        GuiEvents::ListSelectedAction(0, "mark-as-favorite".to_string(), repoid_listpos.clone()),
    );
    mi_mark_favorite.connect_activate(move |_menuiten| {
        esc.send();
    });
    let mi_unmark_favorite = MenuItem::with_label(&t!("CM_MSG_UNMARK_FAVORITE"));
    let esc = EvSenderCache(
        g_ev_se.clone(),
        GuiEvents::ListSelectedAction(0, "unmark-favorite".to_string(), repoid_listpos.clone()),
    );
    mi_unmark_favorite.connect_activate(move |_menuiten| {
        esc.send();
    });


    let menu: gtk::Menu = Menu::new();
    menu.append(&mi_open_browser);
    if repoid_listpos.len() == 1 {
        menu.append(&mi_copy_link);
    }
    menu.append(&mi_mark_read);
    menu.append(&mi_mark_unread);
    menu.append(&mi_delete);
    menu.append(&mi_mark_favorite);
    menu.append(&mi_unmark_favorite);
    menu.show_all();
    let c_ev_time = gtk::current_event_time();
    menu.popup_easy(ev_button, c_ev_time);
}

fn set_sort_indicator(tvc: &TreeViewColumn, _sort_column: i32, sort_ascending: bool) {
    let sorttype = match sort_ascending {
        true => SortType::Ascending,
        _ => SortType::Descending,
    };

    tvc.set_sort_order(sorttype);
    tvc.clicked();
    if !sort_ascending {
        tvc.clicked();
    }
}

fn set_column_notifier(col: &TreeViewColumn, g_ev_se: Sender<GuiEvents>) {
    let esw = EvSenderWrapper(g_ev_se.clone());
    col.connect_sort_order_notify(move |col| {
        //  trace!(            "sort_order_notify:  sort_order_notify col={} {:?}",            col.sort_column_id(),            col.sort_order()        );
        esw.sendw(GuiEvents::ListSortOrderChanged(
            0,
            col.sort_column_id() as u8,
            col.sort_order() == SortType::Ascending,
        ));
    });
    let esw = EvSenderWrapper(g_ev_se);
    col.connect_sort_column_id_notify(move |col| {
        // trace!(            "sort_column_id_notify:  column_id_notify {} : {} ",            col.sort_column_id(),            col.is_sort_indicator()        );
        esw.sendw(GuiEvents::ListSortOrderChanged(
            0,
            col.sort_column_id() as u8,
            col.sort_order() == SortType::Ascending,
        ));
    });
}
