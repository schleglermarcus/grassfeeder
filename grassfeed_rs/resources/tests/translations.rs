#[macro_use]
extern crate rust_i18n;

// i18n!("../resources/locales");
i18n!("/locales");

#[cfg(test)]
mod t {
    use super::*;

    // cargo watch -s "cargo test  t::basic_translation    -- --exact --nocapture"
    #[test]
    fn basic_translation() {
        rust_i18n::set_locale("en");
        let locale = rust_i18n::locale();

        let translated = t!("M_SETTINGS");
        println!("translated={}  locale={}", translated, locale);
        assert_eq!(translated, "Settings".to_string());
    }
}
