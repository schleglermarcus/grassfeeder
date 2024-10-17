#[cfg(feature = "legacy3gtk14")]
use gtk::NotebookBuilder;

#[cfg(not(feature = "legacy3gtk14"))]
use gtk::builders::NotebookBuilder;

use crate::statistics_list::create_statistic_listview;
use crate::util::*;
use flume::Sender;
use gtk::gdk_pixbuf::InterpType;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::traits::TextBufferExt;
use gtk::AboutDialog;
use gtk::Adjustment;
use gtk::Align;
use gtk::ComboBoxText;
use gtk::Dialog;
use gtk::Entry;
use gtk::FileChooserAction;
use gtk::FileChooserDialog;
use gtk::Grid;
use gtk::Image;
use gtk::Label;
use gtk::LevelBar;
use gtk::LevelBarMode;
use gtk::Notebook;
use gtk::Orientation;
use gtk::PositionType;
use gtk::ResponseType;
use gtk::Scale;
use gtk::ScrolledWindow;
use gtk::ShadowType;
use gtk::SpinButton;
use gtk::Spinner;
use gtk::Switch;
use gtk::TextBuffer;
use gtk::TextTagTable;
use gtk::TextView;
use gtk::Window;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiEvents;
use resources::application_id::*;
use resources::gen_icons;
use resources::gen_icons::*;
use resources::id::*;
use resources::parameter::DOWNLOADER_MAX_NUM_THREADS;
use resources::parameter::ICON_SIZE_LIMIT_BYTES;
use resources::parameter::STORE_MESSAGES_PER_SUBSCRIPTION;
use rust_i18n;
use rust_i18n::t;
use ui_gtk::dialogdatadistributor::DialogDataDistributor;
use ui_gtk::iconloader::IconLoader;
use ui_gtk::GtkObjectsType;

const FONTSIZE_MIN: f64 = 5.0;
const FONTSIZE_MAX: f64 = 18.0;
const MAX_LENGTH_NEW_SOURCE_NAME: i32 = 50;
const MAX_LENGTH_NEW_SOURCE_URL: i32 = 200;
const GRID_SPACING: u32 = 5;
const DB_CLEAN_STEPS_MAX: f64 = 10.0;
const ICON_RESCALE_SIZE: i32 = 24;
const ICON_DIALOG_COLUMNS: i32 = 40;

const NONE_ADJ: Option<&Adjustment> = None;
const NONE_TEXT: Option<&TextTagTable> = None;

pub fn create_dialogs(
    gui_event_sender: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) {
    create_icons_dialog(gtk_obj_a.clone(), ddd);
    create_new_folder_dialog(gui_event_sender.clone(), gtk_obj_a.clone());
    create_new_subscription_dialog(gui_event_sender.clone(), gtk_obj_a.clone(), ddd);
    create_subscription_delete_dialog(gui_event_sender.clone(), gtk_obj_a.clone(), ddd);
    create_subscription_edit_dialog(gui_event_sender.clone(), gtk_obj_a.clone(), ddd);
    create_folder_edit_dialog(gui_event_sender.clone(), gtk_obj_a.clone(), ddd);
    create_settings_dialog(gui_event_sender.clone(), gtk_obj_a.clone(), ddd);
    create_opml_import_dialog(gui_event_sender.clone(), gtk_obj_a.clone());
    create_opml_export_dialog(gui_event_sender.clone(), gtk_obj_a.clone());
    create_about_dialog(gtk_obj_a.clone(), ddd);
    create_subscription_statistic_dialog(gtk_obj_a.clone(), ddd);
}

fn create_icons_dialog(gtk_obj_a: GtkObjectsType, ddd: &mut DialogDataDistributor) {
    let dialog = Dialog::with_buttons::<Window>(
        Some("Icons Display"),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[("Ok", ResponseType::Ok)],
    );
    let grid = Grid::new();
    grid.set_vexpand(true);
    grid.set_hexpand(true);
    grid.set_column_spacing(2);
    grid.set_row_spacing(2);
    dialog.content_area().add(&grid);
    dialog.set_default_response(ResponseType::Ok);
    dialog.connect_response(move |dialog, rt| {
        match rt {
            ResponseType::Ok => {}
            ResponseType::DeleteEvent => {}
            _ => warn!("icons_dialog:response unexpected {}", rt),
        }
        dialog.hide();
    });

    ddd.set_dialog_distribute(DIALOG_ICONS, move |dialogdata| {
        const DD_ROWS: usize = 3;
        let mut ddnum = 0;
        let mut icon_id: i32 = -1;
        let mut ic_str = String::default();
        let mut sub_id_str = String::default();
        let mut grid_index = 0;
        let y_base = 0;
        while let Some(aval) = dialogdata.get(ddnum) {
            match ddnum % DD_ROWS {
                0 => {
                    if let AValue::AI32(i) = aval {
                        icon_id = *i;
                    }
                }
                1 => {
                    if let AValue::AIMG(ref s) = aval {
                        ic_str.clone_from(s);
                    }
                }
                2 => {
                    if let AValue::ASTR(ref s) = aval {
                        sub_id_str.clone_from(s);
                    }
                }
                _ => (),
            };
            ddnum += 1;
            if ddnum % DD_ROWS == 0 {
                //  trace!("GR  i{} len:{} subs:{}", icon_id, ic_str.len(), sub_id_str);
                grid_attach_icon(
                    &grid,
                    &ic_str,
                    &format!("i{} s{} #{}", icon_id, sub_id_str, ic_str.len()),
                    y_base,
                    grid_index,
                );
                ic_str = String::default();
                icon_id = -1;
                sub_id_str.clear();
                grid_index += 1;
            }
        }
    });

    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_ICONS, &dialog);
}

fn grid_attach_icon(grid: &Grid, icon_str: &str, tooltip: &str, y_base: i32, grid_index: i32) {
    if let Ok(image) = prepare_icon(icon_str, ICON_RESCALE_SIZE) {
        image.set_tooltip_text(Some(tooltip));
        let y: i32 = grid_index / ICON_DIALOG_COLUMNS + y_base;
        let x: i32 = grid_index % ICON_DIALOG_COLUMNS;
        grid.attach(&image, x, y, 1, 1);
    }
}

