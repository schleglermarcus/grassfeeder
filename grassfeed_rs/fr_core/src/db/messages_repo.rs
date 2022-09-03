use crate::db::message::MessageRow;
use crate::db::sqlite_context::rusqlite_error_to_boxed;
use crate::db::sqlite_context::SqliteContext;
use crate::db::sqlite_context::TableInfo;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;

pub trait IMessagesRepo {
    /// returns index value
    fn insert(&self, entry: &MessageRow) -> Result<i64, Box<dyn std::error::Error>>;

    // returns number of elements ?
    fn insert_tx(&self, e_list: &[MessageRow]) -> Result<i64, Box<dyn std::error::Error>>;

    fn get_by_src_id(&self, src_id: isize) -> Vec<MessageRow>;

    /// returns  the number of read lines for that source id:   -1 for undefined
    fn get_read_sum(&self, src_id: isize) -> isize;

    /// return count of all lines for that source-id.  if not found, returns -1
    fn get_src_sum(&self, src_id: isize) -> isize;

    /// return count of all lines for that source-id
    fn get_all_sum(&self) -> isize;

    fn get_by_index(&self, indexvalue: isize) -> Option<MessageRow>;

    /// returns   feed_src_id  , is_read
    fn get_is_read(&self, repo_id: isize) -> (isize, bool);

    fn get_all_messages(&self) -> Vec<MessageRow>;

    #[deprecated(note = "use update_is_read_many()")]
    fn update_is_read(&self, repo_id: isize, new_is_read: bool);

    fn update_is_read_many(&self, repo_ids: &[i32], new_is_read: bool);
    fn update_is_read_all(&self, source_repo_id: isize, new_is_read: bool);
    /// title string shall be compressed. returns number of lines
    fn update_title(&self, repo_id: isize, new_title_compr: String) -> usize;
    fn update_post_id(&self, repo_id: isize, new_post_id: String) -> usize;
    fn update_entry_src_date(&self, repo_id: isize, n_src_date: i64) -> usize;
    fn update_is_deleted_many(&self, repo_ids: &[i32], new_is_del: bool);

    #[deprecated(note = "use insert()")]
    fn store_entry(&self, entry: &MessageRow) -> Result<MessageRow, Box<dyn std::error::Error>>;

    fn get_ctx(&self) -> &SqliteContext<MessageRow>;

    /// return highest feed_source_id currently stored
    fn get_max_src_index(&self) -> isize;

    fn get_src_not_contained(&self, src_repo_id_list: &[i32]) -> Vec<MessageRow>;
}

pub struct MessagesRepo {
    ctx: SqliteContext<MessageRow>,
}

impl MessagesRepo {
    pub const CONF_DB_KEY: &'static str = "messages_db";

    pub fn new(filename: String) -> Self {
        MessagesRepo {
            ctx: SqliteContext::new(filename),
        }
    }

    pub fn new_by_connection(con_a: Arc<Mutex<Connection>>) -> Self {
        MessagesRepo {
            ctx: SqliteContext::new_by_connection(con_a),
        }
    }

    pub fn new_in_mem() -> Self {
        MessagesRepo {
            ctx: SqliteContext::new_in_memory(),
        }
    }

    pub fn get_ctx(&self) -> &SqliteContext<MessageRow> {
        &self.ctx
    }

    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        self.ctx.get_connection()
    }
}

impl IMessagesRepo for MessagesRepo {
    fn get_ctx(&self) -> &SqliteContext<MessageRow> {
        &self.ctx
    }

    fn insert(&self, entry: &MessageRow) -> Result<i64, Box<dyn std::error::Error>> {
        self.ctx.insert(entry).map_err(rusqlite_error_to_boxed)
    }

    fn insert_tx(&self, e_list: &[MessageRow]) -> Result<i64, Box<dyn std::error::Error>> {

        self.ctx.insert_tx(& e_list.to_vec() ).map_err(rusqlite_error_to_boxed)
    }

    // deprecated
    fn store_entry(&self, _entry: &MessageRow) -> Result<MessageRow, Box<dyn std::error::Error>> {
        unimplemented!()
    }

    fn get_by_src_id(&self, src_id: isize) -> Vec<MessageRow> {
        let prepared = format!(
            "SELECT * FROM {} WHERE feed_src_id={}",
            MessageRow::table_name(),
            src_id
        );
        let mut list: Vec<MessageRow> = Vec::default();
        if let Ok(mut stmt) = (*self.get_connection()).lock().unwrap().prepare(&prepared) {
            match stmt.query_map([], |row| {
                list.push(MessageRow::from_row(row));
                Ok(())
            }) {
                Ok(mr) => {
                    mr.count(); // seems to be necessary
                }
                Err(e) => error!("{} {:?}", &prepared, e),
            }
        }

        list
    }

