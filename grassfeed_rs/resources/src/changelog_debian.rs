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
pub fn create_debian_changelog(
    in_folder: &str,
    out_file: &str,
    package_name: &str,
    top_line_rest: &str,
    bottom_part: &str,
) {
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
    let mut outfile =
        std::fs::File::create(out_file).expect("build.rs, changelog_debian: cannot open out_file!");
    for name in file_list {
        let replaced = name.replace(".txt", "");
        let parts: Vec<&str> = replaced.split(':').collect();

        let chfilename = format!("{}{}", in_folder, name);
        let contents = std::fs::read_to_string(chfilename).unwrap();

        let line1 = format!("{} ({}) {}\n\n", package_name, parts[1], top_line_rest);
        outfile
            .write_all(line1.as_bytes())
            .expect("error writing out file");

        let date_line = contents.lines().next().unwrap();
        contents.lines().skip(1).for_each(|l| {
            let co = format!("  {}\n", l);
            outfile
                .write_all(co.as_bytes())
                .expect("error writing out file");
        });
        let line2 = format!("\n -- {}  {}\n\n", bottom_part, date_line);
        outfile
            .write_all(line2.as_bytes())
            .expect("error writing out file");
    }
}
