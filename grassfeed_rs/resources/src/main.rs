#[macro_use]
extern crate rust_i18n;

i18n!("../resources/locales");

fn main() {
    println!("resources::main() {}", t!("M_SETTINGS"));
    assert_eq!(t!("M_SETTINGS"), "Settings".to_string());
}
