use std::io::Write;

#[allow(dead_code)]
pub fn get_env(key: &str) -> Option<String> {
    if let Some(s1) = std::env::var_os(key) {
        if let Some(s2) = s1.to_str() {
            return Some(s2.to_string());
        }
    }
    None
}

///  with trailing slash
/// returns the contents of the written file.
pub fn create_debian_changelog(
    in_folder: &str,
    out_file: &str,
    package_name: &str,
    top_line_rest: &str,
    bottom_part: &str,
    recent_version: &str,
) -> String {
    let mut file_list: Vec<String> = Vec::default();
    if let Ok(entries) = std::fs::read_dir(in_folder) {
        entries.for_each(|e| {
            if let Ok(direntry) = e {
                let fname = direntry.file_name().to_str().unwrap().to_string();
                if fname.contains(':') && fname.ends_with(".txt") {
                    file_list.push(fname);
                }
            }
        });
    }
    file_list.sort();
    file_list.reverse();
    let mut file_contents: String = String::default();
    let mut outfile =
        std::fs::File::create(out_file).expect("build.rs, changelog_debian: cannot open out_file!");
    let e_msg = format!("Error writing {} ", out_file);
    file_list.iter().enumerate().for_each(|(num, name)| {
        let replaced = name.replace(".txt", "");
        let parts: Vec<&str> = replaced.split(':').collect();
        let version = if num == 0 { recent_version } else { parts[1] };
        let chfilename = format!("{in_folder}{name}");
        let contents = std::fs::read_to_string(chfilename).unwrap();
        let line1 = format!("{} ({}) {}\n\n", package_name, version, top_line_rest);
        file_contents.push_str(&line1);
        outfile.write_all(line1.as_bytes()).expect(&e_msg);

        let date_line = contents.lines().next().unwrap();
        contents.lines().skip(1).for_each(|l| {
            let co = format!("  {l}\n");
            file_contents.push_str(&co);
            outfile.write_all(co.as_bytes()).expect(&e_msg);
        });
        let line2 = format!("\n -- {bottom_part}  {date_line}\n\n");
        file_contents.push_str(&line2);
        outfile.write_all(line2.as_bytes()).expect(&e_msg);
    });

    // for name in file_list {    }
    file_contents
}
