use anyhow::Result;
use chrono::Local;
use dotenv::dotenv;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::{error, info, LevelFilter};
use poise::{
    command,
    serenity_prelude::{GatewayIntents, User},
    Framework, FrameworkOptions,
};
use std::{env, io};

struct Data;
type Error = anyhow::Error;
type Context<'a> = poise::Context<'a, Data, Error>;
type FrameworkError<'a> = poise::FrameworkError<'a, Data, Error>;

fn setup_logger() -> Result<()> {
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
            ))
        })
        .level(LevelFilter::Warn)
        .level_for("muse", LevelFilter::Debug)
        .chain(io::stderr())
        .apply()?;

    Ok(())
}

fn format_user_for_log(user: &User) -> String {
    format!("{} [{}]", user.tag(), user.id)
}

fn log_command(ctx: Context<'_>) {
    info!(
        "{} executed `{}`.",
        format_user_for_log(ctx.author()),
        ctx.command().name,
    );
}

async fn on_error(err: FrameworkError<'_>) {
    match err {
        FrameworkError::Command { ref error, ref ctx } => error!(
            "Error while executing `{}` for {}: {error}",
            ctx.command().qualified_name,
            format_user_for_log(ctx.author())
        ),
        FrameworkError::UnknownCommand { .. } => return,
        _ => error!("{err}"),
    }

    let Some(ctx) = err.ctx() else { return };
    if let Err(e) = ctx
        .send(|m| m.content("There was an error.").ephemeral(true))
        .await
    {
        error!("Error while reporting error: {e}");
    }
}

#[command(prefix_command, owners_only)]
async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[command(slash_command)]
async fn ping(ctx: Context<'_>) -> Result<()> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;

    setup_logger()?;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![ping(), register()],
            pre_command: |ctx| Box::pin(async move { log_command(ctx) }),
            on_error: |err| Box::pin(async move { on_error(err).await }),
            ..Default::default()
        })
        .token(env::var("DISCORD_TOKEN")?)
        .intents(GatewayIntents::non_privileged())
        .setup(|_ctx, _ready, _framework| Box::pin(async move { Ok(Data) }));

    framework.run().await?;

    Ok(())
}
