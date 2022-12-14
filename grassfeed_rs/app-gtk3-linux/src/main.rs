extern crate xdg;
#[macro_use]
extern crate rust_i18n;

mod args_lang;

use fr_core::config::init_system;
use fr_core::config::setup_logger_prod;
use fr_core::db::check_consistency;
use resources::application_id::*;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

i18n!("../resources/locales");

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

    let version_str = env!("CARGO_PKG_VERSION").to_string();
    let o_opts = args_lang::parse_args(&version_str);
    let mut debug_level = 0;
    if o_opts.is_none() {
        return; // commandline option were handled, do not start the gui
    }
    let opts = o_opts.unwrap();
    if opts.debug || opts.check {
        debug_level = 5;
    }
    init_system::check_or_create_folder(&cache);
    let r = setup_logger_prod::setup_logger(debug_level, &cache, APP_NAME);
    if r.is_err() {
        eprintln!("Stopping: {:?}", &r);
        return;
    }
    if opts.check {
        trace!("Database Check {} {} {} ", &version_str, &conf, &cache);
        check_consistency::databases_check_manual(&conf, &cache);
        return; // no gui
    }
    info!(
        "Starting {} with {} {}  locale={:?} V={}",
        APP_NAME,
        &conf,
        &cache,
        rust_i18n::locale(),
        &version_str,
    );
    let gfconf = init_system::GrassFeederConfig {
        path_config: conf,
        path_cache: cache,
        debug_mode: opts.debug,
        version: version_str,
    };
    let appcontext = init_system::start(gfconf);
    init_system::run(&appcontext);
    info!("Stopped {} ", APP_NAME,);
}
