use crate::db::sqlite_context::TableInfo;
use crate::db::sqlite_context::Wrap;
use crate::util;
use lz4_compression::prelude;

pub const MARKERS_FAVORITE: u64 = 1;

#[derive(Default, PartialEq, Clone, Debug, Eq)]
pub struct CompWrap(pub String, pub Option<String>);
impl CompWrap {
    /// compressed, as in DB
    pub fn set(&mut self, s: String) {
        self.0 = s;
    }

    /// uncompressed, for display
    pub fn get_d(&mut self) -> &String {
        if self.1.is_none() {
            self.1 = Some(decompress(&self.0));
        }
        self.1.as_ref().unwrap()
    }

    /// uncompressed, does not cache
    pub fn get_decompressed(&self) -> String {
        decompress(&self.0)
    }

    /// uncompressed, for display
    pub fn set_d(&mut self, s: String) {
        self.0 = compress(&s);
        self.1 = Some(s);
    }
}

///
/// Stores a content for a  single feed item:  Title, Link, isRead, Date, Feed text  etc.
///
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageRow {
    pub message_id: isize,
    pub subscription_id: isize,
    /// keep compressed data in here
    pub title: String,
    /// individual ID for each item
    pub post_id: String,
    pub link: String,
    pub is_deleted: bool,
    pub is_read: bool,
    /// When we requested it
    pub fetch_date: i64,
    /// When it was created/modified
    pub entry_src_date: i64,
    /// when pubdate was not delivered or, was invalid formatted
    pub entry_invalid_pubdate: bool,
    /// Delivered display in html
    pub content_text: String,
    pub enclosure_url: String,
    pub author: String,
    pub categories: String,
    pub markers: u64,
    /// a copy of the decompressed title, needed for sorting
    title_d: Option<String>,
}

impl MessageRow {
    pub fn new() -> Self {
        MessageRow {
            message_id: -1,
            fetch_date: util::timestamp_now(),
            ..Default::default()
        }
    }

    pub fn is_favorite(&self) -> bool {
        self.markers & MARKERS_FAVORITE > 0
    }

    pub fn set_favorite(&mut self, n: bool) {
        if n {
            self.markers |= MARKERS_FAVORITE
        } else {
            self.markers &= !MARKERS_FAVORITE;
        }
    }
}

impl std::fmt::Display for MessageRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let srcdate = util::db_time_to_display(self.entry_src_date);
        let isdel = i32::from(self.is_deleted); // if  { 1 } else { 0 };
        write!(
            f,
            "({} {} '{}' '{}' {} D{}  )",
            self.message_id, self.subscription_id, self.post_id, self.title, srcdate, isdel
        )
    }
}

///  decode String from  base64  , then decompress the data, return String
pub fn decompress(compr_b64: &str) -> String {
    match base64::decode(compr_b64) {
        Ok(buffer) => match prelude::decompress(&buffer) {
            Ok(vec_u8) => match String::from_utf8(vec_u8) {
                Ok(s) => return s,
                Err(e) => error!("decompress:from_utf8: {:?}", e),
            },
            Err(e) => {
                error!("decompress:lz4:decompress: {:?}", e);
            }
        },
        Err(e) => {
            error!("decompress:base64:decode:  {:?}", e);
        }
    }
    String::default()
}

///  Compress the data, then  encode base64  into String
pub fn compress(uncompressed: &str) -> String {
    let compressed_data = prelude::compress(uncompressed.as_bytes());
    base64::encode(compressed_data)
}

impl TableInfo for MessageRow {
    fn table_name() -> String {
        "messages".to_string()
    }

    // https://www.tutorialspoint.com/sqlite/sqlite_data_types.htm
    // INTEGER REAL  TEXT  BLOB		BOOLEAN
    fn create_string() -> String {
        String::from(
        "message_id  INTEGER  PRIMARY KEY, feed_src_id  INTEGER, title  BLOB, post_id  text,  link  text, \
		is_deleted BOOLEAN, is_read BOOLEAN , fetch_date  INTEGER , entry_src_date INTEGER,   \
	 	content_text  BLOB, enclosure_url  text, author BLOB, categories BLOB,  \
		markers INTEGER  	" )
    }

    fn create_indices() -> Vec<String> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_id ON messages (message_id) ; ".to_string(),
            "CREATE INDEX IF NOT EXISTS idx_feed_src ON messages (feed_src_id) ; ".to_string(),
        ]
    }

    fn index_column_name() -> String {
        "message_id".to_string()
    }

    fn get_insert_columns(&self) -> Vec<String> {
        vec![
            String::from("feed_src_id"), // 1
            String::from("title"),
            String::from("post_id"),
            String::from("link"),
            String::from("is_deleted"), // 5
            String::from("is_read"),
            String::from("fetch_date"),
            String::from("entry_src_date"),
            String::from("content_text"),
            String::from("enclosure_url"), // 10
            String::from("author"),
            String::from("categories"),
            String::from("markers"),
        ]
    }

    fn get_insert_values(&self) -> Vec<Wrap> {
        vec![
            Wrap::INT(self.subscription_id), // 1
            Wrap::STR(self.title.clone()),
            Wrap::STR(self.post_id.clone()),
            Wrap::STR(self.link.clone()),
            Wrap::BOO(self.is_deleted), // 5
            Wrap::BOO(self.is_read),
            Wrap::I64(self.fetch_date),
            Wrap::I64(self.entry_src_date),
            Wrap::STR(self.content_text.clone()),
            Wrap::STR(self.enclosure_url.clone()), // 10
            Wrap::STR(self.author.clone()),
            Wrap::STR(self.categories.clone()),
            Wrap::U64(self.markers),
        ]
    }

    fn from_row(row: &rusqlite::Row) -> Self {
        MessageRow {
            message_id: row.get(0).unwrap(),
            subscription_id: row.get(1).unwrap(),
            title: row.get(2).unwrap(),
            post_id: row.get(3).unwrap(),
            link: row.get(4).unwrap(),
            is_deleted: row.get(5).unwrap(),
            is_read: row.get(6).unwrap(),
            fetch_date: row.get(7).unwrap(),
            entry_src_date: row.get(8).unwrap(),
            content_text: row.get(9).unwrap(),
            enclosure_url: row.get(10).unwrap(),
            author: row.get(11).unwrap(),
            categories: row.get(12).unwrap(),
            markers: row.get(13).unwrap(),
            ..Default::default()
        }
    }

    fn get_index_value(&self) -> isize {
        self.message_id
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn t_decompress() {
        setup();
        let input = "8AFSU1MgVHV0b3JpYWwgdHdv";
        assert_eq!(decompress(input).as_str(), "RSS Tutorial two");
    }

    fn setup() {}
}
