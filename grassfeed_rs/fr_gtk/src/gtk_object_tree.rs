use crate::cell_data_func::*;
use crate::dialogs::create_dialogs;
use crate::load_css::TAB_MARKER_HEIGHT;
use crate::util::process_string_to_image;
use crate::util::DragState;
use crate::util::EvSenderCache;
use crate::util::EvSenderWrapper;
use crate::util::MOUSE_BUTTON_LEFT;
use crate::util::MOUSE_BUTTON_RIGHT;
use flume::Sender;
use gdk::EventButton;
use glib::types::Type;
use gtk::builders::ToggleToolButtonBuilder;
use gtk::builders::ToolButtonBuilder;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::pango::WrapMode;
use gtk::prelude::ContainerExt;
use gtk::prelude::GtkMenuItemExt;
use gtk::prelude::TreeModelExt;
use gtk::prelude::TreeViewColumnExt;
use gtk::prelude::TreeViewExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::Adjustment;
use gtk::Align;
use gtk::Button;
use gtk::ButtonBox;
use gtk::CellRendererPixbuf;
use gtk::CellRendererText;
use gtk::Container;
use gtk::Dialog;
use gtk::IconSize;
use gtk::Image;
use gtk::Label;
use gtk::ListStore;
use gtk::Menu;
use gtk::MenuBar;
use gtk::MenuItem;
use gtk::Orientation;
use gtk::Paned;
use gtk::ResizeMode;
use gtk::ScrolledWindow;
use gtk::ShadowType;
use gtk::SortType;
use gtk::ToggleToolButton;
use gtk::ToolButton;
use gtk::Toolbar;
use gtk::TreeIter;
use gtk::TreeModel;
use gtk::TreeView;
use gtk::TreeViewColumn;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::gui_values::PropDef;
use resources::gen_icons;
use resources::id::*;
use rust_i18n::t;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::RwLock;
use ui_gtk::dialogdatadistributor::DialogDataDistributor;
use ui_gtk::GtkGuiBuilder;
use ui_gtk::GtkObjectsType;
use webkit2gtk::TLSErrorsPolicy;
use webkit2gtk::WebContext;
use webkit2gtk::WebContextExt;
use webkit2gtk::WebView;
use webkit2gtk::WebsiteDataManager;

const TOOLBAR_ICON_SIZE: i32 = 24;
const TOOLBAR_BORDER_WIDTH: u32 = 0;
const TOOLBAR_MARGIN: i32 = 0;

thread_local!(
    pub static GLOB_CACHE: RefCell<GuiCacheValues> = RefCell::new(GuiCacheValues::default());
);

#[derive(Default)]
pub struct GuiCacheValues {
    pane0x: i32,
    pane1x: i32,
    col0w: i32,
    window_width: i32,
    window_height: i32,
}

#[derive(Default)]
pub struct GtkObjectTree {
    pub initvalues: HashMap<PropDef, String>,
}

