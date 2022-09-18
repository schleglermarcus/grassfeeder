extern crate xdg;
#[macro_use]
extern crate rust_i18n;

mod args_lang;
mod setup_logger;

//  use fr_core::config::init_system::GrassFeederConfig;
use fr_core::config::init_system;
use resources::application_id::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

i18n!("../resources/locales");

/// include!(concat!(env!("OUT_DIR"), "/gen_git_info.rs"));

fn main() {
    let xdg_dirs = xdg::BaseDirectories::with_prefix(APP_NAME).unwrap();
    let conf: String = xdg_dirs
        .get_config_home()
        .as_path()
        .to_str()
        .unwrap()
        .to_string();
    let cache: String = xdg_dirs
        .get_cache_home()
        .as_path()
        .to_str()
        .unwrap()
        .to_string();

    // let version_str = format!(        "{} : {} : {}",        RCS_CARGO_PKG_VERSION, RCS_BRANCH, RCS_VERSION    );
    let version_str = env!("CARGO_PKG_VERSION").to_string();

    let o_opts = args_lang::parse_args(&version_str);

    let mut debug_level = 0;
    if let Some(ref opts) = o_opts {
        if opts.debug {
            debug_level = 5;
        }
    } else {
        return; // commandline option were handled, do not start the gui
    }
    let r = setup_logger::setup_logger(debug_level, &cache, APP_NAME);
    if r.is_err() {
        eprintln!("Stopping: {:?}", &r);
        return;
    }
    info!(
        "Starting {} with {} {}  locale={:?} V={}",
        APP_NAME,
        &conf,
        &cache,
        rust_i18n::locale(),
        &version_str,
    );
    let mut gfconf = init_system::GrassFeederConfig {
        path_config: conf,
        path_cache: cache,
        debug_mode: false,
        version: version_str,
    };
    if let Some(opts) = o_opts {
        gfconf.debug_mode = opts.debug;
    }
    let appcontext = init_system::start(gfconf);
    init_system::run(&appcontext);
    info!("Stopped {} ", APP_NAME,);
}
