// cd .. ;  cargo run  --example icons

extern crate gdk;
extern crate glib;

use std::fs::File;
use std::io::Write;
use ui_gtk::iconloader::IconLoader;

const ICONS_FOLDER: &str = "fr_gtk/src/icons/";
const OUTPUT_RS1: &str = "resources/src/gen_icons.rs";

/*
convert -size 48x48 xc:transparent -bordercolor brown -compose Copy -border 8  PNG8:02-icon_missing_brown.png
convert -size 48x48 xc:transparent -fill  green -draw "rectangle  21,21 27,27"  PNG8:06-center_point_green.png
convert -size 48x48 xc:transparent  PNG8:03-icon_transparent-48.png

convert -size 64x64 xc:transparent -fill transparent  -stroke blue  -strokewidth 8  -draw "arc 4,4 58,58 0,30"   draw_circle.png
    #  arc:   linker-Rand,Oberer-Rand	Breite,Hoehe	 Startwinkel,Stoppwinkel
*/
pub fn main() {
    generate_icons();
}

fn generate_icons() {
    let current_dir = std::env::current_dir().unwrap();
    println!("GENERATE ICONS dir={:?} {:?} ", current_dir, ICONS_FOLDER);
    let mut file_list: Vec<String> = Vec::default();

    if let Ok(entries) = std::fs::read_dir(ICONS_FOLDER) {
        entries.for_each(|e| {
            if let Ok(direntry) = e {
                let fileonly = direntry.file_name().to_str().unwrap().to_string();
                file_list.push(fileonly);
            }
        });
    }
    file_list.sort();
    let mut lines: Vec<String> = Vec::default();
    let mut names: Vec<String> = Vec::default();
    lines.push("// generated files, do not change".to_string());
    lines.push("// #  cargo run --example icons \n\n".to_string());
    file_list.iter().for_each(|fileonly| {
        let filename = format!("{}{}", ICONS_FOLDER, fileonly);
        let fn_cap = fileonly
            .to_uppercase()
            .replace(".PNG", "")
            .replace("-", "_");
        let icon_name = format!("ICON_{}", fn_cap);
        names.push(icon_name.clone());
        println!("D={:?}  => {:?}", filename, icon_name);
        let buf_ic = IconLoader::file_to_bin(&filename).unwrap();
        let compressed = IconLoader::compress_vec_to_string(&buf_ic);
        lines.push("#[allow(dead_code)]".to_string());
        lines.push(format!("pub const {}: &str = \"{}\";\n", icon_name, compressed).to_string());
    });

    lines.push(format!("pub const ICON_LIST: [&str; {}] = [", names.len()).to_string());
    names.iter().enumerate().for_each(|(i, n)| {
        lines.push(format!("	{},\t// {}", n, i));
    });
    lines.push("];\n\n".to_string());
    names.iter().enumerate().for_each(|(i, n)| {
        let n_r = n.replace("ICON_", "IDX_");
        lines.push(format!("pub const {}: usize = {};", n_r, i));
    });
    lines.push("\n".to_string());

    if true {
        let mut out = File::create(OUTPUT_RS1).unwrap();
        lines.iter().for_each(|l| {
            let _r = write!(out, "{}\n", l);
        });
    } else {
        lines.iter().for_each(|l| println!("{}", l));
    }
}
