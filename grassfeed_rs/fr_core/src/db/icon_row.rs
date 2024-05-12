use super::sqlite_context::TableInfo;
use super::sqlite_context::Wrap;

// #[repr(u8)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CompressionType {
    #[default]
    None = 0,
    Web,
    GtkImage,
    ImageRs,
    Png,
}

impl CompressionType {
    fn from_isize(i: isize) -> Self {
        match i {
            1 => Self::Web,
            2 => Self::GtkImage,
            3 => Self::ImageRs,
            4 => Self::Png,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct IconRow {
    pub web_date: i64,
    pub req_date: i64,
    pub icon_id: isize,
    pub web_size: isize,
    /// 0: raw, from web
    /// 1: gtk-image
    /// 2: image-rs
    /// 3: png
    pub compression_type: CompressionType, // u8,
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
        vec![
            "CREATE INDEX IF NOT EXISTS idx_id ON icons (icon_id) ; ".to_string(),
            "CREATE INDEX IF NOT EXISTS idx_icon ON icons (icon) ; ".to_string(),
        ]
    }

    fn index_column_name() -> String {
        "icon_id".to_string()
    }

    fn get_insert_columns(&self) -> Vec<String> {
        vec![
            String::from("web_url"),
            String::from("web_size"),
            String::from("web_date"),
            String::from("req_date"),
            String::from("compression_type"),
            String::from("icon"), // 5
        ]
    }

    fn get_insert_values(&self) -> Vec<Wrap> {
        vec![
            Wrap::STR(self.web_url.clone()),
            Wrap::INT(self.web_size),
            Wrap::I64(self.web_date),
            Wrap::I64(self.req_date),
            Wrap::INT(self.compression_type.clone() as isize),
            Wrap::STR(self.icon.clone()), // 5
        ]
    }

    fn from_row(row: &rusqlite::Row) -> Self {
        let i_5: isize = row.get(5).unwrap();
        IconRow {
            icon_id: row.get(0).unwrap(),
            web_url: row.get(1).unwrap(),
            web_size: row.get(2).unwrap(),
            web_date: row.get(3).unwrap(),
            req_date: row.get(4).unwrap(),
            compression_type: CompressionType::from_isize(i_5),
            icon: row.get(6).unwrap(),
            ..Default::default()
        }
    }

    fn get_index_value(&self) -> isize {
        self.icon_id
    }
}

impl std::fmt::Debug for IconRow {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("IR")
            .field("id", &self.icon_id)
            .field("req_date", &self.req_date)
            .field("web_date", &self.web_date)
            .field("web_size", &self.web_size)
            .field("compr", &self.compression_type)
            .field("#icon", &self.icon.len())
            .finish()
    }
}
