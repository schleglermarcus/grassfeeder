use glib::StaticType;
use gtk::prelude::CellLayoutExt;
use gtk::prelude::TreeSelectionExt;
use gtk::prelude::TreeViewColumnExt;
use gtk::prelude::TreeViewExt;
use gtk::prelude::WidgetExt;
use gtk::CellRendererText;
use gtk::ListStore;
use gtk::TreeView;
use gtk::TreeViewColumn;
use resources::id::*;
use rust_i18n;
use rust_i18n::t;
use ui_gtk::GtkObjectsType;

const TYPESTRING_TEXT: &str = "text";

pub fn create_statistic_listview(gtk_obj_a: GtkObjectsType) -> TreeView {
    let err_view = TreeView::new();
    err_view.set_headers_visible(true);
    err_view.set_tooltip_column(6);
    err_view.selection().set_mode(gtk::SelectionMode::Multiple);
    err_view.set_activate_on_single_click(false);
    err_view.set_enable_search(false);
    err_view.set_widget_name("msg_list");
    err_view.set_margin_top(2);
    let liststoretypes = &[
        gtk::glib::Type::STRING, // 0 DateTime
        gtk::glib::Type::STRING, // 1 src - message
        u32::static_type(),      // 2 Value
        gtk::glib::Type::STRING, // 3 remote address
        gtk::glib::Type::STRING, // 4 detail message
        bool::static_type(),     // 5 not yet used
        gtk::glib::Type::STRING, // 6 Toolip
    ];
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        CellLayoutExt::pack_start(&col, &cellrendtext, true);
        CellLayoutExt::add_attribute(&col, &cellrendtext, TYPESTRING_TEXT, 0);
        col.set_visible(true);
        col.set_title(&t!("ERRORSLIST_TITLE0"));
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        col.set_sort_column_id(LIST1_COL_TIMESTAMP);
        col.set_sort_indicator(true);
        err_view.append_column(&col);
    }
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        CellLayoutExt::pack_start(&col, &cellrendtext, true);
        CellLayoutExt::add_attribute(&col, &cellrendtext, TYPESTRING_TEXT, 1);
        col.set_visible(true);
        col.set_title(&t!("ERRORSLIST_TITLE1"));
        col.set_resizable(true);
        col.set_sort_column_id(LIST1_COL_SRC);
        col.set_sort_indicator(true);
        col.set_resizable(true);
        err_view.append_column(&col);
    }
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        CellLayoutExt::pack_end(&col, &cellrendtext, false);
        CellLayoutExt::add_attribute(&col, &cellrendtext, TYPESTRING_TEXT, 2);
        col.set_visible(true);
        col.set_title(&t!("ERRORSLIST_TITLE2"));
        col.set_resizable(true);
        col.set_sort_column_id(LIST1_COL_VALUE);
        col.set_sort_indicator(true);
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        err_view.append_column(&col);
    }
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        CellLayoutExt::pack_end(&col, &cellrendtext, false);
        CellLayoutExt::add_attribute(&col, &cellrendtext, TYPESTRING_TEXT, 3);
        col.set_visible(true);
        col.set_title(&t!("ERRORSLIST_TITLE3"));
        col.set_resizable(true);
        col.set_sort_column_id(LIST1_COL_REMOTEADDR);
        col.set_sort_indicator(true);
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        err_view.append_column(&col);
    }
    {
        let cellrendtext = CellRendererText::new();
        let col = TreeViewColumn::new();
        CellLayoutExt::pack_end(&col, &cellrendtext, false);
        CellLayoutExt::add_attribute(&col, &cellrendtext, TYPESTRING_TEXT, 4);
        col.set_visible(true);
        col.set_title(&t!("ERRORSLIST_TITLE4"));
        col.set_resizable(true);
        col.set_sort_column_id(LIST1_COL_DETAIL);
        col.set_sort_indicator(true);
        col.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        err_view.append_column(&col);
    }

    let list_store = ListStore::new(liststoretypes);
    err_view.set_model(Some(&list_store));

    /*
       err_view.connect_selection_notify_event(|_tv, ev_sel| {
           debug!("LIST  _selection_notify_event  {:?}", ev_sel);
           Inhibit(false)
       });

       let esw = EvSenderWrapper(g_ev_se.clone());
       err_view.connect_row_activated(move |t_view, t_path, _tv_column| {
           let t_model = t_view.model().unwrap();
           let t_iter = t_model.iter(t_path).unwrap();
           let repo_id = t_model
               .value(&t_iter, LIST0_COL_MSG_ID as i32)
               .get::<u32>()
               .unwrap() as i32;
           let list_pos = t_path.indices()[0];
           trace!(            "row_activated, double click repoid: {} {}",            repo_id,            list_pos        );
           // esw.sendw(GuiEvents::ListRowDoubleClicked(0, list_pos, repo_id));
       });
    */

    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_tree_view(LISTVIEW1, &err_view);
    ret.set_list_store(LISTVIEW1, &list_store);
    ret.set_list_store_max_columns(LISTVIEW1 as usize, liststoretypes.len() as u8);
    err_view
}
