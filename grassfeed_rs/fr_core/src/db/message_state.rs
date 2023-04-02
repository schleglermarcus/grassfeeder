use crate::db::message::decompress;
use crate::util::db_time_to_display_nonnull;
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct MessageState {
    pub msg_id: isize,
    /// remember  list position for setting the cursor
    pub gui_list_position: isize,
    pub msg_created_timestamp: i64,
    pub is_read_copy: bool,
    pub contents_author_categories_d: Option<(String, String, String)>,
    /// display text, decompressed
    pub title_d: String,
    pub subscription_id_copy: isize,
}

impl std::fmt::Display for MessageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ts_crea = crate::util::db_time_to_display(self.msg_created_timestamp);
        let isread = i32::from(self.is_read_copy); //  if self.is_read_copy { 1 } else { 0 };
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
    /// message-id  , MessageState
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
            gui_list_position: list_pos,
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

    pub fn get_gui_pos(&self, msg_id: isize) -> u32 {
        if let Some(st) = self.msgmap.get(&msg_id) {
            return st.gui_list_position as u32;
        }
        0
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

    ///  Message with the latest Timestemp.
    ///  Returns    Latest-msg-id,  Latest-timestamp,  Earliest-msg-id, earliest-Timestamp
    pub fn find_latest_earliest_created_timestamp(&self) -> (isize, i64, isize, i64) {
        let mut highest_created_timestamp: i64 = 0; // Most Recent
        let mut highest_ts_repo_id: isize = -1;
        let mut earliest_created_timestamp: i64 = i64::MAX; // least recent
        let mut earliest_ts_repo_id: isize = -1;
        self.msgmap.iter().for_each(|(fc_id, fc_state)| {
            if fc_state.msg_created_timestamp > highest_created_timestamp {
                highest_created_timestamp = fc_state.msg_created_timestamp;
                highest_ts_repo_id = *fc_id;
            };
            if fc_state.msg_created_timestamp < earliest_created_timestamp {
                earliest_created_timestamp = fc_state.msg_created_timestamp;
                earliest_ts_repo_id = *fc_id;
            };
        });
        (
            highest_ts_repo_id,
            highest_created_timestamp,
            earliest_ts_repo_id,
            earliest_created_timestamp,
        )
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
        vals.iter().for_each(|v| println!("MS:{v}"));
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
        let mut new_index: isize = current_index;
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




    /// Searches the message before the oldest unread
    pub fn find_before_earliest_unread(&self) -> Option<isize> {
        if self.msgmap.is_empty() {
            debug!("MSGSTATE:  msgmap empty!");
            return None;
        }
        let mut vals: Vec<MessageState> = self.msgmap.values().cloned().collect();
        vals.sort_by(|a, b| a.msg_created_timestamp.cmp(&b.msg_created_timestamp));
        let mut new_index: isize = 0;
        while new_index < vals.len() as isize && vals[new_index as usize].is_read_copy {
            new_index += 1;
        }

/// TODO choose the last entry anyway, even if it's marked

        if new_index > 0_isize {
            return Some(vals[(new_index - 1) as usize].msg_id);
        } else {
            debug!(
                "find_before_earliest_unread:  index==0  #vals={} #val0.date={:?}",
                vals.len(),
                db_time_to_display_nonnull(vals.get(0).unwrap().msg_created_timestamp)
            );
        }
        None
    }

    /// returns the next message-id relative to the one to be deleted.
    ///    message-ids
    ///  Returns :    message-id,  gui-list-pos
    pub fn find_neighbour_message(&self, del_msg_ids: &[i32]) -> Option<(isize, isize)> {
        let mut vals: Vec<MessageState> = self.msgmap.values().cloned().collect();
        vals.sort_by(|a, b| a.msg_created_timestamp.cmp(&b.msg_created_timestamp));
        let mut current_index: isize = -1;
        let mut low_gui_list_pos: isize = -1;
        for (num, sta) in vals.iter().enumerate() {
            if del_msg_ids.contains(&(sta.msg_id as i32)) {
                if low_gui_list_pos < 0 || sta.gui_list_position > low_gui_list_pos {
                    low_gui_list_pos = sta.gui_list_position;
                }
                if current_index < 0 || (num as isize) < current_index {
                    current_index = num as isize;
                }
            }
        }
        if low_gui_list_pos > 0 {
            low_gui_list_pos -= 1;
        }
        match current_index.cmp(&0) {
            Ordering::Greater => current_index -= 1,
            Ordering::Less => {
                if let Some(sta) = vals
                    .iter()
                    .find(|sta| sta.gui_list_position == low_gui_list_pos)
                {
                    return Some((sta.msg_id, low_gui_list_pos));
                }
                return None;
            }
            _ => (),
        }
        let neighbour_id = vals[current_index as usize].msg_id;
        Some((neighbour_id, low_gui_list_pos))
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
        // msm.dump();
        assert_eq!(msm.find_unread_message(4, true), Some(2));
        assert_eq!(msm.find_unread_message(4, false), Some(6));
    }

    //cargo watch -s "cargo test db::message_state::t::t_find_unread_message_simple --lib -- --exact --nocapture"
    // #[ignore]
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

    //cargo watch -s "cargo test db::message_state::t::t_find_before_last_unread  --lib -- --exact --nocapture"
    #[test]
    fn t_find_before_last_unread() {
        let mut msm = MessageStateMap::default();
        let lim = 7;
        for a in 0..lim {
            msm.insert(
                a + 1,
                (a < 2) || (a > 3 && a < 5),
                a + 100,
                (a as i64) * 10000000,
                String::default(),
                0,
            );
        }
        let o_last_read = msm.find_before_earliest_unread();
        assert_eq!(o_last_read, Some(2));
    }

    //cargo watch -s "cargo test db::message_state::t::t_find_neighbour_message  --lib -- --exact --nocapture"
    #[test]
    fn t_find_neighbour_message() {
        let mut msm = MessageStateMap::default();
        let lim = 7;
        for a in 0..lim {
            msm.insert(
                a + 1,
                (a < 2) || (a > 3 && a < 5),
                a + 100,
                (1 + a as i64) * 10000000,
                String::default(),
                0,
            );
        }
        let o_neigh = msm.find_neighbour_message(&[4, 5]);
        assert_eq!(o_neigh, Some((3, 103)))
    }

    //
}