    /// returns  the number of read lines for that source id:   -1 for undefined
    fn get_read_sum(&self, src_id: isize) -> isize {
        let sql = format!(
            "SELECT COUNT({}) FROM {} WHERE feed_src_id = {} and is_read = true ",
            MessageRow::index_column_name(),
            MessageRow::table_name(),
            src_id
        );
        self.ctx.count(sql)
    }

    fn get_src_sum(&self, src_id: isize) -> isize {
        let sql = format!(
            "SELECT COUNT({}) FROM {} WHERE feed_src_id = {} ",
            MessageRow::index_column_name(),
            MessageRow::table_name(),
            src_id
        );
        self.ctx.count(sql)
    }

    /// return count of all lines for that source-id
    fn get_all_sum(&self) -> isize {
        let sql = format!(
            "SELECT COUNT({}) FROM {} ",
            MessageRow::index_column_name(),
            MessageRow::table_name()
        );
        self.ctx.count(sql)
    }

    fn get_by_index(&self, indexvalue: isize) -> Option<MessageRow> {
        self.ctx.get_by_index(indexvalue)
    }

    /// returns   feed_src_id  , is_read
    fn get_is_read(&self, repo_id: isize) -> (isize, bool) {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = {} ",
            MessageRow::table_name(),
            MessageRow::index_column_name(),
            repo_id
        );
        if let Some(msg) = self.ctx.get_one(sql) {
            (msg.message_id, msg.is_read)
        } else {
            (repo_id, false)
        }
    }

    fn get_all_messages(&self) -> Vec<MessageRow> {
        self.ctx.get_all()
    }

    // deprecated
    fn update_is_read(&self, _repo_id: isize, _new_is_read: bool) {
        unimplemented!()
    }

    fn update_is_read_many(&self, repo_ids: &[i32], new_is_read: bool) {
        let joined = repo_ids
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(
            "UPDATE {}  SET  is_read = {} WHERE {} in ( {} )",
            MessageRow::table_name(),
            new_is_read,
            MessageRow::index_column_name(),
            joined
        );
        self.ctx.execute(sql);
    }

    fn update_is_read_all(&self, source_repo_id: isize, new_is_read: bool) {
        let sql = format!(
            "UPDATE {}  SET  is_read = {} WHERE feed_src_id = {}",
            MessageRow::table_name(),
            new_is_read,
            source_repo_id,
        );
        self.ctx.execute(sql);
    }

    /// title string shall be compressed
    fn update_title(&self, repo_id: isize, new_title: String) -> usize {
        if new_title.contains('"') {
            warn!(
                "update_title({}) may not contain quote char! >{}<",
                repo_id, new_title
            );
            return 0;
        }
        let sql = format!(
            "UPDATE {}  SET  title = \"{}\" WHERE {} = {}",
            MessageRow::table_name(),
            new_title,
            MessageRow::index_column_name(),
            repo_id,
        );
        self.ctx.execute(sql)
    }

    fn update_post_id(&self, repo_id: isize, new_post_id: String) -> usize {
        if new_post_id.contains('"') {
            warn!(
                "update_post_id({}) may not contain quote char! >{}<",
                repo_id, new_post_id
            );
            return 0;
        }
        let sql = format!(
            "UPDATE {}  SET  post_id = \"{}\" WHERE {} = {}",
            MessageRow::table_name(),
            new_post_id,
            MessageRow::index_column_name(),
            repo_id,
        );
        self.ctx.execute(sql)
    }

    fn update_entry_src_date(&self, repo_id: isize, n_src_date: i64) -> usize {
        let sql = format!(
            "UPDATE {}  SET  entry_src_date = \"{}\" WHERE {} = {}",
            MessageRow::table_name(),
            n_src_date,
            MessageRow::index_column_name(),
            repo_id,
        );
        self.ctx.execute(sql)
    }

    fn get_max_src_index(&self) -> isize {
        let sql = format!(
            "SELECT MAX( feed_src_id ) FROM {} ",
            MessageRow::table_name()
        );
        self.ctx.count(sql)
    }

    fn get_src_not_contained(&self, src_repo_id_list: &[i32]) -> Vec<MessageRow> {
        let src_ids_jo = src_repo_id_list
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");

        let sql = format!(
            "SELECT * FROM {} where feed_src_id NOT IN ( {} ) and is_deleted = false ",
            MessageRow::table_name(),
            src_ids_jo
        );
        self.ctx.get_list(sql)
    }

    fn update_is_deleted_many(&self, repo_ids: &[i32], new_is_del: bool) {
        let joined = repo_ids
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let sql = format!(
            "UPDATE {}  SET  is_deleted = {} WHERE {} in ( {} )",
            MessageRow::table_name(),
            new_is_del,
            MessageRow::index_column_name(),
            joined
        );
        self.ctx.execute(sql);
    }
}

