use chrono::TimeZone;
use fr_core::db::sqlite_context::SqliteContext;
use fr_core::db::subscription_entry::SubscriptionEntry;
use std::fs::DirEntry;
use std::io::Write;
use std::time::SystemTime;

static DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn unix_time_display(unixtime: u64) -> String {
    let fetchd_loc = chrono::offset::Local.timestamp(unixtime as i64, 0);
    fetchd_loc.format(DATETIME_FORMAT).to_string()
}

/// Modified, Accessed, Created
fn unix_time_from_direntry(direntry: &DirEntry) -> (u64, u64, u64) {
    let mut r_mod: u64 = 0;
    let mut r_acc: u64 = 0;
    let mut r_crea: u64 = 0;

    if let Ok(metadata) = direntry.metadata() {
        if let Ok(modified_sys) = metadata.modified() {
            r_mod = modified_sys
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        if let Ok(t_sys) = metadata.accessed() {
            r_acc = t_sys
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        if let Ok(t_sys) = metadata.created() {
            r_crea = t_sys
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
    }
    (r_mod, r_acc, r_crea)
}

///  with trailing slash
fn create_debian_changelog(
    in_folder: &str,
    out_file: &str,
    package_name: &str,
    top_line_rest: &str,
    bottom_part: &str,
) {
    let mut file_t_mod_list: Vec<(String, u64)> = Vec::default();
    if let Ok(entries) = std::fs::read_dir(in_folder) {
        entries.for_each(|e| {
            if let Ok(direntry) = e {
                let fname = direntry.file_name().to_str().unwrap().to_string();
                let (tmod, tacc, tcrea) = unix_time_from_direntry(&direntry);
                // debug!(                    "fname={}  \t{}\t{}\t{}",                    fname,                    unix_time_display(tmod),                    unix_time_display(tacc),                    unix_time_display(tcrea),                );
                if fname.contains(':') && fname.ends_with(".txt") {
                    file_t_mod_list.push((fname, t_mod));
                }
            }
        });
    }
    file_list.sort();
    debug!("FILEs={:?}  =>   OUT={}", file_list, out_file);

    let o_outfile = std::fs::File::create(out_file);
    if o_outfile.is_err() {
        error!("opening {} : {:?}", out_file, o_outfile.err());
        return;
    }
    let mut outfile = o_outfile.unwrap();

    for name in file_list {
        //         let parts = name.replace(".txt", "").split(':').collect::<&str>().as_slice();
        let replaced = name.replace(".txt", "");
        let parts: Vec<&str> = replaced.split(':').collect();
        // debug!("S1={:?}", s1);

        //		outfile.wr
        let line1 = format!("LINE {}    {:?} \n", name, parts);
        let o_wr = outfile.write_all(line1.as_bytes());
        if o_wr.is_err() {
            error!("writing to {} => {:?}", out_file, o_wr.err());
        }
    }
}

#[test]
pub fn do_changelog() {
    setup();

    create_debian_changelog(
        "../app-changes/",
        "../target/test_debian_changelog.txt",
        "grassfeeder",
        "unstable; urgency=low",
        "Marcus der Schlegler <schleglermarcus@posteo.de>",
    );
}

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
        let _r = testing::logger_config::setup_logger();
    });
}