///  this runs in the gtk thread
impl GtkGuiBuilder for GtkObjectTree {
    fn build_gtk(
        &self,
        gui_event_sender: Sender<GuiEvents>,
        gtk_obj_a: GtkObjectsType,
        ddd: &mut DialogDataDistributor,
    ) {
        const FRAME_RESIZE: bool = true; // should this child expand when the paned widget is resized.
        const FRAME_SHRINK: bool = true; // can this child be made smaller than its requisition.
        const NONE_ADJ: Option<&Adjustment> = None;
        let window: gtk::Window = (*gtk_obj_a).read().unwrap().get_window().unwrap();
        let esw = EvSenderWrapper(gui_event_sender.clone());
        crate::load_css::load_css();
        window.connect_size_allocate(move |_win, rectangle| {
            let n_w: i32 = (*rectangle).width();
            let n_h: i32 = (*rectangle).height();
            let (last_w, last_h) = GLOB_CACHE.with(|glob| {
                (
                    (*glob.borrow()).window_width,
                    (*glob.borrow()).window_height,
                )
            });
            if n_w != last_w || n_h != last_h {
                GLOB_CACHE.with(|glob| {
                    (*glob.borrow_mut()).window_width = n_w;
                    (*glob.borrow_mut()).window_height = n_h;
                });
                esw.sendw(GuiEvents::WindowSizeChanged(n_w, n_h));
            }
        });
        create_dialogs(gui_event_sender.clone(), gtk_obj_a.clone(), ddd);
        let drag_state = Rc::new(RwLock::new(DragState::default()));
        let paned_top = Paned::new(Orientation::Horizontal);
        paned_top.set_wide_handle(true);
        window.add(&paned_top);
        let box_1_v = gtk::Box::new(Orientation::Vertical, 0);
        paned_top.pack1(&box_1_v, false, false);
        paned_top.set_size_request(20, -1);
        let esw = EvSenderWrapper(gui_event_sender.clone());
        paned_top.connect_leave_notify_event(move |paned_top: &Paned, _a2| {
            let newpos: i32 = paned_top.position();
            if newpos != GLOB_CACHE.with(|glob| (*glob.borrow()).pane1x) {
                GLOB_CACHE.with(|glob| {
                    (*glob.borrow_mut()).pane1x = newpos;
                });
                esw.sendw(GuiEvents::PanedMoved(1, newpos));
            }
            gtk::Inhibit(false)
        });

        let mode_debug = self.get_bool(PropDef::AppModeDebug);

        let p2p = self.get_int(PropDef::GuiPane2Pos, 120) as i32;
        paned_top.set_position(p2p);
        let box_2_h = gtk::Box::new(Orientation::Horizontal, 0);
        box_1_v.add(&box_2_h);

        let menubar = create_menubar(gui_event_sender.clone(), gtk_obj_a.clone(), mode_debug);
        box_2_h.pack_start(&menubar, false, false, 0);
        let toolbar = create_toolbar(gui_event_sender.clone(), gtk_obj_a.clone());
        box_2_h.add(&toolbar);
        box_2_h.set_spacing(0);

        let paned_1 = Paned::new(Orientation::Horizontal);
        paned_1.set_size_request(100, -1);
        paned_1.set_wide_handle(true);
        box_1_v.add(&paned_1);
        let scrolledwindow_0 = ScrolledWindow::new(NONE_ADJ, NONE_ADJ);
        scrolledwindow_0.set_widget_name("scrolledwindow_0");
        scrolledwindow_0.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic); // scrollbar-h, scrollbar-v
        scrolledwindow_0.set_vexpand(true);
        scrolledwindow_0.set_shadow_type(ShadowType::EtchedIn);
        scrolledwindow_0.set_min_content_width(50);
        let sourcetree = crate::treeview2::create_treeview(
            gui_event_sender.clone(),
            drag_state,
            gtk_obj_a.clone(),
        );
        scrolledwindow_0.add(&sourcetree);
        paned_1.pack1(&scrolledwindow_0, false, FRAME_SHRINK);
        paned_1.set_resize_mode(ResizeMode::Queue); // is it needed ?
        let esw = EvSenderWrapper(gui_event_sender.clone());
        paned_1.connect_position_notify(move |paned| {
            let newpos: i32 = paned.position();
            // debug!("paned1: pos {}", newpos);
            if newpos != GLOB_CACHE.with(|glob| (*glob.borrow()).pane0x) {
                GLOB_CACHE.with(|glob| {
                    (*glob.borrow_mut()).pane0x = newpos;
                });
                esw.sendw(GuiEvents::PanedMoved(0, newpos));
            }
        });
        //         paned_1.connect_position_set_notify(|p| {            debug!("paned1: pos_set {}", p.position());        });
        let p1p = self.get_int(PropDef::GuiPane1Pos, 90) as i32;
        paned_1.set_position(p1p);
        let col1width = self.get_int(PropDef::GuiCol1Width, 77) as i32;
        let sort_col = self.get_int(PropDef::GuiList0SortColumn, 0);
        let sort_asc = self.get_bool(PropDef::GuiList0SortAscending);
        let content_treeview2 = create_listview(
            gui_event_sender.clone(),
            col1width,
            gtk_obj_a.clone(),
            sort_col as i32,
            sort_asc,
        );

