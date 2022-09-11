use gumdrop::Options;
use resources::application_id::*;

const CARGO_PKG_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const LOCALES_LIST: [&str; 2] = ["en", "de"];

i18n!("locales");

// Defines options that can be parsed from the command line.
//
// `derive(Options)` will generate an implementation of the trait `Options`.
// Each field must either have a `Default` implementation or an inline
// default value provided.
//
// (`Debug` is derived here only for demonstration purposes.)
#[derive(Debug, Options)]
pub struct MyOptions {
    // #[options(free)]
    // free: Vec<String>,
    #[options(help = "print this message.")]
    help: bool,

    #[options(help = "show version info.")]
    version: bool,

    #[options(help = "print debug messages, lower the treshold for logging")]
    pub debug: bool,

    #[options(help = "Language selection.")]
    lang: Option<String>,
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

pub fn parse_args() -> Option<MyOptions> {
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
            "{} {} {} {}",
            call_path[0], APP_NAME_CAMEL, CARGO_PKG_DESCRIPTION, CARGO_PKG_VERSION,
        );
        return None;
    }
    let initresult = init_locales(opts.lang.clone());
    if opts.debug {
        println!(
            "only:    i18n-locale={}  options={:?}  init_loc:{:?}",
            rust_i18n::locale(),
            opts,
            initresult
        );
    }

    Some(opts)
}