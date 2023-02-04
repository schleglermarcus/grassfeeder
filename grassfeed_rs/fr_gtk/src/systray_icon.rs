use crate::gtk::prelude::WidgetExt;
use gtk::prelude::GtkMenuItemExt;
use gtk::prelude::MenuShellExt;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::UiSenderWrapperType;
use libappindicator::AppIndicator;

pub const ICON_PATH_REL: &str = "usr/share/pixmaps/grassfeeder/"; // "/usr/share/pixmaps/grassfeeder/";
pub const FILENAME_BASE: &str = "grassfeeder-indicator2";

//	libappindicator  is somewhat broken. It requests absolute paths for the Icons.
//  This works with the regular *.deb distribution
//  But it fails with   AppImage distribution
pub fn create_systray_icon_3(g_ev_se: UiSenderWrapperType, app_url: String) -> AppIndicator {
    let icon_path_base;

    let icon_filename1 = format!("{ICON_PATH_REL}{FILENAME_BASE}.png");
    if std::path::Path::new(&icon_filename1).exists() {
        icon_path_base = ICON_PATH_REL.to_string();
        debug!("Icon1 {} found! ", icon_filename1,);
    } else {
        warn!("Icon {} missing! ", icon_filename1,);
        let icon_filename2 = format!("/{ICON_PATH_REL}{FILENAME_BASE}.png");
        icon_path_base = format!("/{ICON_PATH_REL}");
        if !std::path::Path::new(&icon_filename2).exists() {
            debug!("Icon2 {} found! ", icon_filename2);
        } else {
            error!("Icon {} missing!", &icon_filename2);
        }
    }
    let mut indicator = AppIndicator::new(app_url.as_str(), "");
    indicator.set_icon_theme_path(&icon_path_base);
    indicator.set_icon(FILENAME_BASE);

    let mut menu = gtk::Menu::new();
    let mi1 = gtk::MenuItem::with_label(&t!("SYSTRAY_CMD_SHOW_WINDOW"));
    let se_w1 = g_ev_se.clone(); //  EvSenderWrapper(g_ev_se.clone());
    mi1.connect_activate(move |_| {
        se_w1.send(GuiEvents::Indicator(
            "show-window".to_string(),
            gtk::current_event_time(),
        ));
    });
    menu.append(&mi1);
    let mi2 = gtk::MenuItem::with_label(&t!("SYSTRAY_CMD_QUIT"));
    let se_w2 = g_ev_se.clone();
    mi2.connect_activate(move |_| {
        se_w2.send(GuiEvents::Indicator(
            "app-quit".to_string(),
            gtk::current_event_time(),
        ));
    });
    menu.append(&mi2);
    menu.show_all();
    menu.connect_focus(|_m, dir| {
        debug!("menu: focus! {:?}", dir);
        gtk::Inhibit(false)
    });
    menu.connect_window_notify(|_m| {
        debug!("menu: win_notif ! ");
    });
    indicator.set_menu(&mut menu);
    indicator.set_title(&t!("ABOUT_APP_DESCRIPTION")); // later: more interactive text
    indicator
}
