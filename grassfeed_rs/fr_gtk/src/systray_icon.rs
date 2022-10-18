use crate::gtk::prelude::WidgetExt;
use crate::util::EvSenderWrapper;
use flume::Sender;
use gtk::prelude::GtkMenuItemExt;
use gtk::prelude::MenuShellExt;
use gui_layer::abstract_ui::GuiEvents;
use libappindicator::AppIndicator;
use libappindicator::AppIndicatorStatus;

pub const ICON_PATH: &str = "/usr/share/pixmaps/grassfeeder/";

pub const ICON2: &str = "grassfeeder-indicator2"; // grassfeeder-indicator2.png

pub fn create_status_icon_1(gui_event_sender: Sender<GuiEvents>, app_url: String) -> AppIndicator {
    debug!("INDI: {} {}  {}", &app_url, ICON_PATH, ICON2);


    let mut indicator = AppIndicator::new("app_url.as_str()" , "");
    indicator.set_icon_theme_path(ICON_PATH);
    indicator.set_icon(ICON2);
    indicator.set_status(AppIndicatorStatus::Active);
    let mut menu = gtk::Menu::new();
    let mi1 = gtk::CheckMenuItem::with_label("TODO  Show Window ");
    let esw = EvSenderWrapper(gui_event_sender.clone());
    mi1.connect_activate(move |_| {
        esw.sendw(GuiEvents::Indicator("window-restore".to_string()));
    });
    menu.append(&mi1);
    let mi2 = gtk::CheckMenuItem::with_label("TODO  Quit ");
    let esw = EvSenderWrapper(gui_event_sender.clone());
    mi2.connect_activate(move |_| {
        esw.sendw(GuiEvents::Indicator("application-quit".to_string()));
    });
    menu.append(&mi2);
    indicator.set_menu(&mut menu);
    indicator
}

fn create_tray2() {
    // gtk::init().unwrap();
    let mut indicator = AppIndicator::new("libappindicator test application", "");
    indicator.set_status(AppIndicatorStatus::Active);
    indicator.set_icon_theme_path(ICON_PATH);
    indicator.set_icon(ICON2);
    let mut m = gtk::Menu::new();
    let mi = gtk::CheckMenuItem::with_label("Hello Rust!");
    mi.connect_activate(|_| {
        debug!("TRAy2   activate  ->  quit");
    });

    mi.connect_hide(|_m| debug!("MENU hide!"));
    mi.connect_focus(|_m, _n| {
        debug!("MENU focus!");
        gtk::Inhibit(false)
    });

    mi.connect_draw(|_m, _n| {
        debug!("MENU draw!");
        gtk::Inhibit(false)
    });

    m.append(&mi);
    indicator.set_menu(&mut m);
    m.show_all();
}
