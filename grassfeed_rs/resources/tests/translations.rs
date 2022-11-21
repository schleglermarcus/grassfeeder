#[macro_use]
extern crate rust_i18n;

i18n!("../resources/locales");

#[cfg(test)]
mod t {
    use super::*;

    // cargo watch -s "cargo test  t::basic_translation    -- --exact --nocapture"
    #[test]
    fn basic_translation() {
        rust_i18n::set_locale("en");
        let locale = rust_i18n::locale();
        println!("translated={}  locale={}", t!("M_SETTINGS"), locale);
        assert_eq!(t!("M_SETTINGS"), "Settings".to_string());
    }
}
