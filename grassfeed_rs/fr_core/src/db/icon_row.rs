use super::sqlite_context::TableInfo;
use super::sqlite_context::Wrap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconRow {
    pub web_date: i64,
    pub req_date: i64,
    pub icon_id: isize,
    pub web_size: isize,
    /// 0: raw, from web
    /// 1: gtk-image
    /// 2: image-rs
    /// 3: png
    pub compression_type: u8,
    pub web_url: String,
    pub icon: String,
}

impl TableInfo for IconRow {
    fn table_name() -> String {
        "icons".to_string()
    }

    // https://www.tutorialspoint.com/sqlite/sqlite_data_types.htm
    // INTEGER REAL  TEXT  BLOB		BOOLEAN
    fn create_string() -> String {
        String::from(
            "icon_id  INTEGER  PRIMARY KEY, web_url  TEXT, web_size  INTEGER,  \
            web_date INTEGER,  req_date INTEGER, compression_type INTEGER, icon TEXT ",
        )
    }

    fn create_indices() -> Vec<String> {
        vec!["CREATE INDEX IF NOT EXISTS idx_id ON icons (icon_id) ; ".to_string()]
    }

    fn index_column_name() -> String {
        "icon_id".to_string()
    }

    fn get_insert_columns(&self) -> Vec<String> {
        vec![
            String::from("icon_id"), // 1
            String::from("web_url"),
            String::from("web_size"),
            String::from("web_date"),
            String::from("req_date"), // 5
            String::from("compression_type"),
            String::from("icon"),
        ]
    }

    fn get_insert_values(&self) -> Vec<Wrap> {
        vec![
            Wrap::INT(self.icon_id), // 1
            Wrap::STR(self.web_url.clone()),
            Wrap::INT(self.web_size),
            Wrap::I64(self.web_date),
            Wrap::I64(self.req_date), // 5
            Wrap::INT(self.compression_type as isize),
            Wrap::STR(self.icon.clone()),
        ]
    }

    fn from_row(row: &rusqlite::Row) -> Self {
        IconRow {
            icon_id: row.get(0).unwrap(),
            web_url: row.get(1).unwrap(),
            web_size: row.get(2).unwrap(),
            web_date: row.get(3).unwrap(),
            req_date: row.get(4).unwrap(),
            compression_type: row.get(5).unwrap(),
            icon: row.get(6).unwrap(),
            ..Default::default()
        }
    }

    fn get_index_value(&self) -> isize {
        self.icon_id
    }
}
