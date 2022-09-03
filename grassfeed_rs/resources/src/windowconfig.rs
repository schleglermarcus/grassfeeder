#[derive(Clone, Debug)]
pub struct GtkWindowConfig {
    pub title: String,
    pub default_width: i32,
    pub default_height: i32,
    pub show_menubar: bool,
    pub app_url: String,
}

impl Default for GtkWindowConfig {
    fn default() -> Self {
        GtkWindowConfig {
            title: String::from("default title"),
            default_width: 50,
            default_height: 50,
            show_menubar: false,
            app_url: String::from("default.wc"),
        }
    }
}
