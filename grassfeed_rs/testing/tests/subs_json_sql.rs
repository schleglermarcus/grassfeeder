use fr_core::db::sqlite_context::SqliteContext;
use fr_core::db::subscription_entry::SubscriptionEntry;

const IN_JSON: &str = "../fr_core/tests/data/san_subs_list_dmg1.json";
const OUT_DB: &str = "../target/subs_json_sql_imported.db";

// const IN_JSON: &str = "/home/work/.config/grassfeeder/subscription_list.json";
// const OUT_DB: &str = "/home/work/.config/grassfeeder/messages.db";

#[test]
pub fn import_json() {
    setup();
    let lines = std::fs::read_to_string(IN_JSON.to_string()).unwrap();
    let dec_r: serde_json::Result<Vec<SubscriptionEntry>> = serde_json::from_str(&lines);
    if dec_r.is_err() {
        error!("serde_json:from_str {:?}   {:?} ", dec_r.err(), IN_JSON);
        return;
    }
    let ctx = SqliteContext::new(OUT_DB.to_string());
    let _r = ctx.delete_table();
    ctx.create_table();
    let json_vec = dec_r.unwrap();

    let mut num_json: usize = 0;
    let mut num_db: usize = 0;
    for entry in json_vec {
        num_json += 1;
        if entry.subs_id < 10 {
            continue;
        }
        match ctx.insert(&entry, entry.subs_id != 0) {
            Ok(_indexval) => {
                num_db += 1;
            }
            Err(e) => {
                error!("store_entry: {:?} {:?} ", &entry, e);
            }
        }
    }
    debug!(
        "imported {}=>{}  {} of {} subscriptions. ",
        IN_JSON, OUT_DB, num_db, num_json,
    );
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
