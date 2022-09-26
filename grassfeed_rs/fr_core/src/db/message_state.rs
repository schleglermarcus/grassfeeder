use crate::db::message::decompress;
use std::collections::HashMap;

#[derive(Default)]
pub struct MessageStateMap {
    msgmap: HashMap<isize, MessageState>,
}

impl MessageStateMap {
    pub fn insert(
        &mut self,
        msg_id: isize,
        is_read: bool,
        list_pos: isize,
        ts_created: i64,
        title_compressed: String,
    ) {
        let mut st = MessageState {
            gui_list_position: list_pos as isize, // list_position.unwrap_or(-1),
            msg_created_timestamp: ts_created,
            is_read_copy: is_read,
            ..Default::default()
        };
        if !title_compressed.is_empty() {
            st.title_d = decompress(&title_compressed);
        }
        self.msgmap.insert(msg_id, st);
    }

    pub fn get_isread(&self, msg_id: isize) -> bool {
        if let Some(st) = self.msgmap.get(&msg_id) {
            return st.is_read_copy;
        }
        false
    }

    pub fn get_title(&self, msg_id: isize) -> Option<String> {
        if let Some(st) = self.msgmap.get(&msg_id) {
            return Some(st.title_d.clone());
        }
        None
    }

    pub fn get_contents_author_categories(
        &self,
        msg_id: isize,
    ) -> Option<(String, String, String)> {
        if let Some(st) = self.msgmap.get(&msg_id) {
            return st.contents_author_categories_d.clone();
        }
        None
    }

    // msg-id,  timestamp
    pub fn get_highest_created_timestamp(&self) -> (isize, i64) {
        let mut highest_created_timestamp: i64 = 0; // Most Recent
        let mut highest_ts_repo_id: isize = -1;
        self.msgmap.iter().for_each(|(fc_id, fc_state)| {
            if fc_state.msg_created_timestamp > highest_created_timestamp {
                highest_created_timestamp = fc_state.msg_created_timestamp;
                highest_ts_repo_id = *fc_id;
            }
        });
        // debug!(            "mostRecent={} {}",            highest_ts_repo_id, highest_created_timestamp        );
        (highest_ts_repo_id, highest_created_timestamp)
    }

    pub fn contains(&self, msg_id: isize) -> bool {
        self.msgmap.contains_key(&msg_id)
    }

    pub fn set_read_many(&mut self, msg_ids: &Vec<i32>, is_read: bool) {
        self.msgmap
            .iter_mut()
            .filter(|(id, _st)| msg_ids.contains(&((**id) as i32)))
            .for_each(|(_id, st)| st.is_read_copy = is_read);
    }

    pub fn set_contents_author_categories(
        &mut self,
        msg_id: isize,
        co_au_ca: &(String, String, String),
    ) {
        if let Some(st) = self.msgmap.get_mut(&msg_id) {
            st.contents_author_categories_d.replace(co_au_ca.clone());
        }
    }

    pub fn clear(&mut self) {
        self.msgmap.clear();
    }
}

#[derive(Default, Debug, Clone)]
pub struct MessageState {
    /// remember  list position for setting the cursor
    pub gui_list_position: isize,
    pub msg_created_timestamp: i64,
    pub is_read_copy: bool,
    pub contents_author_categories_d: Option<(String, String, String)>,

    /// display text, decompressed
    pub title_d: String,
}
