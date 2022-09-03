use chrono::DateTime;
use chrono::FixedOffset;
use chrono::Local;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use lz4_compression::prelude;
use serde::Deserialize;
use serde::Serialize;
use std::io::BufWriter;
use std::io::Write;

const FILENAME_BIN: &str = "../target/db_sub_serde.txt";
const FILENAME_JSON: &str = "../target/db_sub_serde-json.txt";

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SubSource {
    pub num: isize,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SubList {
    #[serde(rename = "SubList")]
    pub list: Vec<SubSource>,
}

fn write_to(
    filename: String,
    input: &Vec<SubscriptionEntry>,
    converter: &dyn Fn(&SubscriptionEntry) -> Option<String>,
) -> std::io::Result<usize> {
    let mut bytes_written: usize = 0;
    let out = std::fs::File::create(filename)?;
    let mut buf = BufWriter::new(out);
    input
        .iter()
        .filter_map(|se| converter(se))
        .for_each(|line| {
            let bbuf = line.as_bytes();
            match buf.write(bbuf) {
                Ok(bytes) => {
                    let _r = buf.write(&['\n' as u8]);
                    bytes_written += bytes + 1;
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        });
    buf.flush()?;
    Ok(bytes_written)
}

fn subscription_entry_to_json(input: &SubscriptionEntry) -> Option<String> {
    match serde_json::to_string(input) {
        Ok(encoded) => Some(encoded),
        Err(er) => {
            error!("serde_json {:?} \n {:?}", er, &input.subs_id);
            None
        }
    }
}

fn subscription_entry_to_txt(input: &SubscriptionEntry) -> Option<String> {
    match bincode::serialize(input) {
        Ok(encoded) => Some(compress_a(&encoded)),
        Err(er) => {
            error!("bincode_serizalize {:?} \n {:?}", er, &input.subs_id);
            None
        }
    }
}

#[test]
fn test_compare_txt() {
    setup();
    let list = prepare_sub_list();
    let _r = write_to(FILENAME_BIN.to_string(), &list, &subscription_entry_to_txt);
    let _r = write_to(
        FILENAME_JSON.to_string(),
        &list,
        &subscription_entry_to_json,
    );
    let list_txt = read_from(FILENAME_BIN.to_string(), &txt_to_subscription_entry);
    assert_eq!(list_txt.len(), list.len());
    list_txt
        .iter()
        .enumerate()
        .for_each(|(n, l)| assert_eq!(l, list.get(n).unwrap()));

    let list_json = read_from(FILENAME_JSON.to_string(), &json_to_subscription_entry);
    assert_eq!(list_json.len(), list.len());
    list_json
        .iter()
        .enumerate()
        .for_each(|(n, l)| assert_eq!(l, list.get(n).unwrap()));
}

fn read_from(
    filename: String,
    converter: &dyn Fn(String) -> Option<SubscriptionEntry>,
) -> Vec<SubscriptionEntry> {
    let mut subscriptions_list: Vec<SubscriptionEntry> = Vec::default();
    match std::fs::read_to_string(filename.clone()) {
        Ok(f_str) => {
            subscriptions_list = f_str
                .lines()
                .filter_map(|line| converter(line.to_string()))
                .collect();
        }
        Err(e) => {
            error!("{:?}  {}", e, filename)
        }
    }
    subscriptions_list
}

fn json_to_subscription_entry(line: String) -> Option<SubscriptionEntry> {
    let dec_r: serde_json::Result<SubscriptionEntry> = serde_json::from_str(&line);
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("serde_json:from_str {:?}   {:?} ", e, &line);
            None
        }
    }
}

fn txt_to_subscription_entry(line: String) -> Option<SubscriptionEntry> {
    let dc_bytes = decompress_a(line.to_string());
    let dec_r: bincode::Result<SubscriptionEntry> = bincode::deserialize(&dc_bytes);
    match dec_r {
        Ok(dec_se) => Some(dec_se),
        Err(e) => {
            error!("bincode:deserialize {:?}   {:?} ", e, &line);
            None
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct SubscriptionEntry {
    pub subs_id: isize,
    pub display_name: String,
    pub is_folder: bool,
    pub url: String, // xml_url
    pub icon_id: usize,
    pub parent_repo_id: isize,
    pub folder_position: isize,
    ///  timestamp the website compiled the rss file
    pub updated_ext: i64,
    ///  timestamp when we updated this feed from external website
    pub updated_int: i64,
    ///  timestamp when we got the last icon from the website
    pub updated_icon: i64,
    pub expanded: bool,
    pub website_url: String,
    pub last_selected_msg: isize,
    pub num_msg_all: isize,
    pub num_msg_unread: isize,
    #[serde(skip)]
    pub num_msg_all_unread_dirty: bool,
    #[serde(skip)]
    pub status: usize,
    #[serde(skip)]
    pub tree_path: Option<Vec<u16>>,
}

// #[ignore]
#[test]
fn subs_serde_json() {
    setup();
    let list = prepare_sub_list();
    let serialized = serde_json::to_string(&list).unwrap();
    assert_eq!(serialized.len(), 640);
    let c_b64 = compress_a(serialized.as_bytes());
    assert_eq!(c_b64.len(), 564);
}

// #[ignore]
#[test]
fn subs_serde_bincode() {
    setup();
    let list = prepare_sub_list();
    let encoded: Vec<u8> = bincode::serialize(&list).unwrap();
    assert_eq!(encoded.len(), 349);
    let compr = compress_a(&encoded);
    assert_eq!(compr.len(), 288);
}

fn prepare_sub_list() -> Vec<SubscriptionEntry> {
    let mut list: Vec<SubscriptionEntry> = Vec::default();
    let mut s1 = SubscriptionEntry::default();
    s1.display_name = "A 無料ダウンロード অঙ্কিতার আইনজীবী   ".to_string();
    s1.url = "http://falschzitate.blogspot.com/feeds/posts/default".to_string();
    s1.num_msg_all = 5;
    s1.num_msg_unread = 2;
    s1.subs_id = 123;
    s1.icon_id = 456;
    s1.parent_repo_id = 789;
    s1.folder_position = 234;
    s1.updated_ext = -1;
    s1.updated_int = -1;
    s1.updated_icon = -1;
    s1.expanded = true;
    s1.is_folder = false;
    list.push(s1);
    let mut s1 = SubscriptionEntry::default();
    s1.display_name = "B".to_string();
    list.push(s1);
    list
}

///  Compress the data, then  encode base64  into String
pub fn compress_a(uncompressed_bytes: &[u8]) -> String {
    let compressed_data = prelude::compress(uncompressed_bytes); // uncompressed.as_bytes()
    base64::encode(compressed_data)
}

///  Modifies this entry, decompresses text values from DB
pub fn decompress_a(encoded_b64: String) -> Vec<u8> {
    match base64::decode(encoded_b64) {
        Ok(buffer) => match prelude::decompress(&buffer) {
            Ok(vec_u8) => return vec_u8,
            Err(e) => {
                error!("decompress_a:lz4:decompress: {:?}", e);
            }
        },
        Err(e) => {
            error!("decompress_a:base64:decode:  {:?}", e);
        }
    }
    Vec::default()
}

#[test]
fn test_compress() {
    let h = "Hello".to_string();
    let c = compress_a(h.as_bytes());
    let d = decompress_a(c);
    assert_eq!(h.as_bytes(), d);
}

// Sequences into   xml does not work.
// #[ignore]
// #[test]
#[allow(dead_code)]
fn subs_serde_xml() {
    setup();
    let mut slist = SubList::default();
    let s1 = SubSource { num: 3 };
    slist.list.push(s1);
    match serde_xml_rs::to_string(&slist) {
        Ok(xml_ser) => {
            debug!("XML={}", xml_ser);
            debug!("XML LENGTH={}", xml_ser.len());
        }
        Err(e) => warn!("{:?}", e),
    };
}

// #[test]
#[allow(dead_code)]
fn time_display() {
    // setup();
    let now: DateTime<Local> = Local::now();
    let rfc2822: String = now.to_rfc2822();
    let another_st = "Tue, 1 Jul 2003 10:52:37 +0000";
    let another: DateTime<FixedOffset> = DateTime::parse_from_rfc2822(&another_st).unwrap();
    let nai: NaiveDate = NaiveDate::from_ymd(0, 1, 1);
    let ndt: NaiveDateTime = NaiveDateTime::from_timestamp(0, 0);
    trace!(
        "Now is: {}   Fixed other={}   Naive={}  NTS={}   ",
        rfc2822,
        another,
        nai,
        ndt
    );
}

#[macro_use]
extern crate rust_i18n;

// Init translations for current crate.
i18n!("../resources/locales");

#[test]
fn t_translations() {
    setup();
    let about: String = t!("M_ABOUT");
    assert_eq!(about, "About".to_string());
    let about: String = t!("M_ABOUT", locale = "de");
    assert_eq!(about, "Über".to_string());
}

// ------------------------------------

#[macro_use]
extern crate log;
use std::sync::Once;
static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config::setup_logger();
    });
}
