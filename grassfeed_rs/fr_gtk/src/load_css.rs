// https://thegnomejournal.wordpress.com/2011/03/15/styling-gtk-with-css/
// https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Transitions/Using_CSS_transitions
// https://docs.gtk.org/gtk3/css-overview.html#selectors
// https://gtk-rs.org/gtk3-rs/stable/latest/docs/gtk/prelude/trait.WidgetExt.html
// The :focused pseudo-class is deprecated. Use :focus instead.
// The :prelight pseudo-class is deprecated. Use :hover instead.
// GTK_DEBUG=interactive
//

pub const TAB_MARKER_HEIGHT: u8 = 1;

use crate::gtk::prelude::CssProviderExt;
// use crate::gtk::prelude::StyleProviderExt;

// double curly brackets for rust strings
fn style_scrolled(name: &str, w_id: u8, height: u8) -> String {
    format!(
        "\
    #{}_{}    {{ border-top:{}px solid transparent; }} \
    #{}_{}_1  {{ border-top:{}px solid green;    }} \
    #{}_{}_2  {{ border-top:{}px solid transparent;  \
    transition-property:border-top-color; transition-duration:3s;  \
    transition-timing-function:linear;  transition-delay:1s; }} ",
        name, w_id, height, name, w_id, height, name, w_id, height
    )
}

pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.connect_parsing_error(|_a1, section, p_err| error!("{:?} {:?} ", section, p_err));
    let s1 = style_scrolled("scrolledwindow", 0, TAB_MARKER_HEIGHT);
    let s2 = style_scrolled("scrolledwindow", 1, TAB_MARKER_HEIGHT);
    let s3 = style_scrolled("box", 1, TAB_MARKER_HEIGHT);
    let style = format!("{} \n {} \n {} \n ", s1, s2, s3);
    match provider.load_from_data(style.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to load CSS: {}  \n{}", e, style);
        }
    };

/*
    let wp = gtk::WidgetPath::new();

    let val = provider.style_property(
        &wp,
        gtk::StateFlags::NORMAL,
        glib::ParamSpecString::new("background-color", "", "", None, glib::ParamFlags::READABLE),
    );
    debug!("PROV: VAL={:?}", val);
*/


    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Error initializing gtk css provider."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
