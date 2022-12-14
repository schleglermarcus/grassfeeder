use fr_core::config::init_system;
use resources::loc;

#[macro_use]
extern crate rust_i18n;
i18n!("../resources/locales");

// cargo watch -s "cargo run  --example regular --features ui-gtk   "
fn main() {
    setup();
    loc::init_locales();
    let gfconf = init_system::GrassFeederConfig {
        path_config: "target/db_rungui_reg/".to_string(),
        path_cache: "target/db_rungui_reg/".to_string(),
        debug_mode: true,
        version: "run_reg-0".to_string(),
    };
    let appcontext = init_system::start(gfconf);
    init_system::run(&appcontext);
}

// ------------------------------------
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config_local::setup_logger();
    });
}
