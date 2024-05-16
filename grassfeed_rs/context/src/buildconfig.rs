use crate::BuildConfig;
use log::debug;
use std::collections::HashMap;

#[derive(Default)]
pub struct BuildConfigContainer {
    pub map: HashMap<String, String>,
}

impl BuildConfig for BuildConfigContainer {
    fn get(&self, key: &str) -> Option<String> {
        self.map.get(key).cloned()
    }

    fn get_int(&self, key: &str) -> Option<isize> {
        if let Some(r) = self.map.get(key).map(|s| s.parse::<isize>()) {
            return r.ok();
        }
        None
    }

    fn get_bool(&self, key: &str) -> bool {
        if let Some(Ok(v)) = self.map.get(key).map(|s| s.parse::<bool>()) {
            return v;
        }
        false
    }

    fn dump(&self) {
        debug!("BuildConfig: {:?}", self.map);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn buildconfigcontainertest() {
        let mut bcc = BuildConfigContainer::default();
        bcc.map
            .insert(String::from("dbkey1"), String::from("String1"));
        bcc.map.insert(String::from("dbkey2"), String::from("-7"));
        assert_eq!(bcc.get("dbkey1"), Some(String::from("String1")));
        assert_eq!(bcc.get_int("dbkey2"), Some(-7));
        assert_eq!(bcc.get_int("dbkey1"), None);
    }
}
