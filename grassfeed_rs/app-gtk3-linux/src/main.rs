extern crate xdg;
#[macro_use]
extern crate rust_i18n;

mod args_lang;
mod setup_logger;

use fr_core::config::prepare_ini::GrassFeederConfig;
use fr_core::grassfeeder;
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
    let o_opts = args_lang::parse_args();

    let mut debug_level = 0;
    if let Some(ref opts) = o_opts {
        if opts.debug {
            debug_level = 5;
        }
    }
    let _r = setup_logger::setup_logger(debug_level, &cache, APP_NAME);
    info!(
        "Starting {} with {} {}  locale={:?}",
        APP_NAME,
        conf,
        cache,
        rust_i18n::locale()
    );
    let mut gfconf = GrassFeederConfig {
        path_config: conf,
        path_cache: cache,
        debug_mode: false,
    };
    if let Some(opts) = o_opts {
        gfconf.debug_mode = opts.debug;

        let appcontext = grassfeeder::start(gfconf);
        grassfeeder::run(&appcontext);

        info!("Stopped {} ", APP_NAME,);
    }
}