impl Buildable for MessagesRepo {
    type Output = MessagesRepo;
    fn build(conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        match conf.get(MessagesRepo::CONF_DB_KEY) {
            Some(filename) => MessagesRepo::new(filename),
            None => {
                panic!(
                    "No database location from config!  {}  Stopping",
                    MessagesRepo::CONF_DB_KEY
                );
            }
        }
    }

    fn section_name() -> String {
        String::from("messagesrepo")
    }
}

use context::StartupWithAppContext;
impl StartupWithAppContext for MessagesRepo {
    fn startup(&mut self, _ac: &AppContext) {
        self.ctx.create_table();
    }
}

// ---------------------------------------------

#[cfg(test)]
mod t {

    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    // #[ignore]
    #[test]
    fn t_get_src_not_contained() {
        setup();
        let msgrepo_r = prepare_3_rows();
        let mut e1 = MessageRow::default();
        e1.feed_src_id = 1;
        let _r = (*msgrepo_r).borrow().insert(&e1);
        // let all = (*msgrepo_r).borrow().get_all_messages();	    for msg in all {	        trace!("{:?}", msg);	    }
        let src_not: Vec<i32> = vec![0, 3];
        let msg_not = (*msgrepo_r).borrow().get_src_not_contained(&src_not);
        // for msg in & msg_not {	        debug!(" NOT {:?}", msg);	    }
        assert_eq!(msg_not.len(), 1);
        assert_eq!(msg_not.get(0).unwrap().feed_src_id, 1);
    }

    #[test]
    fn t_get_max_src_index_existing() {
        let msgrepo_r = prepare_3_rows();
        let maxindex = (*msgrepo_r).borrow().get_max_src_index();
        assert_eq!(maxindex, 3);
    }

    //cargo watch -s "cargo test  db::messages_repo::t::t_get_max_src_index_empty   --lib -- --exact --nocapture "
    #[test]
    fn t_get_max_src_index_empty() {
        setup();
        let messagesrepo = MessagesRepo::new(":memory:".to_string());
        messagesrepo.get_ctx().create_table();
        let maxindex = messagesrepo.get_max_src_index();
        assert_eq!(maxindex, -1);
    }

    #[test]
    fn t_store_entries_add() {
        let msg_r = prepare_3_rows();
        let insert = vec![MessageRow::default(), MessageRow::default()];
        let r = (*msg_r).borrow().insert_tx(&insert);
        assert!(r.is_ok());
        assert_eq!(r.unwrap() as usize, insert.len());
        let list = (*msg_r).borrow().get_all_messages();
        assert_eq!(list.len(), 5);
    }

