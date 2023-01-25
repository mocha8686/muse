use std::{env, io};

use anyhow::Result;
use chrono::Local;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::{info, trace, LevelFilter};

use crate::{format::format_user_for_log, types::Context};

pub(crate) fn setup_logger() -> Result<()> {
    trace!("Setting up logger...");

    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Blue)
        .debug(Color::Magenta)
        .trace(Color::White);

    Dispatch::new()
        .format(move |out, msg, record| {
            out.finish(format_args!(
                "{b}{time}{e} {l}{level:<5}{e} {c}{module}{e} {l}{msg}{e}",
                time = Local::now().format("[%Y-%m-%d %T]"),
                module = record.target(),
                level = record.level(),
                l = format_args!("\x1B[{}m", colors.get_color(&record.level()).to_fg_str()),
                b = format_args!("\x1B[{}m", Color::BrightBlack.to_fg_str()),
                c = format_args!("\x1B[{}m", Color::Cyan.to_fg_str()),
                e = "\x1B[0m",
            ));
        })
        .level(if env::var("MUSE_LOG_VERBOSE").is_ok() {
            LevelFilter::Info
        } else {
            LevelFilter::Warn
        })
        .level_for(
            "muse",
            if env::var("MUSE_LOG_VERBOSE").is_ok() {
                LevelFilter::Trace
            } else {
                LevelFilter::Debug
            },
        )
        .level_for("tracing", LevelFilter::Warn)
        .chain(io::stderr())
        .apply()?;

    trace!("Logger set up.");

    Ok(())
}

pub(crate) fn log_command(ctx: Context<'_>) {
    info!(
        "{} executed `{}`.",
        format_user_for_log(ctx.author()),
        ctx.command().name,
    );
}
