#[path = "../resources/src/gitversion.rs"]
mod gitversion;

pub fn main() {


    if let Some(osstr) = std::env::var_os("OUT_DIR") {
        gitversion::build_rs_main(osstr.to_str().unwrap());
    }
}
