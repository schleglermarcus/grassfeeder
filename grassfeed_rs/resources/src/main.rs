
#[macro_use]
extern crate  rust_i18n;
// extern crate i18_local as rust_i18n;


// #[macro_use]
// extern crate rust_i18n;

use rust_i18n::i18n;

i18n!("../resources/locales");

fn main() {
    println!("resources::main() {}", t!("M_SETTINGS"));
    assert_eq!(t!("M_SETTINGS"), "Settings".to_string());
}