        let scrolledwindow_1 = ScrolledWindow::new(NONE_ADJ, NONE_ADJ);
        scrolledwindow_1.set_widget_name("scrolledwindow_1");
        scrolledwindow_1.add(&content_treeview2);
        scrolledwindow_1.set_shadow_type(gtk::ShadowType::EtchedIn);
        scrolledwindow_1.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic); // scrollbar-h, scrollbar-v
        scrolledwindow_1.set_vexpand(true);
        paned_1.pack2(&scrolledwindow_1, FRAME_RESIZE, FRAME_SHRINK);
        let content_tab_widget = self.create_content_tabs_2(gtk_obj_a.clone());
        paned_top.pack2(&content_tab_widget, true, true); // resize  , shrink: yes

        // box_1_v
        let box3_status = gtk::Box::new(Orientation::Horizontal, 0);
        box_1_v.add(&box3_status);
        let label_st1 = Label::new(Some("|____|"));
        label_st1.set_width_request(20);
        box3_status.add(&label_st1);
        label_st1.set_tooltip_text(Some("Hello"));

        let label_st2 = Label::new(Some(">some_url<"));
        label_st2.set_width_request(100);

        label_st2.connect_label_notify(move |label| {
            let l2txt = label.text().to_string();
            debug!("LABEL2   NOTIFY changing of contained text ",);

            label.set_tooltip_text(Some(l2txt.as_str()));
            //             label_st2.set_tooltip_text(Some("LABEL2!!!"));
        });
        label_st2.connect_button_press_event(|label2: &Label, evb: &EventButton| {
            debug!(
                "LABEL2   connect_button_press_event  {:?}   button={:?}",
                label2.text(),
                evb.button()
            );
            gtk::Inhibit(false)
        });

        label_st2.connect_focus(|_l2: &Label, _dirtype| {
            debug!("LABEL2 focus ");
            gtk::Inhibit(false)
        });

        label_st2.connect_focus_on_click_notify(|_l2: &Label| {
            debug!("LABEL2   focus_on_click_notify ");
            // gtk::Inhibit(false)
        });

        let layout_st = gtk::Layout::new(NONE_ADJ, NONE_ADJ);
        layout_st.add(&label_st2);
        layout_st.set_width(100);
        layout_st.set_vexpand(false);
        layout_st.set_hexpand(true);
        box3_status.add(&layout_st);
        {
            let mut ret = (*gtk_obj_a).write().unwrap();
            ret.set_label(LABEL_STATUS_1, &label_st1);
            ret.set_label(LABEL_STATUS_2, &label_st2);
            ret.set_paned(PANED_1_LEFT, &paned_1);
            ret.set_scrolledwindow(SCROLLEDWINDOW_0, &scrolledwindow_0);
            ret.set_scrolledwindow(SCROLLEDWINDOW_1, &scrolledwindow_1);
        }
        connect_keyboard(gui_event_sender, gtk_obj_a.clone());
    }
}

impl GtkObjectTree {
    fn get_int(&self, name: PropDef, defaul: usize) -> usize {
        if self.initvalues.is_empty() {
            error!("GtkObjectTree: gui_values not present.   {:?}", &name);
            return defaul;
        }
        match self.initvalues.get(&name) {
            Some(s) => match s.parse::<usize>() {
                Ok(i) => i,
                Err(_e) => {
                    warn!(
                        "GtkObjectTree: using default {} for {}",
                        defaul,
                        name.tostring()
                    );
                    defaul
                }
            },
            None => defaul,
        }
    }

    fn get_bool(&self, name: PropDef) -> bool {
        if self.initvalues.is_empty() {
            return false;
        }
        match self.initvalues.get(&name) {
            Some(b) => match b.parse::<bool>() {
                Ok(i) => i,
                Err(_e) => false,
            },
            None => false,
        }
    }

