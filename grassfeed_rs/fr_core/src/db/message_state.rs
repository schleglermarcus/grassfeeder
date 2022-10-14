use crate::db::message::decompress;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct MessageState {
    /// remember  list position for setting the cursor
    pub gui_list_position: isize,
    pub msg_created_timestamp: i64,
    pub is_read_copy: bool,
    pub contents_author_categories_d: Option<(String, String, String)>,
    /// display text, decompressed
    pub title_d: String,
    pub msg_id: isize,
    pub subscription_id_copy: isize,
}

impl std::fmt::Display for MessageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ts_crea = crate::util::db_time_to_display(self.msg_created_timestamp);
        let isread = if self.is_read_copy { 1 } else { 0 };
        write!(
            f,
            "ID{}  Pos {}  isread {} '{}' created:{}   C_A_C:{:?}  )",
            self.msg_id,
            self.gui_list_position,
            isread,
            self.title_d,
            ts_crea,
            &self.contents_author_categories_d
        )
    }
}

#[derive(Default)]
pub struct MessageStateMap {
    msgmap: HashMap<isize, MessageState>,
}

impl MessageStateMap {
    pub fn insert(
        &mut self,
        msg_id_: isize,
        is_read: bool,
        list_pos: isize,
        ts_created: i64,
        title_compressed: String,
        subs_id: isize,
    ) {
        let mut st = MessageState {
            gui_list_position: list_pos as isize, // list_position.unwrap_or(-1),
            msg_created_timestamp: ts_created,
            is_read_copy: is_read,
            msg_id: msg_id_,
            subscription_id_copy: subs_id,
            ..Default::default()
        };
        if !title_compressed.is_empty() {
            st.title_d = decompress(&title_compressed);
        }
        self.msgmap.insert(msg_id_, st);
    }

    pub fn get_subscription_ids(&self, msg_ids: &[i32]) -> Vec<isize> {
        let mut subs_ids: Vec<isize> = Vec::default();
        self.msgmap.iter().for_each(|(id, m_state)| {
            if msg_ids.contains(&(*id as i32)) && !subs_ids.contains(&m_state.subscription_id_copy)
            {
                subs_ids.push(m_state.subscription_id_copy);
            }
        });
        subs_ids.sort();
        subs_ids
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

    pub fn set_read_many(&mut self, msg_ids: &[i32], is_read: bool) {
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

    pub fn dump(&self) {
        let mut vals: Vec<MessageState> = self.msgmap.values().cloned().collect();
        vals.sort_by(|a, b| a.msg_id.cmp(&b.msg_id));
        vals.iter().for_each(|v| println!("MS:{}", v));
    }

    /// Searches the next unread message.
    ///  message-id,  seek-to:  true:higher timestamps, false: lower_timestamps
    pub fn find_unread_message(&self, current_msg_id: isize, seek_to_later: bool) -> Option<isize> {
        if !self.msgmap.contains_key(&current_msg_id) {
            return None;
        }
        let mut vals: Vec<MessageState> = self.msgmap.values().cloned().collect();
        vals.sort_by(|a, b| b.msg_created_timestamp.cmp(&a.msg_created_timestamp));
        let mut current_index: isize = -1;
        for (a, msg) in vals.iter().enumerate() {
            if msg.msg_id == current_msg_id {
                current_index = a as isize;
            }
        }
        let mut new_index: isize = current_index as isize;
        if seek_to_later {
            while new_index < vals.len() as isize && vals[new_index as usize].is_read_copy {
                new_index += 1;
            }
            if new_index >= vals.len() as isize {
                return None;
            }
        } else {
            while new_index >= 0 && vals[new_index as usize].is_read_copy {
                new_index -= 1;
            }
            if new_index < 0 {
                return None;
            }
        }
        if new_index == current_index {
            None
        } else {
            Some(vals[new_index as usize].msg_id)
        }
    }
}

#[cfg(test)]
pub mod t {
    use super::*;

    //cargo watch -s "cargo test db::message_state::t::t_find_unread_message_window  --lib -- --exact --nocapture"
    #[test]
    fn t_find_unread_message_window() {
        let mut msm = MessageStateMap::default();
        let lim = 7;
        for a in 0..lim {
            msm.insert(
                a + 1,
                a > 1 && a < 5,
                a + 100,
                (a as i64) * 10000000,
                String::default(),
                0,
            );
        }
        msm.dump();
        assert_eq!(msm.find_unread_message(4, true), Some(2));
        assert_eq!(msm.find_unread_message(4, false), Some(6));
    }

    //cargo watch -s "cargo test db::message_state::t::t_find_unread_message_simple --lib -- --exact --nocapture"
    #[ignore]
    #[test]
    fn t_find_unread_message_simple() {
        let mut msm = MessageStateMap::default();
        for a in 0..3 {
            msm.insert(
                a + 1,
                true,
                a + 100,
                (a as i64) * 10000000,
                String::default(),
                0,
            );
        }
        //  msm.dump();
        assert_eq!(msm.find_unread_message(0, false), None);
        assert_eq!(msm.find_unread_message(10, true), None);
        assert_eq!(msm.find_unread_message(1, false), None);
        assert_eq!(msm.find_unread_message(1, true), None);
    }
}
