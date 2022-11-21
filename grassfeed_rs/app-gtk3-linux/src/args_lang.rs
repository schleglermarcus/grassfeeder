use gumdrop::Options;
use resources::application_id::*;

const CARGO_PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

const LOCALES_LIST: [&str; 2] = ["en", "de"]; // later into environment variable ?

// i18n!("../resources/locales");

#[derive(Debug, Options)]
pub struct MyOptions {
    #[options(help = "print this message.")]
    help: bool,

    #[options(help = "show version info.")]
    version: bool,

    #[options(help = "print debug messages, lower the treshold for logging")]
    pub debug: bool,

    #[options(help = "Language selection.")]
    lang: Option<String>,

	#[options(help = "Databases consistency check")]
    pub check: bool,

}

/// 1. Set the desired language, if available
/// 2. Set the environment given lanuguage, if available
/// 3. If both failed, set  the    "en"  language
///    Returns the selected   language
pub fn init_locales(desired: Option<String>) -> Option<String> {
    let mut selected: Option<String> = None;
    if let Some(d) = desired {
        if LOCALES_LIST.contains(&d.as_str()) {
            selected = Some(d);
        }
    }
    if selected.is_none() {
        if let Ok(lang) = std::env::var("LANG") {
            let lowercaselang = lang.to_lowercase();
            LOCALES_LIST.iter().for_each(|l| {
                if lowercaselang.starts_with(l) {
                    selected.replace(l.to_string());
                }
            });
        }
    }
    if selected.is_none() {
        selected = Some("en".to_string()); // default
    }
    rust_i18n::set_locale(selected.as_ref().unwrap().as_str());
    selected
}

pub fn parse_args(version_str: &str) -> Option<MyOptions> {
    let args: Vec<String> = std::env::args().collect();
    let (call_path, argsonly) = args.split_at(1);
    let o_opts = MyOptions::parse_args_default(argsonly);
    if let Err(e) = o_opts {
        println!("Error parsing options: {:?}", e);
        println!("{} ", MyOptions::usage());
        return None;
    }
    let opts = o_opts.unwrap();
    if opts.help_requested() {
        println!("{} ", opts.self_usage());
        println!("\t\t\tAvailable Languages: {:?}", LOCALES_LIST,);
        return None;
    }
    if opts.version {
        println!(
            "{} {} {} {} ",
            APP_NAME_CAMEL, CARGO_PKG_DESCRIPTION, version_str, call_path[0],
        );
        return None;
    }
    let _selected_lang = init_locales(opts.lang.clone());
    Some(opts)
}