    // Later1:  Stack + Stackswitcher
    // Later2:  Set font size
    //
    // fn set_preferred_languages(&self, languages: &[&str])
    // fn set_spell_checking_enabled(&self, enabled: bool)
    fn create_content_tabs_2(&self, gtk_obj_a: GtkObjectsType) -> Container {
        let box1_v = gtk::Box::new(Orientation::Vertical, 0);
        let linkbutton1 = gtk::LinkButton::new("--");
        linkbutton1.set_label("--");
        linkbutton1.set_halign(Align::Start);
        // linkbutton1.set_(true);
        box1_v.pack_start(&linkbutton1, false, false, 0);

        let box3_h = gtk::Box::new(Orientation::Horizontal, 0);
        box3_h.set_height_request(TAB_MARKER_HEIGHT as i32);
        box3_h.set_widget_name("box_1");
        box1_v.pack_start(&box3_h, false, false, 1);

        let box2_h = gtk::Box::new(Orientation::Horizontal, 0);
        box1_v.pack_start(&box2_h, false, false, 1);
        let label_author = Label::new(Some("-"));
        label_author.set_halign(Align::Start);
        label_author.set_wrap(true);
        label_author.set_line_wrap_mode(WrapMode::Word);
        box2_h.pack_start(&label_author, false, false, 5);

        let label_date = Label::new(Some("-"));
        label_date.set_halign(Align::Center);
        box2_h.pack_start(&label_date, false, false, 5);
        let label_cat = Label::new(Some("-"));
        label_cat.set_halign(Align::End);
        label_cat.set_wrap(true);
        label_cat.set_line_wrap_mode(WrapMode::Word);
        box2_h.pack_end(&label_cat, false, false, 5);

        let wconte: WebContext;
        if let Some(browserdir) = self.initvalues.get(&PropDef::BrowserDir) {
            let wk_dm = WebsiteDataManager::builder()
                .base_cache_directory(browserdir)
                .base_data_directory(browserdir)
                .disk_cache_directory(browserdir)
                .hsts_cache_directory(browserdir)
                .indexeddb_directory(browserdir)
                .local_storage_directory(browserdir)
                .build();
            wconte = WebContext::with_website_data_manager(&wk_dm);
            wconte.set_favicon_database_directory(Some(browserdir));
        } else {
            error!("build_gtk BrowserDir missing!");
            wconte = WebContext::default().unwrap();
        }

        wconte.set_spell_checking_enabled(false);
        wconte.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
        let webview1: WebView = WebView::with_context(&wconte);
        webview1.set_widget_name("webview_0");
        webview1.set_border_width(4);

        box1_v.pack_start(&webview1, true, true, 0);
        {
            let mut ret = (*gtk_obj_a).write().unwrap();
            ret.add_web_context(&wconte);
            ret.add_web_view(&webview1);
            ret.set_label(LABEL_BROWSER_MSG_DATE, &label_date);
            ret.set_label(LABEL_BROWSER_MSG_AUTHOR, &label_author);
            ret.set_label(LABEL_BROWSER_MSG_CATEGORIES, &label_cat);
            ret.set_linkbutton(LINKBUTTON_BROWSER_TITLE, &linkbutton1);
            ret.set_box(BOX_CONTAINER_4_BROWSER, &box1_v);
            ret.set_box(BOX_CONTAINER_3_MARK, &box3_h);
        }
        box1_v.upcast()
    }
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

fn create_listview(
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
    content_tree_view.set_widget_name("TREEVIEW2");
    content_tree_view.set_margin_top(2);
    let liststoretypes = &[
        Pixbuf::static_type(), // 0: feed icon
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
        content_tree_view.append_column(&col);
    }
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        col.pack_start(&cellrendtext, true);
        col.add_attribute(&cellrendtext, TYPESTRING_TEXT, 1);
        col.set_visible(true);
        col.set_title("Title");
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
            if new_width != GLOB_CACHE.with(|glob| (*glob.borrow()).col0w) {
                GLOB_CACHE.with(|glob| {
                    (*glob.borrow_mut()).col0w = new_width;
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
        col.set_title("Date");
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
        let sel = tree_view.selection();
        if tree_view.model().is_none() {
            return;
        }
        if sel.count_selected_rows() != 1 {
            return; // either no row selected, then it's a application focus setting, or too many - then it's a selection range
        }
        let t_model: TreeModel = tree_view.model().unwrap();
        let mut list_pos = -1;
        let mut o_t_iter: Option<TreeIter> = None;
        let (o_tpath, o_tree_view_column) = tree_view.cursor();
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
        let mut sort_col_id = -1;
        if let Some(tvc) = o_tree_view_column {
            sort_col_id = tvc.sort_column_id();
        }
        if list_pos >= 0 && sort_col_id != LIST0_COL_ISREAD {
            esw.sendw(GuiEvents::ListRowActivated(0, list_pos, repo_id));
        }
    });
    content_tree_view
        .selection()
        .connect_changed(move |t_selection| {
            let n_rows = t_selection.count_selected_rows();
            if n_rows <= 0 {
                return;
            }
            if false {
                let (tp_list, t_model) = t_selection.selected_rows();
                for t_path in tp_list {
                    if let Some(t_iter) = t_model.iter(&t_path) {
                        let repo_id = t_model
                            .value(&t_iter, LIST0_COL_MSG_ID as i32)
                            .get::<u32>()
                            .unwrap() as i32;
                        trace!("LIST changed multiple {}  ", repo_id);
                    }
                }
            }
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
        // debug!(            "ROW_ACTIVaTED, double click repoid: {} {}",            repo_id, list_pos        );
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
                        if tvc.sort_column_id() == LIST0_COL_ISREAD {
                            esw.sendw(GuiEvents::ListCellClicked(
                                0,
                                list_pos,
                                tvc.sort_column_id(),
                                repo_id,
                            ));
                        }
                    }
                }
            }
            if button_num == MOUSE_BUTTON_RIGHT {
                let (tp_list, t_model) = treeview.selection().selected_rows();
                let mut repoid_listpos: Vec<(i32, i32)> = Vec::default();
                //            let mut list_positions: Vec<i32> = Vec::default();
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
                // trace!("LIST RightMouse repo_listpos={:?}  ", &repoid_listpos);
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

fn connect_keyboard(g_ev_se: Sender<GuiEvents>, gtk_obj_a: GtkObjectsType) {
    let esw = EvSenderWrapper(g_ev_se);

    if let Some(win) = (*gtk_obj_a).read().unwrap().get_window() {
        win.connect_key_press_event(move |_win, key| {
            let keyval = key.keyval();
            let _keystate = key.state();
            esw.sendw(GuiEvents::KeyPressed(*keyval as isize, keyval.to_unicode()));
            Inhibit(false)
        });
    }
}

// MenuBar
//   MenuItem
//     Menu
//       MenuItem
pub fn create_menubar(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    mode_debug: bool,
) -> MenuBar {
    let icons_dialog: Dialog = (*gtk_obj_a)
        .read()
        .unwrap()
        .get_dialog(DIALOG_ICONS)
        .unwrap()
        .clone();
    let menubar = MenuBar::new();
    menubar.set_border_width(TOOLBAR_BORDER_WIDTH);
    menubar.set_margin(TOOLBAR_MARGIN);
    {
        let m_file_item = MenuItem::with_label(&t!("M_FILE"));
        m_file_item.set_widget_name("M_FILE");
        menubar.append(&m_file_item);
        let menu_file = Menu::new();
        m_file_item.set_submenu(Some(&menu_file));
        let m_import_opml = MenuItem::with_label(&t!("M_IMPORT_OPML"));
        m_import_opml.set_widget_name("M_IMPORT_OPML");
        menu_file.add(&m_import_opml);
        let gtk_obj_a1 = gtk_obj_a.clone();
        m_import_opml.connect_activate(move |_m| {
            let opml_import_dialog: Dialog = (*gtk_obj_a1)
                .read()
                .unwrap()
                .get_dialog(DIALOG_OPML_IMPORT)
                .unwrap()
                .clone();
            opml_import_dialog.show();
        });
        let m_import_opml = MenuItem::with_label(&t!("M_EXPORT_OPML"));
        m_import_opml.set_widget_name("M_EXPORT_OPML");
        menu_file.add(&m_import_opml);
        let gtk_obj_a2 = gtk_obj_a.clone();
        m_import_opml.connect_activate(move |_m| {
            let opml_export_dialog: Dialog = (*gtk_obj_a2)
                .read()
                .unwrap()
                .get_dialog(DIALOG_OPML_EXPORT)
                .unwrap()
                .clone();
            opml_export_dialog.show();
        });
        let m_file_quit = MenuItem::with_label(&t!("M_FILE_QUIT"));
        m_file_quit.set_widget_name("M_FILE_QUIT");
        menu_file.add(&m_file_quit);
        let se = g_ev_se.clone();
        m_file_quit.connect_activate(move |_m| {
            se.send(GuiEvents::MenuActivate(_m.widget_name().to_string()))
                .unwrap();
        });
    }
    {
        let m_item = MenuItem::with_label(&t!("M_OPTIONS"));
        m_item.set_widget_name("M_OPTIONS");
        menubar.append(&m_item);
        let menu_file = Menu::new();
        m_item.set_submenu(Some(&menu_file));
        let m_settings = MenuItem::with_label(&t!("M_SETTINGS"));
        m_settings.set_widget_name("M_SETTINGS");
        menu_file.add(&m_settings);
        let se = g_ev_se.clone();
        m_settings.connect_activate(move |_m| {
            se.send(GuiEvents::MenuActivate(_m.widget_name().to_string()))
                .unwrap();
        });
    }
    {
        let m_item = MenuItem::with_label(&t!("M_HELP"));
        m_item.set_widget_name("M_HELP");
        menubar.append(&m_item);
        let menu = Menu::new();
        m_item.set_submenu(Some(&menu));
        {
            let m_about = MenuItem::with_label(&t!("M_ABOUT"));
            m_about.set_widget_name("M_ABOUT");
            menu.add(&m_about);
            let esw = EvSenderWrapper(g_ev_se);
            m_about.connect_activate(move |_m| {
                esw.sendw(GuiEvents::MenuActivate(_m.widget_name().to_string()));
            });
        }
        if mode_debug {
            let m_icons = MenuItem::with_label(&t!("M_ICONS"));
            m_icons.set_widget_name("M_ICONS");
            menu.add(&m_icons);
            let icons_d = icons_dialog;
            m_icons.connect_activate(move |_m| {
                icons_d.show_all();
            });
        }
    }
    menubar
}

pub fn create_toolbar(g_ev_se: Sender<GuiEvents>, gtk_obj_a: GtkObjectsType) -> Toolbar {
    let toolbar = Toolbar::new();
    toolbar.set_height_request(16);
    toolbar.set_icon_size(IconSize::SmallToolbar);
    toolbar.set_margin(TOOLBAR_MARGIN);
    toolbar.set_border_width(TOOLBAR_BORDER_WIDTH);
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_20_FOLDER_NEW_48,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let button1: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_ADD_FOLDER"))
            .build();
        let new_folder_d: Dialog = (*gtk_obj_a)
            .read()
            .unwrap()
            .get_dialog(DIALOG_NEW_FOLDER)
            .unwrap()
            .clone();
        button1.connect_clicked(move |_b| {
            new_folder_d.show_all();
        });
        toolbar.insert(&button1, -1);
    }
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_10_RSS_ADD_32,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );

        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_ADD_FEED"))
            .build();
        toolbar.insert(&but, -1);
        let new_feedsource_dialog: Dialog = (*gtk_obj_a)
            .read()
            .unwrap()
            .get_dialog(DIALOG_NEW_FEED_SOURCE)
            .unwrap()
            .clone();
        but.connect_clicked(move |_b| {
            new_feedsource_dialog.show_all();
        });
    }
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_18_RELOAD_32,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_REFRESH_ALL"))
            .build();
        toolbar.insert(&but, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        but.connect_clicked(move |_b| {
            esw.sendw(GuiEvents::ToolBarButton("reload-feeds-all".to_string()));
        });
    }
    if false {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_34_DATA_XP2,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text("troubleshooting pane move")
            .build();
        toolbar.insert(&but, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        but.connect_clicked(move |_b| {
            esw.sendw(GuiEvents::ToolBarButton(
                "toolbutton-troubleshoot1".to_string(),
            ));
        });
    }
    if false {
        let ttb1: ToggleToolButton = ToggleToolButtonBuilder::new().label("Special1").build();
        toolbar.insert(&ttb1, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        ttb1.connect_active_notify(move |sw| {
            esw.sendw(GuiEvents::ToolBarToggle(
                "special1".to_string(),
                sw.is_active(),
            ));
        });
    }
    if false {
        let ttb2: ToggleToolButton = ToggleToolButtonBuilder::new().label("Special2").build();
        toolbar.insert(&ttb2, -1);
        let esw = EvSenderWrapper(g_ev_se);
        ttb2.connect_active_notify(move |sw| {
            esw.sendw(GuiEvents::ToolBarToggle(
                "special2".to_string(),
                sw.is_active(),
            ));
        });
    }
    toolbar
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
    /*		later
        let mi_delete = MenuItem::with_label(&t!("CM_MSG_DELETE"));
        let esc = EvSenderCache(
            g_ev_se.clone(),
            GuiEvents::ListSelectedAction(0, "messages-delete".to_string(), repoid_listpos.clone()),
        );
        mi_delete.connect_activate(move |_menuiten| {
            esc.send();
        });
    */
    let mi_copy_link = MenuItem::with_label(&t!("CM_MSG_COPY_LINK_CLIPBOARD"));
    if repoid_listpos.len() == 1 {
        let esc = EvSenderCache(
            g_ev_se,
            GuiEvents::ListSelectedAction(0, "copy-link".to_string(), repoid_listpos.clone()),
        );
        mi_copy_link.connect_activate(move |_menuiten| {
            esc.send();
        });
    }

    let menu: gtk::Menu = Menu::new();
    menu.append(&mi_open_browser);
    if repoid_listpos.len() == 1 {
        menu.append(&mi_copy_link);
    }
    menu.append(&mi_mark_read);
    menu.append(&mi_mark_unread);
    //    menu.append(&mi_delete);		// later
    menu.show_all();
    let c_ev_time = gtk::current_event_time();
    menu.popup_easy(ev_button, c_ev_time);
}

pub fn create_buttonbox(_g_ev_se: Sender<GuiEvents>) -> ButtonBox {
    let buttonbox = ButtonBox::new(Orientation::Horizontal);
    let button1: Button = Button::with_label("button1");
    buttonbox.add(&button1);
    buttonbox
}

// ---
