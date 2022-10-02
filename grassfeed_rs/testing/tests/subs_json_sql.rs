use fr_core::db::errors_repo::ErrorEntry;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::sqlite_context::SqliteContext;
use fr_core::db::subscription_entry::SubscriptionEntry;

// #[ignore]
#[test]
fn t_error_repo_store() {
    setup();
    let mut e_repo = ErrorRepo::new("../target/err_rep/");
    let mut e1 = ErrorEntry::default();
    e1.text = "Hello!".to_string();
    e1.subs_id = 13;
    e_repo.store_error(&e1);
    e_repo.check_or_store();
    let next_id = e_repo.next_id();
    assert!(next_id > 10);
    let subs_list = e_repo.get_by_subscription(13);
    assert!(subs_list.len() >= 1);
    // debug!("LIST={:?}", subs_list);
}

//

const IN_JSON: &str = "../fr_core/tests/data/san_subs_list_dmg1.json";
const OUT_DB: &str = "../target/db_rungui_reg/subscriptions.db";

// const IN_JSON: &str = "/home/www/.config/grassfeeder/subscription_list.json";
// const OUT_DB: &str = "/home/www/.config/grassfeeder/subscriptions.db";
#[ignore]
#[test]
pub fn import_json() {
    setup();
    debug!("converting: {} => {} ... ", IN_JSON, OUT_DB);
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
        let _r = testing::logger_config_local::setup_logger();
    });
}
