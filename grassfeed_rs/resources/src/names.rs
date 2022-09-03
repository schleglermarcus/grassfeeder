use core::slice::Iter;

pub enum FetchUpdaterIntervalNames {
    None,
    Minutes,
    Hours,
    Days,
}


#[deprecated]
pub const FETCH_UPDATER_INTERVAL_NAMES: [&str; 4] = ["", "Minutes", "Hours", "Days"];

/*
#[deprecated]
pub fn get_fetch_updater_interval_name(num: i32) -> &'static str {
    if num >= FETCH_UPDATER_INTERVAL_NAMES.len() as i32 {
        return "";
    }
    FETCH_UPDATER_INTERVAL_NAMES[num as usize]
}

#[deprecated]
pub fn get_fetch_interval(name: &str) -> i32 {
    resolve(name, FETCH_UPDATER_INTERVAL_NAMES.iter())
}

*/
pub const FOCUS_POLICY_NAMES: [&str; 5] = [
    "",
    "None",
    "Last Selected Message",
    "Most Recent Message",
    "Before Oldest Unread Message",
];

/*
#[deprecated]
pub fn get_focus_policy_num(name: &str) -> u32 {
    resolve(name, FOCUS_POLICY_NAMES.iter()) as u32
}

#[deprecated]
pub fn get_focus_policy_name(num: u32) -> &'static str {
    if num >= FOCUS_POLICY_NAMES.len() as u32 {
        return "";
    }
    FOCUS_POLICY_NAMES[num as usize]
}
*/

pub fn resolve(name: &str, list_iter: Iter<'_, &str>) -> i32 {
    if let Some(n) = list_iter
        .enumerate()
        .filter(|(_n, s)| **s == name)
        .map(|(n, _s)| n)
        .next()
    {
        return n as i32;
    }
    0
}

#[cfg(test)]
mod names_test {}
