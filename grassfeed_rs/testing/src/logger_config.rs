#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use fern::colors::Color;
use fern::colors::ColoredLevelConfig;

pub fn setup_logger() -> Result<(), fern::InitError> {
    let mut colors = ColoredLevelConfig::new().info(Color::Green);
    colors.trace = Color::Blue;
    colors.debug = Color::Cyan;
    colors.info = Color::Green;
    colors.warn = Color::Yellow;
    colors.error = Color::Red;

    let logfilename = "../target/testing.log";
    let _r = std::fs::remove_file(logfilename);

    let o_logfile = fern::log_file(logfilename);
    if o_logfile.is_err() {
        error!("setup_logger: cannot create {}", logfilename);
        return Err(fern::InitError::Io(o_logfile.err().unwrap()));
    }
    let logfile = o_logfile.unwrap();
    //    let logfile = fern::log_file(logfilename).unwrap();
    fern::Dispatch::new()
        .format(move |out, message, record| {
            const TARGET_WIDTH: usize = 25;
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
        .level(log::LevelFilter::Trace)
        .level_for("rustls", log::LevelFilter::Info)
        .level_for("sqlparser::parser", log::LevelFilter::Info)
        .level_for("ureq", log::LevelFilter::Info)
        .level_for("testing::minihttpserver", log::LevelFilter::Info)
        .level_for("fr_core::downloader", log::LevelFilter::Debug)
        .chain(logfile)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
