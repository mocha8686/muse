use anyhow::{anyhow, Result};
use chrono::Local;
use dotenv::dotenv;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::{error, info, LevelFilter};
use poise::{
    command,
    serenity_prelude::{ChannelType, GatewayIntents, GuildChannel, Mentionable, User},
    Framework, FrameworkOptions,
};
use songbird::SerenityInit;
use std::{env, io};

struct Data;
type Error = anyhow::Error;
type Context<'a> = poise::Context<'a, Data, Error>;
type FrameworkError<'a> = poise::FrameworkError<'a, Data, Error>;

const SONGBIRD_MANAGER_ERR: &str = "Failed to acquire Songbird manager.";

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

#[command(prefix_command, owners_only, hide_in_help)]
async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Play a song or add it to the queue.
#[command(slash_command, guild_only)]
async fn play(
    ctx: Context<'_>,
    #[description = "The song to play (YouTube search or URL)."] song: String,
    #[description = "The voice channel to join."] voice_channel: Option<GuildChannel>,
) -> Result<()> {
    let voice_channel = match voice_channel {
        Some(channel) => match channel.kind {
            ChannelType::Voice => channel.id,
            _ => {
                ctx.send(|m| {
                    m.content(format!("{} is not a voice channel.", channel.mention()))
                        .ephemeral(true)
                })
                .await?;
                return Ok(());
            }
        },
        None => {
            let guild = ctx.guild().unwrap();
            let Some(channel_id) = guild.voice_states.get(&ctx.author().id).and_then(|voice_state| voice_state.channel_id) else {
                ctx.send(|m| m.content("I'm not in a voice channel. Join or specify one.").ephemeral(true)).await?;
                return Ok(());
            };
            channel_id
        }
    };

    let Some(manager) = songbird::get(ctx.serenity_context()).await else {
        return Err(anyhow!(SONGBIRD_MANAGER_ERR));
    };

    let (_, res) = manager.join(ctx.guild_id().unwrap(), voice_channel).await;
    res?;

    Ok(())
}

/// Leave the voice channel.
#[command(slash_command, guild_only)]
async fn leave(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let Some(manager) = songbird::get(ctx.serenity_context()).await else {
        return Err(anyhow!(SONGBIRD_MANAGER_ERR));
    };

    let handler = manager.get(guild_id);
    if handler.is_some() {
        manager.remove(guild_id).await?;
        ctx.say("Left voice channel.").await?;
    } else {
        ctx.send(|m| m.content("I'm not in a voice channel.").ephemeral(true))
            .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;

    setup_logger()?;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![play(), register()],
            pre_command: |ctx| Box::pin(async move { log_command(ctx) }),
            on_error: |err| Box::pin(async move { on_error(err).await }),
            ..Default::default()
        })
        .token(env::var("DISCORD_TOKEN")?)
        .intents(GatewayIntents::non_privileged())
        .setup(|_ctx, _ready, _framework| Box::pin(async move { Ok(Data) }))
        .client_settings(|c| c.register_songbird());

    framework.run().await?;

    Ok(())
}
