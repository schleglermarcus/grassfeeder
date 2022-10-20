use core::slice::Iter;

pub enum FetchUpdaterIntervalNames {
    None,
    Minutes,
    Hours,
    Days,
}

pub const FOCUS_POLICY_NAMES: [&str; 5] = [
    "",
    "None",
    "LastSelectedMessage",
    "MostRecentMessage",
    "BeforeOldestUnreadMessage",
];

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
