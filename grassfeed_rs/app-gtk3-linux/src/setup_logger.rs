use fern::colors::Color;
use fern::colors::ColoredLevelConfig;

const TARGET_WIDTH: usize = 20;

/// 0: regular, no debug Output
/// 1: error Output
/// 2: Warn Output
/// 3: Info
/// 4: Debug
/// 5: Trace
pub fn setup_logger(
    debug_level: u8,
    cache_dir: &String,
    app_name: &str,
) -> Result<(), fern::InitError> {
    let filter_level: log::LevelFilter = levelfilter_for_num(debug_level);
    let mut colors = ColoredLevelConfig::new().info(Color::Green);
    colors.trace = Color::Blue;
    colors.debug = Color::Cyan;
    colors.info = Color::Green;
    colors.warn = Color::Yellow;
    colors.error = Color::Red;
    let logfilename = format!("{}{}.log", cache_dir, app_name);
    if debug_level > 0 {
        fern::Dispatch::new()
            .level(filter_level)
            .level_for("rustls", log::LevelFilter::Info)
            .level_for("ureq", log::LevelFilter::Info)
            .format(move |out, message, record| {
                let target: &str = record.target();
                let t_short = if target.len() > TARGET_WIDTH {
                    target.split_at(target.len() - TARGET_WIDTH).1
                } else {
                    target
                };
                out.finish(format_args!(
                    "{} {:5} {:12}\t{}",
                    chrono::Local::now().format("%H:%M:%S:%3f"),
                    colors.color(record.level()),
                    t_short,
                    message
                ))
            })
            .chain(std::io::stdout())
            .chain(fern::log_file(logfilename)?)
            .apply()?;
    } else {
        fern::Dispatch::new()
            .level(filter_level)
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{} {:5} {:12}\t{}",
                    chrono::Local::now().format("%H:%M:%S:%3f"),
                    colors.color(record.level()),
                    record.target(),
                    message
                ))
            })
            .chain(fern::log_file(logfilename)?)
            .apply()?;
    }

    Ok(())
}

fn levelfilter_for_num(loc_level: u8) -> log::LevelFilter {
    match loc_level {
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Warn,
    }
}