use crate::util::EvSenderWrapper;
use flume::Sender;
use gtk::prelude::GtkMenuItemExt;
use gtk::prelude::MenuShellExt;
use gui_layer::abstract_ui::GuiEvents;
use libappindicator::AppIndicator;
use libappindicator::AppIndicatorStatus;
// use ui_gtk::GtkObjectsType;
// use gtk::prelude::WidgetExt;

pub const ICON_PATH: &str = "/usr/share/pixmaps/grassfeeder/";

pub const ICON2: &str = "grassfeeder-indicator2.png";
// 						 grassfeeder-indicator2.png

pub fn create_status_icon(gui_event_sender: Sender<GuiEvents>, app_url: String) {
    debug!("INDI: {} {}  {}", &app_url, ICON_PATH, ICON2);
    let mut indicator = AppIndicator::new(app_url.as_str(), "");
    indicator.set_icon_theme_path(ICON_PATH);
    indicator.set_icon("grassfeeder-indicator2");
	indicator.set_status(AppIndicatorStatus::Active);

    let mut m = gtk::Menu::new();
    let mi1 = gtk::CheckMenuItem::with_label("TODO  Show Window ");
    let esw = EvSenderWrapper(gui_event_sender.clone());
    mi1.connect_activate(move |_| {
        esw.sendw(GuiEvents::Indicator("window-restore".to_string()));
    });
    m.append(&mi1);

    let mi2 = gtk::CheckMenuItem::with_label("TODO  Quit ");
    let esw = EvSenderWrapper(gui_event_sender.clone());
    mi2.connect_activate(move |_| {
        esw.sendw(GuiEvents::Indicator("application-quit".to_string()));
    });
    m.append(&mi2);

	indicator.set_menu(&mut m);
	
}

/*
fn libapp1() {
    gtk::init().unwrap();
    let mut indicator = AppIndicator::new("libappindicator test application", "");
    indicator.set_status(AppIndicatorStatus::Active);

    // let p_cmd = Path::new(env!("CARGO_MANIFEST_DIR"));
    // let icon_path = p_cmd.join("examples");
    // let icon_path = p_cmd.join("src/icons/");
    // let icon_p_str = icon_path.to_str().unwrap();

    let icon_path = "/usr/share/pixmaps/grassfeeder/";

    println!("01    {:?}", icon_path);
    indicator.set_icon_theme_path(icon_path);

    // indicator.set_icon_full("rust-logo", "icon");
    //    indicator.set_icon_full("grassfeeder-indicator1", "icon");

    indicator.set_icon("grassfeeder-indicator2.png");
    // 							 04-grass-cut-2.png

    println!("02");

    let mut m = gtk::Menu::new();
    let mi = gtk::CheckMenuItem::with_label("Hello Rust!");
    mi.connect_activate(|_| {
        println!("activate  ->  quit");
        gtk::main_quit();
    });
    m.append(&mi);
    indicator.set_menu(&mut m);

    m.show_all();
    println!("main ....");
    gtk::main();

    println!("finished");
}
*/
