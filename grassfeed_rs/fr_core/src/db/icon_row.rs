use super::sqlite_context::TableInfo;
use super::sqlite_context::Wrap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconRow {
    pub icon_id: isize,
    pub web_url: String,
    pub web_size: isize,
    pub web_date: isize,
    /// 0: raw, from web
    /// 1: gtk-image
    /// 2: image-rs
    /// 3: png
    pub compression_type: u8,
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
            "icon_id  INTEGER  PRIMARY KEY, web_url  text, web_size  INTEGER,  \
            web_date INTEGER, compression_type INTEGER, icon: text ",
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
            String::from("compression_type"), // 5
            String::from("icon"),
        ]
    }

    fn get_insert_values(&self) -> Vec<Wrap> {
        vec![
            Wrap::INT(self.icon_id), // 1
            Wrap::STR(self.web_url.clone()),
            Wrap::INT(self.web_size),
            Wrap::INT(self.web_date),
            Wrap::INT(self.compression_type as isize), // 5
            Wrap::STR(self.icon.clone()),
        ]
    }

    fn from_row(row: &rusqlite::Row) -> Self {
        IconRow {
            icon_id: row.get(0).unwrap(),
            web_url: row.get(1).unwrap(),
            web_size: row.get(2).unwrap(),
            web_date: row.get(3).unwrap(),
            compression_type: row.get(4).unwrap(),
            icon: row.get(5).unwrap(),
            ..Default::default()
        }
    }

    fn get_index_value(&self) -> isize {
        self.icon_id
    }
}
