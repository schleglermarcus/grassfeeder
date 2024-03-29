#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use fern::colors::Color;
use fern::colors::ColoredLevelConfig;

#[allow(dead_code)]
pub enum QuietFlags {
    All = 0,
    MiniHttp = 1,
    Config = 2,
    Db = 4,
    Downloader = 8,
    Controller = 16,
}

#[allow(dead_code)]
pub fn setup_fern_logger(debug_flags: u64) -> Result<(), fern::InitError> {
    let mut colors = ColoredLevelConfig::new().info(Color::Green);
    colors.trace = Color::Blue;
    colors.debug = Color::Cyan;
    colors.info = Color::Green;
    colors.warn = Color::Yellow;
    colors.error = Color::Red;
    const TARGET_LEN: usize = 40;
    let mut level_config = fern::Dispatch::new()
        .level(log::LevelFilter::Trace)
        .level(log::LevelFilter::Trace)
        .level_for("sled", log::LevelFilter::Info)
        .level_for("sqlparser::parser", log::LevelFilter::Info)
        .level_for("ureq", log::LevelFilter::Info)
        .level_for("rustls", log::LevelFilter::Info)
        .level_for("webbrowser::os", log::LevelFilter::Info);
    if debug_flags & QuietFlags::MiniHttp as u64 > 0 {
        level_config = level_config.level_for("testing::minihttpserver", log::LevelFilter::Debug);
    }
    if debug_flags & QuietFlags::Config as u64 > 0 {
        level_config = level_config.level_for("fr_core::config", log::LevelFilter::Debug);
    }
    if debug_flags & QuietFlags::Db as u64 > 0 {
        level_config = level_config.level_for("fr_core::db", log::LevelFilter::Info);
    }
    if debug_flags & QuietFlags::Downloader as u64 > 0 {
        level_config = level_config.level_for("fr_core::downloader", log::LevelFilter::Info);
    }
    if debug_flags & QuietFlags::Controller as u64 > 0 {
        level_config = level_config.level_for("fr_core::controller", log::LevelFilter::Info);
    }
    let format_config = level_config.format(move |out, message, record| {
        let target: &str = record.target();
        let t_short = if target.len() > TARGET_LEN {
            target.split_at(target.len() - TARGET_LEN).1
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
    });
    format_config.chain(std::io::stdout()).apply()?;
    Ok(())
}
