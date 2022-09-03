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

    fern::Dispatch::new()
        .format(move |out, message, record| {
            let target: &str = record.target();
            let t_short = if target.len() > 12 {
                target.split_at(target.len() - 12).1
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
        .level_for("sled", log::LevelFilter::Info)
        // .level_for("sled::pagecache", log::LevelFilter::Info)
        // .level_for("sled::tree", log::LevelFilter::Debug)
        // .level_for("sled::context", log::LevelFilter::Debug)
        .level_for("sqlparser::parser", log::LevelFilter::Info)
        .level_for("ureq", log::LevelFilter::Info)
        .level_for("testing::minihttpserver", log::LevelFilter::Debug)
        //		.level_for("ureq::stream", log::LevelFilter::Info)
        // .level_for("ureq::unit", log::LevelFilter::Info)
        // .level_for("ureq::pool", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

/*
fn setup_logger() -> Result<(), fern::InitError> {
    let mut colors = ColoredLevelConfig::new().info(Color::Green);
    colors.trace = Color::Blue;
    colors.debug = Color::Cyan;
    colors.info = Color::Green;
    colors.warn = Color::Yellow;
    colors.error = Color::Red;

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                //                "{} {:5} {:?}:{:?} {}",
                "{} {:5} {}",
                chrono::Local::now().format("%H:%M:%S:%3f"),
                colors.color(record.level()),
                //record.target(),
                //				record.file(),
                //				record.line(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .level_for("XXX", log::LevelFilter::Debug)
        .chain(std::io::stdout())
        // .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}


*/
