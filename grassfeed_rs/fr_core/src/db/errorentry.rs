use crate::db::sqlite_context::TableInfo;
use crate::db::sqlite_context::Wrap;
use crate::util::db_time_to_display;
use serde::Deserialize;
use serde::Serialize;

// pub const ESRC_DEFAULT: isize = 0;
pub enum ESRC {
    NONE = 0,
    GpDlFinished = 1,
    SubsmoveTruncated = 2,
    DragEvalstart = 3,
    IconsFeedtext = 4,
    IconsAHEx = 5,
    IconsAHMain = 6,
    IconsDownload = 7,
    IconsCheckimg= 8,
    IconsDownscale= 9,
    MsgDlStart= 10,
    MsgEvalFltEmpty= 11,
    MsgEvalFltStrange= 12,
}

///
/// List of Errors
///
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorEntry {
    pub err_id: isize,
    pub subs_id: isize,
    pub e_src: isize,
    pub e_val: isize,
    pub date: i64,
    pub remote_address: String,
    pub text: String,
}

impl std::fmt::Debug for ErrorEntry {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("ErrorE")
            .field("ID", &self.err_id)
            .field("subs_id", &self.subs_id)
            .field("e_src", &self.e_src)
            .field("e_val", &self.e_val)
            .field("date", &db_time_to_display(self.date))
            .field("text", &self.text)
            .field("url", &self.remote_address)
            .finish()
    }
}

/*
impl ErrorEntry {
    pub fn to_line(&self, display_name: String) -> String {
        let mut disp = display_name;
        disp.truncate(30);
        let mut e_text = self.text.clone();
        e_text.truncate(50);
        let mut e_remot = self.remote_address.clone();
        e_remot.truncate(40);
        format!(
            "{:20} {:16} {:4} {:50} {:40}",
            disp,
            db_time_to_display(self.date),
            self.err_code,
            e_text,
            e_remot,
        )
    }
}
 */

impl TableInfo for ErrorEntry {
    fn table_name() -> String {
        "errors".to_string()
    }

    // https://www.tutorialspoint.com/sqlite/sqlite_data_types.htm
    // INTEGER REAL  TEXT  BLOB		BOOLEAN
    fn create_string() -> String {
        String::from(
            "err_id  INTEGER  PRIMARY KEY, subs_id  INTEGER, e_src  INTEGER,  e_val  INTEGER , 
              date  INTEGER,   remote_address TEXT, err_text TEXT ",
        )
    }

    fn create_indices() -> Vec<String> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_err_id  ON errors (err_id) ; ".to_string(),
            "CREATE INDEX IF NOT EXISTS idx_subs_id ON errors (subs_id) ; ".to_string(),
        ]
    }

    fn index_column_name() -> String {
        "err_id".to_string()
    }

    fn get_insert_columns(&self) -> Vec<String> {
        vec![
            String::from("subs_id"), // 1
            String::from("date"),
            String::from("e_src"),
            String::from("e_val"),
            String::from("remote_address"), // 5
            String::from("err_text"),
        ]
    }

    fn get_insert_values(&self) -> Vec<Wrap> {
        vec![
            Wrap::INT(self.subs_id),
            Wrap::I64(self.date),
            Wrap::INT(self.e_src),
            Wrap::INT(self.e_val),
            Wrap::STR(self.remote_address.clone()), // 5
            Wrap::STR(self.text.clone()),
        ]
    }

    fn from_row(row: &rusqlite::Row) -> Self {
        ErrorEntry {
            err_id: row.get(0).unwrap(),
            subs_id: row.get(1).unwrap(),
            e_src: row.get(2).unwrap(),
            e_val: row.get(3).unwrap(),
            date: row.get(4).unwrap(),
            remote_address: row.get(5).unwrap(),
            text: row.get(6).unwrap(),
            ..Default::default()
        }
    }

    fn get_index_value(&self) -> isize {
        self.err_id
    }
}