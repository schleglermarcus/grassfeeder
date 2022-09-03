use fr_core::config::prepare_ini::GrassFeederConfig;
use resources::loc;

#[macro_use]
extern crate rust_i18n;
i18n!("../resources/locales");

#[ignore]
#[test]
fn rungui_regular() {
    setup();
    loc::init_locales();
    let gfconf = GrassFeederConfig {
        path_config: "../target/db_rungui_reg".to_string(),
        path_cache: "../target/db_rungui_reg".to_string(),
        debug_mode: true,
    };
    let appcontext = fr_core::grassfeeder::start(gfconf);
    fr_core::grassfeeder::run(&appcontext);
}

// ------------------------------------
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config::setup_logger();
    });
}