fn prepare_icon(icon_str: &str, rescale_size: i32) -> Result<Image, String> {
    if icon_str.len() > ICON_SIZE_LIMIT_BYTES {
        return Err(format!(
            "Icon too large:  {} > {}",
            icon_str.len(),
            ICON_SIZE_LIMIT_BYTES
        ));
    }
    let buf = IconLoader::decompress_string_to_vec(icon_str, "prepare_icon");
    match IconLoader::vec_to_pixbuf(&buf) {
        Ok(pb) => {
            let pb_scaled = pb
                .scale_simple(rescale_size, rescale_size, InterpType::Bilinear)
                .unwrap();
            Ok(Image::from_pixbuf(Some(&pb_scaled)))
        }
        Err(e) => {
            error!("prepare_icon : {:?}  length:{:?}", e, icon_str.len());
            Err(e.to_string())
        }
    }
}

pub fn create_new_folder_dialog(g_ev_se: Sender<GuiEvents>, gtk_obj_a: GtkObjectsType) {
    let dialog = Dialog::with_buttons::<Window>(
        Some(&t!("D_NEW_FOLDER_TITLE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[
            (&t!("D_BUTTON_OK"), ResponseType::Ok),
            (&t!("D_BUTTON_CANCEL"), ResponseType::Cancel),
        ],
    );
    let entry1 = Entry::new();
    dialog.content_area().add(&entry1);
    entry1.set_activates_default(true);
    let ent_c = entry1;
    dialog.set_default_response(ResponseType::Ok);
    dialog.connect_response(move |dialog, rt| {
        match rt {
            ResponseType::Ok => {
                let e_text: String = ent_c.text().as_str().to_string();
                let payload = vec![AValue::ASTR(e_text)];
                let _r = g_ev_se.send(GuiEvents::DialogData("new-folder".to_string(), payload));
            }
            ResponseType::Cancel | ResponseType::DeleteEvent => {
                dialog.hide();
            }
            _ => {
                warn!("new_folder:response unexpected {}", rt);
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_NEW_FOLDER, &dialog);
}

// Later:  launch this dialog  from controller
pub fn create_new_subscription_dialog(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) {
    let width = 600;
    let icon_size_pixel = 24;
    let dialog = Dialog::with_buttons::<Window>(
        Some(&t!("D_NEW_SUBSCRIPTION_TITLE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[
            (&t!("D_BUTTON_OK"), ResponseType::Ok),
            (&t!("D_BUTTON_CANCEL"), ResponseType::Cancel),
        ],
    );
    dialog.set_width_request(width);
    let box1v = gtk::Box::new(Orientation::Vertical, 1);
    dialog.content_area().add(&box1v);
    let label1 = Label::new(Some(&t!("D_NEW_SUBSCRIPTION_URL")));
    box1v.pack_start(&label1, false, false, 0);
    let entry_url = Entry::new();
    entry_url.set_expand(true);
    entry_url.set_max_length(MAX_LENGTH_NEW_SOURCE_URL);
    box1v.pack_start(&entry_url, true, true, 0);

    let label2 = Label::new(Some(&t!("D_NEW_SUBSCRIPTION_NAME")));
    box1v.pack_start(&label2, false, false, 1);
    let box2h = gtk::Box::new(Orientation::Horizontal, 1);
    let entry_name = Entry::new();
    entry_name.set_expand(true);
    box2h.pack_start(&entry_name, true, true, 1);
    entry_name.set_max_length(MAX_LENGTH_NEW_SOURCE_NAME);

    box2h.set_expand(true);
    let spinner = Spinner::new();
    box2h.pack_end(&spinner, false, false, 0);
    spinner.set_active(false); // shall be invisible at start of dialog
    spinner.stop(); // no animation at  start of dialog
    box1v.pack_start(&box2h, false, false, 1);
    let box3h = gtk::Box::new(Orientation::Horizontal, 1);
    let label3 = Label::new(None);
    box3h.pack_start(&label3, true, true, 1);

    let mut image_icon = Image::new();
    if let Ok(image_icon_prep) = prepare_icon(
        gen_icons::ICON_LIST[gen_icons::IDX_03_ICON_TRANSPARENT_48],
        icon_size_pixel,
    ) {
        image_icon = image_icon_prep;
    }
    box3h.pack_end(&image_icon, false, false, 0);
    box1v.pack_start(&box3h, false, false, 1);
    let ev_se = g_ev_se.clone();
    entry_url.connect_text_notify(move |entry_url| {
        let e_text = entry_url.text().as_str().to_string();
        let _r = ev_se.send(GuiEvents::DialogEditData(
            "feedsource-edit".to_string(),
            AValue::ASTR(e_text),
        ));
    });
    entry_url.set_activates_default(true);
    dialog.set_default_response(ResponseType::Ok);
    let dialog_c = dialog.clone();
    entry_name.connect_text_notify(move |e2| {
        let e2text = e2.text().as_str().to_string();
        let isempty = e2text.trim().is_empty();
        dialog_c.set_response_sensitive(ResponseType::Ok, !isempty);
    });
    entry_name.set_activates_default(true);
    let ent1_c = entry_url.clone();
    let ent2_c = entry_name.clone();
    let ev_se = g_ev_se;
    dialog.connect_response(move |dialog, rt| {
        match rt {
            ResponseType::Ok => {
                let f2txt = ent2_c.text().as_str().to_string();
                let payload = vec![
                    AValue::ASTR(ent1_c.text().as_str().to_string()),
                    AValue::ASTR(f2txt),
                ];
                let _r = ev_se.send(GuiEvents::DialogData("new-feedsource".to_string(), payload));
                ent1_c.buffer().set_text("");
                ent2_c.buffer().set_text("");
            }
            ResponseType::Cancel | ResponseType::DeleteEvent => {
                ent1_c.buffer().set_text("");
                ent2_c.buffer().set_text("");
            }
            _ => {
                warn!("new-subscription:response unexpected {}", rt);
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    let ent2_c = entry_name.clone();
    let label3_c = label3.clone();
    dialog.connect_show(move |dialog| {
        ent2_c.set_text("");
        label3_c.set_text("");
        dialog.set_response_sensitive(ResponseType::Ok, false);
    });
    let entry_name_c = entry_name.clone();
    let image_icon_c = image_icon;
    let label3_c = label3;
    let ent1_c = entry_url.clone();
    ddd.set_dialog_distribute(DIALOG_NEW_SUBSCRIPTION, move |dialogdata| {
        if dialogdata.len() < 2 {
            return; // empty subscription dialog
        }
        if let Some(s) = dialogdata.first().unwrap().str() {
            entry_name_c.set_text(&s); // 0: Display Name
        }
        if let Some(s) = dialogdata.get(1).unwrap().str() {
            label3_c.set_text(&s); // 2: homepage
        }
        if let Some(s) = dialogdata.get(2).unwrap().str() {
            if !s.is_empty() {
                match prepare_icon(&s, icon_size_pixel) {
                    Ok(image_icon) => image_icon_c.set_pixbuf(image_icon.pixbuf().as_ref()),
                    Err(e) => {
                        warn!("Cannot display the icon: {} ", e);
                    }
                }
            }
        }
        let spinner_act = dialogdata.get(3).unwrap().boo();
        spinner.set_active(spinner_act);
        if let Some(s) = dialogdata.get(4).unwrap().str() {
            ent1_c.set_text(&s); // 4: feed-url
        }
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_NEW_SUBSCRIPTION, &dialog);
    ret.set_text_entry(TEXTENTRY_NEWSOURCE_URL, &entry_url);
    ret.set_text_entry(TEXTENTRY_NEWSOURCE_E2, &entry_name);
}

pub fn create_subscription_delete_dialog(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) {
    let width = 400;
    let dialog = Dialog::with_buttons::<Window>(
        Some(&t!("D_DELETE_SUBSCRIPTION_TITLE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[
            (&t!("D_BUTTON_YES"), ResponseType::Yes),
            (&t!("D_BUTTON_NO"), ResponseType::No),
        ],
    );
    dialog.set_width_request(width);
    let box1v = gtk::Box::new(Orientation::Vertical, 0);
    dialog.content_area().add(&box1v);
    let label1 = Label::new(None);
    box1v.pack_start(&label1, false, false, 0);
    let label2 = Label::new(None);
    box1v.pack_start(&label2, false, false, 0);
    let label3 = Label::new(None);
    box1v.pack_start(&label3, false, false, 0);
    dialog.set_default_response(ResponseType::Yes);
    let ev_se = g_ev_se;
    dialog.connect_response(move |dialog, rt| {
        match rt {
            ResponseType::Yes => {
                let _r = ev_se.send(GuiEvents::DialogData(
                    "feedsource-delete".to_string(),
                    Vec::<AValue>::default(),
                ));
            }
            ResponseType::No | ResponseType::DeleteEvent => {
                trace!("fs_delete: response cancel or delete ");
            }
            _ => {
                warn!("fs_delete: response unexpected {}", rt);
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    let label1c = label1;
    let label2c = label2;
    let label3c = label3;
    let dialog_c = dialog.clone();
    ddd.set_dialog_distribute(DIALOG_FS_DELETE, move |dialogdata| {
        let is_folder: bool = dialogdata.first().unwrap().boo();
        if is_folder {
            label1c.set_text(&t!("D_DELETE_FOLDER_QUEST")); // "Delete this folder ?"
            dialog_c.set_title(&t!("D_DELETE_FOLDER_TITLE"));
        } else {
            label1c.set_text(&t!("D_DELETE_SUBSCRIPTION_QUEST")); // "Delete this feed source ?"
            dialog_c.set_title(&t!("D_DELETE_SUBSCRIPTION_TITLE"));
        }
        if let Some(s) = dialogdata.get(1).unwrap().str() {
            label2c.set_text(&s);
        }
        if let Some(s) = dialogdata.get(2).unwrap().str() {
            label3c.set_text(&s);
        }
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_FS_DELETE, &dialog);
}

fn create_subscription_edit_dialog(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) {
    let width = 500;
    let dialog = Dialog::with_buttons::<Window>(
        Some(&t!("D_EDIT_SUBSCRIPTION_TITLE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[
            (&t!("D_BUTTON_OK"), ResponseType::Ok),
            (&t!("D_BUTTON_CANCEL"), ResponseType::Cancel),
        ],
    );
    dialog.set_width_request(width);
    dialog.set_default_response(ResponseType::Ok);
    let notebook: Notebook = NotebookBuilder::new()
        .scrollable(true)
        .show_border(true)
        .show_tabs(true)
        .width_request(50)
        .build();
    dialog.content_area().add(&notebook);
    let grid1 = Grid::new();
    grid1.set_widget_name("subs_edit_grid1");
    grid1.set_vexpand(false);
    grid1.set_hexpand(true);
    grid1.set_column_spacing(3);
    grid1.set_row_spacing(3);
    let label_nb1 = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_TAB1")));
    notebook.append_page(&grid1, Some(&label_nb1));

    let mut line = 0;
    let label1 = Label::new(Some(&t!("D_NEW_SUBSCRIPTION_NAME")));
    grid1.attach(&label1, 0, line, 1, 1);
    let entry1 = Entry::new();
    entry1.set_expand(true);
    entry1.set_hexpand(true);
    entry1.set_vexpand(false);
    entry1.set_activates_default(true);
    entry1.set_max_length(MAX_LENGTH_NEW_SOURCE_NAME);
    grid1.attach(&entry1, 1, line, 1, 1);
    line += 1;

    let label2 = Label::new(Some(&t!("D_NEW_SUBSCRIPTION_URL")));
    grid1.attach(&label2, 0, line, 1, 1);
    let entry2 = Entry::new();
    entry1.set_hexpand(true);
    entry1.set_vexpand(false);
    entry2.set_activates_default(true);
    entry2.set_max_length(MAX_LENGTH_NEW_SOURCE_URL);
    grid1.attach(&entry2, 1, line, 1, 1);
    line += 1;

    let label0 = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_ICON")));
    grid1.attach(&label0, 0, line, 1, 1);
    let empty_image = Image::new();
    process_string_to_image(
        gen_icons::ICON_06_CENTER_POINT_GREEN,
        &empty_image,
        &String::default(),
        DIALOG_ICON_SIZE,
    );
    grid1.attach(&empty_image, 1, line, 1, 1);
    // line += 1;

    let label_nb2 = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_TAB2")));
    let grid2 = Grid::new();
    notebook.append_page(&grid2, Some(&label_nb2));
    grid2.set_vexpand(false);
    grid2.set_hexpand(true);

    let mut line = 0;
    let label1a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_MAIN_WEBSITE")));
    grid2.attach(&label1a, 0, line, 1, 1);
    let label1b = Label::new(Some("_"));
    grid2.attach(&label1b, 1, line, 1, 1);
    line += 1;
    let label2a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_NUM_MESSAGES")));
    grid2.attach(&label2a, 0, line, 1, 1);
    let label2b = Label::new(Some("_"));
    grid2.attach(&label2b, 1, line, 1, 1);
    line += 1;
    let label3a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_NUM_UNREAD")));
    grid2.attach(&label3a, 0, line, 1, 1);
    let label3b = Label::new(Some("_"));
    grid2.attach(&label3b, 1, line, 1, 1);
    line += 1;
    let label4a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_LAST_DOWNLOAD")));
    grid2.attach(&label4a, 0, line, 1, 1);
    let label4b = Label::new(Some("_"));
    grid2.attach(&label4b, 1, line, 1, 1);
    line += 1;
    let label5a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_LAST_CREATION")));
    grid2.attach(&label5a, 0, line, 1, 1);
    let label5b = Label::new(Some("_"));
    grid2.attach(&label5b, 1, line, 1, 1);
    // line += 1;

    let label_nb3 = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_TAB3")));
    let scrolledwindow1 = ScrolledWindow::new(NONE_ADJ, NONE_ADJ);
    scrolledwindow1.set_widget_name("scrolledwindow_0");
    scrolledwindow1.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic); // scrollbar-h, scrollbar-v
    scrolledwindow1.set_vexpand(true);
    scrolledwindow1.set_shadow_type(ShadowType::EtchedIn);

    let textview = TextView::new();
    textview.set_vexpand(true);
    textview.set_monospace(true);
    notebook.append_page(&scrolledwindow1, Some(&label_nb3));
    scrolledwindow1.add(&textview);

    let ev_se = g_ev_se;
    let entry1c = entry1.clone();
    let entry2c = entry2.clone();
    dialog.connect_response(move |dialog, rt| {
        match rt {
            ResponseType::Ok => {
                let av = vec![
                    AValue::ASTR(entry1c.text().to_string()),
                    AValue::ASTR(entry2c.text().to_string()),
                ];
                let _r = ev_se.send(GuiEvents::DialogData(
                    "subscription-edit-ok".to_string(),
                    av,
                ));
            }
            ResponseType::Cancel | ResponseType::DeleteEvent => {
                debug!("feedsource_edit: cancel or delete ");
            }
            _ => {
                warn!("feedsource_edit:response unexpected {}", rt);
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });

    let entry1c = entry1;
    let entry2c = entry2;
    let image_c = empty_image;
    let label1b_c = label1b;
    let label2b_c = label2b;
    let label3b_c = label3b;
    let label4b_c = label4b;
    let label5b_c = label5b;
    let textview_c = textview.clone();
    ddd.set_dialog_distribute(DIALOG_SUBS_EDIT, move |dialogdata| {
        let mut url = String::default();
        if let Some(s) = dialogdata.first().unwrap().str() {
            entry1c.set_text(&s); //   0: displayname
        }
        if let Some(s) = dialogdata.get(1).unwrap().str() {
            entry2c.set_text(&s); //   1: url
            url = s;
        }
        if !process_icon_to_image(dialogdata.get(2), &image_c, &url) {
            process_string_to_image(
                gen_icons::ICON_05_RSS_FEEDS_GREY_64_D,
                &image_c,
                &url,
                DIALOG_ICON_SIZE,
            ); //  2: icon
        }
        if let Some(s) = dialogdata.get(3).unwrap().str() {
            label2b_c.set_text(&s); // 3: num-all
        }
        if let Some(s) = dialogdata.get(4).unwrap().str() {
            label3b_c.set_text(&s); // 4: num-unread
        }
        if let Some(s) = dialogdata.get(5).unwrap().str() {
            label1b_c.set_text(&s); // 5: main website
        }
        if let Some(s) = dialogdata.get(6).unwrap().str() {
            label4b_c.set_text(&s); // update-int
        }
        if let Some(s) = dialogdata.get(7).unwrap().str() {
            label5b_c.set_text(&s); // update-ext
        }
        if let Some(s) = dialogdata.get(8).unwrap().str() {
            let buffer = textview_c.buffer().unwrap(); // error lines
            buffer.set_text(&s);
        }
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_SUBS_EDIT, &dialog);
    ret.set_text_view(DIALOG_TEXTVIEW_ERR, &textview);
}

fn create_folder_edit_dialog(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) {
    let width = 400;
    let dialog = Dialog::with_buttons::<Window>(
        Some(&t!("D_EDIT_FOLDER_NAME")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[
            (&t!("D_BUTTON_OK"), ResponseType::Ok),
            (&t!("D_BUTTON_CANCEL"), ResponseType::Cancel),
        ],
    );
    dialog.set_width_request(width);
    dialog.set_default_response(ResponseType::Ok);
    let notebook: Notebook = NotebookBuilder::new()
        .scrollable(true)
        .show_border(true)
        .show_tabs(true)
        .width_request(50)
        .build();
    dialog.content_area().add(&notebook);
    let grid1 = Grid::new();
    grid1.set_vexpand(false);
    grid1.set_hexpand(true);
    grid1.set_column_spacing(2);
    let label_nb1 = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_TAB1")));
    notebook.append_page(&grid1, Some(&label_nb1));
    let label1 = Label::new(Some(&t!("D_NEW_SUBSCRIPTION_NAME")));
    grid1.attach(&label1, 0, 0, 1, 1);
    let entry1 = Entry::new();
    entry1.set_expand(true);
    entry1.set_activates_default(true);
    grid1.attach(&entry1, 1, 0, 1, 1);
    let box2v = gtk::Box::new(Orientation::Vertical, 0);
    let label_nb2 = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_TAB2")));
    notebook.append_page(&box2v, Some(&label_nb2));
    let ev_se = g_ev_se;
    let entry1c = entry1.clone();
    dialog.connect_response(move |dialog, rt| {
        match rt {
            ResponseType::Ok => {
                let av = vec![AValue::ASTR(entry1c.text().to_string()), AValue::None];
                let _r = ev_se.send(GuiEvents::DialogData("folder-edit".to_string(), av));
            }
            ResponseType::Cancel | ResponseType::DeleteEvent => {
                debug!("folder_edit: cancel or delete ");
            }
            _ => {
                warn!("folder_edit:response unexpected {}", rt);
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    let entry1c = entry1;
    ddd.set_dialog_distribute(DIALOG_FOLDER_EDIT, move |dialogdata| {
        if let Some(s) = dialogdata.first().unwrap().str() {
            entry1c.set_text(&s);
        }
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_FOLDER_EDIT, &dialog);
}

pub fn get_fetch_updater_interval_name(num: i32) -> String {
    match num {
        1 => t!("D_SETTINGS_INTERVAL_01_MINUTES"),
        2 => t!("D_SETTINGS_INTERVAL_02_HOURS"),
        3 => t!("D_SETTINGS_INTERVAL_03_DAYS"),
        _ => String::default(),
    }
}

pub fn get_fetch_interval(name: String) -> i32 {
    println!(
        "M={}   H={}   D={}",
        t!("D_SETTINGS_INTERVAL_01_MINUTES"),
        t!("D_SETTINGS_INTERVAL_02_HOURS"),
        t!("D_SETTINGS_INTERVAL_03_DAYS")
    );

    if t!("D_SETTINGS_INTERVAL_01_MINUTES").cmp(&name) == std::cmp::Ordering::Equal {
        return 1;
    }
    if t!("D_SETTINGS_INTERVAL_02_HOURS").cmp(&name) == std::cmp::Ordering::Equal {
        return 2;
    }
    if t!("D_SETTINGS_INTERVAL_03_DAYS").cmp(&name) == std::cmp::Ordering::Equal {
        return 3;
    }
    0
}

pub fn get_focus_policy_name(num: i32) -> String {
    match num {
        1 => t!("D_SETTINGS_FOCUS_POLICY_NONE"),
        2 => t!("D_SETTINGS_FOCUS_POLICY_LAST_SELECTED"),
        3 => t!("D_SETTINGS_FOCUS_POLICY_NEWEST"),
        4 => t!("D_SETTINGS_FOCUS_POLICY_BEFORE_UNREAD"),
        _ => String::default(),
    }
}

pub fn get_focus_policy(name: String) -> i32 {
    if t!("D_SETTINGS_FOCUS_POLICY_NONE").cmp(&name) == std::cmp::Ordering::Equal {
        return 1;
    }
    if t!("D_SETTINGS_FOCUS_POLICY_LAST_SELECTED").cmp(&name) == std::cmp::Ordering::Equal {
        return 2;
    }
    if t!("D_SETTINGS_FOCUS_POLICY_NEWEST").cmp(&name) == std::cmp::Ordering::Equal {
        return 3;
    }
    if t!("D_SETTINGS_FOCUS_POLICY_BEFORE_UNREAD").cmp(&name) == std::cmp::Ordering::Equal {
        return 4;
    }
    0
}

fn create_settings_dialog(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) {
    let width = 300;
    let dialog = Dialog::with_buttons::<Window>(
        Some(&t!("D_SETTINGS_TITLE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[
            (&t!("D_BUTTON_OK"), ResponseType::Ok),
            (&t!("D_BUTTON_CANCEL"), ResponseType::Cancel),
        ],
    );
    dialog.set_width_request(width);
    dialog.set_default_response(ResponseType::Ok);
    dialog.set_hexpand(true);
    let notebook: Notebook = NotebookBuilder::new()
        .scrollable(true)
        .show_border(true)
        .show_tabs(true)
        .width_request(50)
        .build();
    dialog.content_area().add(&notebook);
    let sw_subs_update_onstart = Switch::new();
    let spinb_source_update = SpinButton::with_range(1.0, 60.0, 1.0);
    let cbt_timescale = ComboBoxText::with_entry();
    let spinb_numthread = SpinButton::with_range(1.0, DOWNLOADER_MAX_NUM_THREADS as f64, 1.0);
    let cbt_focuspolicy = ComboBoxText::with_entry();
    let sw_display_feedcount = Switch::new();
    let spinb_msg_keep_count =
        SpinButton::with_range(20.0, STORE_MESSAGES_PER_SUBSCRIPTION as f64, 20.0);
    let sw_fontsize_manual_enable = Switch::new();
    let spinb_fontsize_manual = SpinButton::with_range(FONTSIZE_MIN, FONTSIZE_MAX, 1.0);
    let scale_bright = Scale::with_range(Orientation::Horizontal, 0.0, 255.0, 1.0);
    let sw_browser_cache_clear = Switch::new();
    {
        let grid1 = Grid::new();
        grid1.set_vexpand(true);
        grid1.set_hexpand(true);
        grid1.set_row_spacing(GRID_SPACING);
        grid1.set_column_spacing(GRID_SPACING);
        grid1.set_margin(4);
        let label_nb1 = Label::new(Some(&t!("D_SETTINGS_TAB1")));
        notebook.append_page(&grid1, Some(&label_nb1));
        let mut line = 0;
        let label1 = Label::new(Some(&t!("D_SETTINGS_UPDATE_ON_START")));
        grid1.attach(&label1, 0, line, 1, 1);
        grid1.attach(&sw_subs_update_onstart, 1, line, 1, 1);
        sw_subs_update_onstart.set_halign(Align::Start);
        line += 1;

        let label2 = Label::new(Some(&t!("D_SETTINGS_UPDATE_UPDATE_AFTER")));
        grid1.attach(&label2, 0, line, 1, 1);
        grid1.attach(&spinb_source_update, 1, line, 1, 1);

        cbt_timescale.append_text(&get_fetch_updater_interval_name(1));
        cbt_timescale.append_text(&get_fetch_updater_interval_name(2));
        cbt_timescale.append_text(&get_fetch_updater_interval_name(3));
        cbt_timescale.set_id_column(0);
        cbt_timescale.set_halign(Align::Start);
        grid1.attach_next_to(
            &cbt_timescale,
            Some(&spinb_source_update),
            PositionType::Right,
            1,
            1,
        );
        line += 1;
        let label2 = Label::new(Some(&t!("D_SETTINGS_UPDATERS_PARALLEL")));
        grid1.attach(&label2, 0, line, 1, 1);
        grid1.attach(&spinb_numthread, 1, line, 1, 1);
        line += 1;

        let label2 = Label::new(Some(&t!("D_SETTINGS_MESSAGE_FOCUS_POLICY")));
        grid1.attach(&label2, 0, line, 1, 1);
        cbt_focuspolicy.append_text(&get_focus_policy_name(1));
        cbt_focuspolicy.append_text(&get_focus_policy_name(2));
        cbt_focuspolicy.append_text(&get_focus_policy_name(3));
        cbt_focuspolicy.append_text(&get_focus_policy_name(4));
        cbt_focuspolicy.set_id_column(0);
        grid1.attach(&cbt_focuspolicy, 1, line, 1, 1);
    }
    let label_nb2 = Label::new(Some(&t!("D_SETTINGS_TAB2")));
    {
        let grid2 = Grid::new();
        notebook.append_page(&grid2, Some(&label_nb2));
        grid2.set_vexpand(true);
        grid2.set_hexpand(true);
        grid2.set_margin(4);
        grid2.set_row_spacing(GRID_SPACING);
        grid2.set_column_spacing(GRID_SPACING);

        let mut line = 0;
        let label2_1 = Label::new(Some(&t!("D_SETTINGS_SHOW_MESSAGE_COUNT")));
        grid2.attach(&label2_1, 0, line, 1, 1);
        grid2.attach(&sw_display_feedcount, 1, line, 1, 1);
        sw_display_feedcount.set_halign(Align::Start);

        line += 1;
        let label2_2 = Label::new(Some(&t!("D_SETTINGS_MESSAGES_KEEP_COUNT")));
        grid2.attach(&label2_2, 0, line, 1, 1);
        grid2.attach(&spinb_msg_keep_count, 1, line, 1, 1);

        line += 1;
        let label2_3 = Label::new(Some(&t!("D_SETTINGS_MANUAL_FONT_SIZE")));
        grid2.attach(&label2_3, 0, line, 1, 1);
        let spinb_fontsize_manual_c = spinb_fontsize_manual.clone();
        sw_fontsize_manual_enable.connect_state_set(move |_sw, state| {
            spinb_fontsize_manual_c.set_sensitive(state);
            gtk::Inhibit(false)
        });
        grid2.attach(&sw_fontsize_manual_enable, 1, line, 1, 1);
        grid2.attach_next_to(
            &spinb_fontsize_manual,
            Some(&sw_fontsize_manual_enable),
            PositionType::Right,
            1,
            1,
        );
        sw_fontsize_manual_enable.set_halign(Align::Start);
        sw_fontsize_manual_enable.set_valign(Align::Center);
        sw_fontsize_manual_enable.set_vexpand(false);
        line += 1;
        let label2_4 = Label::new(Some(&t!("D_SETTINGS_BROWSER_BACKGROUND_BRIGHTNESS")));
        grid2.attach(&label2_4, 0, line, 1, 1);
        grid2.attach(&scale_bright, 1, line, 1, 1);

        line += 1;
        let label2_5 = Label::new(Some(&t!("D_SETTINGS_BROWSER_CACHE_CLEAR")));
        grid2.attach(&label2_5, 0, line, 1, 1);
        grid2.attach(&sw_browser_cache_clear, 1, line, 1, 1);
        sw_browser_cache_clear.set_halign(Align::Start);
        if false {
            line += 1;
            let label2_5 = Label::new(Some(&t!("D_SETTINGS_SYSTRAY_ICON_ENABLE")));
            grid2.attach(&label2_5, 0, line, 1, 1);
        }
    }

    let lb_clean = LevelBar::new();
    let label_nb3 = Label::new(Some(&t!("D_SETTINGS_TAB3")));
    let textview2 = TextView::new();
    let b_checknow = gtk::Button::with_label(&t!("D_SETTINGS_DB_CLEAN_B1"));
    {
        let grid3 = Grid::new();
        notebook.append_page(&grid3, Some(&label_nb3));
        grid3.set_vexpand(true);
        grid3.set_hexpand(true);
        grid3.set_margin(4);
        grid3.set_row_spacing(GRID_SPACING);
        grid3.set_column_spacing(GRID_SPACING);
        let mut line = 0;
        let label1_1 = Label::new(Some(&t!("D_SETTINGS_DB_CLEAN")));
        grid3.attach(&label1_1, 0, line, 1, 1);
        grid3.attach(&b_checknow, 1, line, 1, 1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        b_checknow.connect_clicked(move |_m| {
            esw.sendw(GuiEvents::ButtonClicked("D_SETTINGS_CHECKNOW".to_string()));
        });
        line += 1;
        lb_clean.set_mode(LevelBarMode::Discrete);
        lb_clean.set_min_value(0.0);
        lb_clean.set_max_value(DB_CLEAN_STEPS_MAX);
        lb_clean.set_height_request(16);
        grid3.attach(&lb_clean, 0, line, 2, 1);

        line += 1;
        let scrolledwindow2 = ScrolledWindow::new(NONE_ADJ, NONE_ADJ);
        scrolledwindow2.set_widget_name("scrolledwindow_2");
        scrolledwindow2.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic); // scrollbar-h, scrollbar-v
        scrolledwindow2.set_vexpand(true);
        scrolledwindow2.set_shadow_type(ShadowType::EtchedIn);

        textview2.set_vexpand(true); //        textview2.set_monospace(true);
        textview2.set_hexpand(true);
        scrolledwindow2.add(&textview2);
        grid3.attach(&scrolledwindow2, 0, line, 2, 3);
    }

    let ev_se = g_ev_se;
    let sw_subs_update_onstart_c = sw_subs_update_onstart.clone();
    let spinb_source_update_c = spinb_source_update.clone();
    let cbt_timescale_c: ComboBoxText = cbt_timescale.clone();
    let spinb_numthread_c = spinb_numthread.clone();
    let cbt_focuspolicy_c = cbt_focuspolicy.clone();
    let sw_display_feedcount_c = sw_display_feedcount.clone();
    let spinb_msg_keep_count_c = spinb_msg_keep_count.clone();
    let sw_fontsize_manual_enable_c = sw_fontsize_manual_enable.clone();
    let spinb_fontsize_manual_c = spinb_fontsize_manual.clone();
    let scale_bright_c = scale_bright.clone();
    let sw_browser_cache_clear_c = sw_browser_cache_clear.clone();

    dialog.connect_response(move |dialog, rt| {
        match rt {
            ResponseType::Ok => {
                let mut av = Vec::<AValue>::default();
                av.push(AValue::ABOOL(sw_subs_update_onstart_c.state())); // 0
                av.push(AValue::AI32(spinb_source_update_c.value() as i32)); // 1
                if let Some(fetch_interval_n) = cbt_timescale_c.active() {
                    av.push(AValue::AI32((fetch_interval_n + 1) as i32)); // 2 UpdateFeeds Unit:  1:minutes  2:hours  3:days
                } else {
                    av.push(AValue::AI32(0));
                }
                av.push(AValue::AI32(spinb_numthread_c.value() as i32)); // 3 Web Fetcher Threads
                if let Some(focuspolicy_id) = cbt_focuspolicy_c.active() {
                    av.push(AValue::AI32((focuspolicy_id + 1) as i32)); // 4 Message Focus Policy
                } else {
                    av.push(AValue::AI32(0));
                }
                av.push(AValue::ABOOL(sw_display_feedcount_c.state())); // 5 : DisplayCountOfAllFeeds
                av.push(AValue::AI32(spinb_msg_keep_count_c.value() as i32)); // 6 : Number of Kept messages
                av.push(AValue::ABOOL(sw_fontsize_manual_enable_c.state())); // 7 : ManualFontSizeEnable
                av.push(AValue::AI32(spinb_fontsize_manual_c.value() as i32)); // 8 : ManualFontSizeEnable
                av.push(AValue::AU32(scale_bright_c.value() as u32)); // 9 : Browser BG
                av.push(AValue::ABOOL(sw_browser_cache_clear_c.state())); // 10 : browser cache cleanup
                let _r = ev_se.send(GuiEvents::DialogData("settings".to_string(), av));
            }
            ResponseType::Cancel | ResponseType::DeleteEvent => {
                trace!("settings: cancel or delete ");
            }
            _ => {
                warn!("settings:response unexpected {}", rt);
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });

    ddd.set_dialog_distribute(DIALOG_SETTINGS, move |dialogdata| {
        sw_subs_update_onstart.set_state(dialogdata.first().unwrap().boo()); // 0 : UpdateFeedsOnStart
        let mut interval_cardinal = dd_get_uint(dialogdata, 1, 1); // 1 UpdateFeeds Cardinal
        interval_cardinal = std::cmp::max(1, interval_cardinal);
        let spinbuttonvalue: f64 = interval_cardinal as f64;
        spinb_source_update.set_value(spinbuttonvalue);
        let interval_unit = dd_get_uint(dialogdata, 2, 3); // 2 UpdateFeeds Unit:  1:minutes  2:hours  3:days
        cbt_timescale.set_active(Some(interval_unit - 1));
        let fetcher_threads = dd_get_uint(dialogdata, 3, 1); // 3 Web Fetcher Threads
        spinb_numthread.set_value(fetcher_threads as f64);
        let focus_policy: i32 = dialogdata.get(4).unwrap().int().unwrap(); //dd_get_uint(dialogdata, 4, 1); // 4 Message Focus Policy
        cbt_focuspolicy.set_active(Some((focus_policy - 1) as u32));
        sw_display_feedcount.set_state(dialogdata.get(5).unwrap().boo()); // 5 : DisplayCountOfAllFeeds
        let msg_keep_count = dd_get_uint(dialogdata, 6, 999); // 6 : Number of Kept messages
        spinb_msg_keep_count.set_value(msg_keep_count as f64);
        let fontsize_enable = dialogdata.get(7).unwrap().boo();
        sw_fontsize_manual_enable.set_state(fontsize_enable); // 7 : ManualFontSizeEnable
        spinb_fontsize_manual.set_sensitive(fontsize_enable);
        let fontsize_manual = dd_get_uint(dialogdata, 8, 10); // 8 : ManualFontSizeEnable
        spinb_fontsize_manual.set_value(fontsize_manual as f64);
        let browser_bg = dd_get_uint(dialogdata, 9, 0); // 9 : Browser_bg
        scale_bright.set_value(browser_bg as f64);
        sw_browser_cache_clear.set_state(dialogdata.get(10).unwrap().boo()); // 10 : browser cache cleanup
                                                                             //  sw_subs_db_cleanup.set_state(dialogdata.get(11).unwrap().boo()); // 11 : DB cleanup
    });
    let tview_clean = textview2.clone();
    ddd.set_dialog_distribute(DIALOG_SETTINGS_CHECK, move |dialogdata| {
        if let Some(level) = dialogdata.first().unwrap().uint() {
            if level == 0 {
                let tbuf = TextBuffer::new(NONE_TEXT);
                tview_clean.set_buffer(Some(&tbuf));
            }
            lb_clean.set_value(level as f64);
        }
        if let Some(s_add) = dialogdata.get(1).unwrap().str() {
            if let Some(buf) = tview_clean.buffer() {
                buf.set_text(&s_add);
            }
        }
    });

    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_SETTINGS, &dialog);
    ret.set_text_view(DIALOG_TEXTVIEW_CLEAN, &textview2);
    ret.set_button(BUTTON_SETTINGS_CLEAN_START, &b_checknow);
}

fn create_opml_import_dialog(g_ev_se: Sender<GuiEvents>, gtk_obj_a: GtkObjectsType) {
    let dialog = FileChooserDialog::new(
        Some(&t!("D_IMPORT_OPML_SELECT_FILE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        FileChooserAction::Open,
    );
    dialog.add_buttons(&[("Open", ResponseType::Ok), ("Cancel", ResponseType::Cancel)]);
    dialog.set_select_multiple(false);
    let ev_se = g_ev_se;
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Ok {
            let files = dialog.filenames();
            if !files.is_empty() {
                let payload = vec![AValue::ASTR(
                    files[0].as_path().to_str().unwrap().to_string(),
                )];
                debug!("import-opml  payload={:?}", &payload);

                let _r = ev_se.send(GuiEvents::DialogData("import-opml".to_string(), payload));
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_OPML_IMPORT, &dialog.upcast());
}

fn create_opml_export_dialog(g_ev_se: Sender<GuiEvents>, gtk_obj_a: GtkObjectsType) {
    let dialog = FileChooserDialog::new(
        Some(&t!("D_STORE_OPML_SELECT_FILE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        FileChooserAction::Save,
    );
    dialog.add_buttons(&[("Save", ResponseType::Ok), ("Cancel", ResponseType::Cancel)]);
    dialog.set_select_multiple(false);
    let ev_se = g_ev_se;
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Ok {
            let files = dialog.filenames();
            if !files.is_empty() {
                let payload = vec![AValue::ASTR(
                    files[0].as_path().to_str().unwrap().to_string(),
                )];
                let _r = ev_se.send(GuiEvents::DialogData("export-opml".to_string(), payload));
            }
        }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_OPML_EXPORT, &dialog.upcast());
}

fn create_about_dialog(gtk_obj_a: GtkObjectsType, ddd: &mut DialogDataDistributor) {
    let dialog = AboutDialog::new();
    dialog.set_program_name(APP_NAME_CAMEL);
    dialog.set_version(None); // is injected
    dialog.set_title(format!("{}: {}", t!("ABOUT_APP_TXT"), &APP_NAME_CAMEL).as_str());
    dialog.set_comments(Some(&format!(
        "{} \n {}",
        t!("ABOUT_APP_DESCRIPTION"),
        CARGO_PKG_LICENSE
    )));
    dialog.set_authors(&[CARGO_PKG_AUTHORS, AUTHOR_STATIC]);
    dialog.set_website_label(Some(APP_WEBSITE_LABEL));
    dialog.set_website(Some(APP_WEBSITE));
    dialog.set_license(Some(APP_LICENSE));
    let buf = IconLoader::decompress_string_to_vec(ICON_04_GRASS_CUT_2, "about_dialog");
    let pb: Pixbuf = IconLoader::vec_to_pixbuf(&buf).unwrap();
    dialog.set_logo(Some(&pb));
    dialog.set_transient_for((*gtk_obj_a).read().unwrap().get_window().as_ref());
    dialog.connect_response(move |dialog, _r_type| {
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    let dia_c = dialog.clone();
    ddd.set_dialog_distribute(DIALOG_ABOUT, move |dialogdata| {
        if let Some(s) = dialogdata.first().unwrap().str() {
            dia_c.set_version(Some(&s));
        }
    });

    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_ABOUT, &dialog.upcast());
}

fn create_subscription_statistic_dialog(
    gtk_obj_a: GtkObjectsType,
    ddd: &mut DialogDataDistributor,
) {
    let width = 800;
    let dialog = Dialog::with_buttons::<Window>(
        Some(&t!("D_SUBSCRIPTION_STATISTIC_TITLE")),
        (*gtk_obj_a).read().unwrap().get_window().as_ref(),
        gtk::DialogFlags::MODAL,
        &[(&t!("D_BUTTON_OK"), ResponseType::Ok)],
    );
    dialog.set_width_request(width);
    dialog.set_default_response(ResponseType::Ok);

    let grid2 = Grid::new();
    dialog.content_area().add(&grid2);
    //  notebook.append_page(&grid2, Some(&label_nb2));
    grid2.set_vexpand(true);
    grid2.set_hexpand(true);

    let mut line = 0;
    let label1a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_MAIN_WEBSITE")));
    grid2.attach(&label1a, 0, line, 1, 1);
    let label1b = Label::new(Some("_"));
    grid2.attach(&label1b, 1, line, 1, 1);
    line += 1;
    let label2a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_NUM_MESSAGES")));
    grid2.attach(&label2a, 0, line, 1, 1);
    let label2b = Label::new(Some("_"));
    grid2.attach(&label2b, 1, line, 1, 1);
    line += 1;
    let label3a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_NUM_UNREAD")));
    grid2.attach(&label3a, 0, line, 1, 1);
    let label3b = Label::new(Some("_"));
    grid2.attach(&label3b, 1, line, 1, 1);
    line += 1;
    let label4a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_LAST_DOWNLOAD")));
    grid2.attach(&label4a, 0, line, 1, 1);
    let label4b = Label::new(Some("_"));
    grid2.attach(&label4b, 1, line, 1, 1);
    line += 1;
    let label5a = Label::new(Some(&t!("D_EDIT_SUBSCRIPTION_LAST_CREATION")));
    grid2.attach(&label5a, 0, line, 1, 1);
    let label5b = Label::new(Some("_"));
    grid2.attach(&label5b, 1, line, 1, 1);
    // line += 1;
    // let label6a = Label::new(None);
    // grid2.attach(&label6a, 0, line, 1, 1);
    // let label6b = Label::new(None);
    // grid2.attach(&label6b, 1, line, 1, 1);

    line += 1;
    //    let label7a = Label::new(Some(&t!("D_SUBSCRIPTION_STATISTIC_ERRORLIST")));
    let scrolledwindow1 = ScrolledWindow::new(NONE_ADJ, NONE_ADJ);
    grid2.attach(&scrolledwindow1, 0, line, 2, 1);

    scrolledwindow1.set_widget_name("scrolledwindow_0");
    scrolledwindow1.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic); // scrollbar-h, scrollbar-v
    scrolledwindow1.set_vexpand(true);
    scrolledwindow1.set_hexpand(true);
    scrolledwindow1.set_shadow_type(ShadowType::EtchedIn);
    let err_list = create_statistic_listview(gtk_obj_a.clone());
    scrolledwindow1.add(&err_list);
    dialog.connect_response(move |dialog, _rt| {
        // match rt {
        //     ResponseType::Ok => {}
        //     _ => {
        //         warn!("statistics:response unexpected {}", rt);
        //     }
        // }
        dialog.hide();
    });
    dialog.connect_delete_event(|dia, _| {
        dia.hide();
        gtk::Inhibit(true)
    });
    //    let textview_c = textview.clone();
    ddd.set_dialog_distribute(DIALOG_SUBSCRIPTION_STATISTIC, move |dialogdata| {
        // let mut url = String::default();
        if let Some(s) = dialogdata.get(3).unwrap().str() {
            label2b.set_text(&s); // 3: num-all
        }
        if let Some(s) = dialogdata.get(4).unwrap().str() {
            label3b.set_text(&s); // 4: num-unread
        }
        if let Some(s) = dialogdata.get(5).unwrap().str() {
            label1b.set_text(&s); // 5: main website
        }
        if let Some(s) = dialogdata.get(6).unwrap().str() {
            label4b.set_text(&s); // update-int
        }
        if let Some(s) = dialogdata.get(7).unwrap().str() {
            label5b.set_text(&s); // update-ext
        }
    });
    let mut ret = (*gtk_obj_a).write().unwrap();
    ret.set_dialog(DIALOG_SUBSCRIPTION_STATISTIC, &dialog);
}

//
#[cfg(test)]
mod names_test {
    use super::*;

    #[test]
    fn interval() {
        assert_eq!(get_fetch_interval("Hozrs".to_string()), 0);
        println!("!!! Hours");
        assert_eq!(get_fetch_interval("Hours".to_string()), 2);
    }
}