    #[test]
    fn t_insert() {
        let msg_r = prepare_3_rows();
        let insert = MessageRow::default();
        let r = (*msg_r).borrow().insert(&insert);
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), 4);
        let list = (*msg_r).borrow().get_all_messages();
        assert_eq!(list.len(), 4);
    }

    #[test]
    fn t_update_entry_src_date() {
        let msg_r = prepare_3_rows();

        (*msg_r).borrow().update_entry_src_date(1, 11);

        assert_eq!(
            (*msg_r).borrow().get_by_index(1).unwrap().entry_src_date,
            11
        );
    }

    #[test]
    fn t_update_post_id() {
        let msg_r = prepare_3_rows();
        assert_eq!(
            (*msg_r).borrow().update_post_id(1, "some_id".to_string()),
            1
        );
        //let list = (*msg_r).borrow().get_all_messages();    debug!("{:#?}", list);
        assert_eq!(
            (*msg_r).borrow().get_by_index(1).unwrap().post_id.as_str(),
            "some_id"
        );
        assert_eq!(
            (*msg_r)
                .borrow()
                .update_post_id(2, "\" delete ".to_string()),
            0
        );
    }

    #[test]
    fn t_update_title() {
        let msg_r = prepare_3_rows();
        let titles: [&str; 5] = [
            "hello",
            "Japan 無料ダウンロード",
            "korean 기사 요약 -",
            ") delete",
            "\' delete ",
        ];
        for t in titles {
            (*msg_r).borrow().update_title(1, t.to_string());
            assert_eq!((*msg_r).borrow().get_by_index(1).unwrap().title.as_str(), t);
        }
        assert_eq!(
            (*msg_r).borrow().update_title(2, "\" delete ".to_string()),
            0
        );
        assert_eq!(
            (*msg_r).borrow().get_by_index(2).unwrap().title.as_str(),
            ""
        );
    }

    fn prepare_3_rows() -> Rc<RefCell<dyn IMessagesRepo>> {
        setup();
        let messagesrepo = MessagesRepo::new(":memory:".to_string());
        messagesrepo.get_ctx().create_table();
        let msg_r: Rc<RefCell<dyn IMessagesRepo>> = Rc::new(RefCell::new(messagesrepo));
        let mut e1 = MessageRow::default();
        let _r = (*msg_r).borrow().insert(&e1);
        e1.feed_src_id = 3;
        let _r = (*msg_r).borrow().insert(&e1);
        e1.feed_src_id = 3;
        e1.is_read = true;
        let _r = (*msg_r).borrow().insert(&e1);
        msg_r
    }

    #[test]
    fn t_update_is_read_all() {
        let msg_r = prepare_3_rows();
        (*msg_r).borrow().update_is_read_all(3, true);
        let list = (*msg_r).borrow().get_all_messages();
        assert_eq!(list.get(0).unwrap().is_read, false);
        assert_eq!(list.get(1).unwrap().is_read, true);
        assert_eq!(list.get(2).unwrap().is_read, true);
        //    assert_eq!((*msg_r).borrow().get_is_read(3), (3, true));
    }

    #[test]
    fn t_update_is_read_many() {
        let msg_r = prepare_3_rows();
        let repo_ids = vec![2, 3];
        (*msg_r).borrow().update_is_read_many(&repo_ids, true);
        let list = (*msg_r).borrow().get_all_messages();
        assert_eq!(list.get(0).unwrap().is_read, false);
        assert_eq!(list.get(1).unwrap().is_read, true);
        assert_eq!(list.get(2).unwrap().is_read, true);
        //    assert_eq!((*msg_r).borrow().get_is_read(3), (3, true));
    }

    #[test]
    fn t_get_is_read() {
        let msg_r = prepare_3_rows();
        assert_eq!((*msg_r).borrow().get_is_read(2), (2, false));
        assert_eq!((*msg_r).borrow().get_is_read(3), (3, true));
    }

    #[test]
    fn t_get_read_sum() {
        let msg_r = prepare_3_rows();
        let num = (*msg_r).borrow().get_read_sum(3);
        assert_eq!(num, 1);
    }

    #[test]
    fn t_get_src_sum() {
        let msg_r = prepare_3_rows();
        let num = (*msg_r).borrow().get_src_sum(3);
        assert_eq!(num, 2);
    }

    #[test]
    fn t_get_all_sum() {
        let msg_r = prepare_3_rows();
        let num = (*msg_r).borrow().get_all_sum();
        assert_eq!(num, 3);
    }

    #[test]
    fn t_get_by_src_id() {
        let msg_r = prepare_3_rows();
        let list = (*msg_r).borrow().get_by_src_id(3);
        assert_eq!(list.len(), 2);
        assert_eq!(list.get(0).unwrap().message_id, 2);
    }

    //RUST_BACKTRACE=1 cargo watch -s "cargo test  db::messages_repo::t::t_insert_get_row   --lib -- --exact --nocapture "
    // #[ignore]
    #[test]
    fn t_insert_get_row() {
        setup();
        let messagesrepo = MessagesRepo::new(":memory:".to_string());
        messagesrepo.get_ctx().create_table();
        let mut e1 = MessageRow::default();
        let r1 = messagesrepo.get_ctx().insert(&e1);
        assert!(r1.is_ok());
        e1.feed_src_id = 3;
        e1.title = "title3".to_string();
        e1.post_id = "47".to_string();
        e1.link = "link47".to_string();
        e1.is_deleted = true;
        e1.is_read = true;
        e1.fetch_date = 22;
        e1.entry_src_date = 33;
        e1.content_text = "select content".to_string();
        e1.enclosure_url = "delete enclosure".to_string();
        e1.author = "from authorized".to_string();
        e1.categories = "cat1 cat2".to_string();
        let r2 = messagesrepo.get_ctx().insert(&e1);
        assert!(r2.is_ok());
        let e1 = messagesrepo.get_ctx().get_by_index(1);
        assert!(e1.is_some());
        let e2 = messagesrepo.get_ctx().get_by_index(2).unwrap();
        //debug!("E2={:?} ", e2);
        assert_eq!(e2.message_id, 2);
        assert_eq!(e2.feed_src_id, 3);
        assert_eq!(e2.title.as_str(), "title3");
        assert_eq!(e2.post_id.as_str(), "47");
        assert_eq!(e2.link.as_str(), "link47");
        assert_eq!(e2.is_deleted, true);
        assert_eq!(e2.is_read, true);
        assert_eq!(e2.fetch_date, 22);
        assert_eq!(e2.entry_src_date, 33);
        assert_eq!(e2.content_text.as_str(), "select content");
        assert_eq!(e2.enclosure_url.as_str(), "delete enclosure");
        assert_eq!(e2.author.as_str(), "from authorized");
        assert_eq!(e2.categories.as_str(), "cat1 cat2");
    }

    fn setup() {} // dummy
}
