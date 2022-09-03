const LOCALES_LIST: [&str; 2] = ["en", "de"];

pub fn init_locales() {
    if let Ok(lang) = std::env::var("LANG") {
        LOCALES_LIST.iter().for_each(|l| {
            if lang.starts_with(l) {
                rust_i18n::set_locale(l);
            }
        });
        //  println!("{key}: {val:?}"),    Err(e) => println!("couldn't interpret {key}: {e}"),
    }
}
