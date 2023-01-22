use anyhow::{anyhow, Result};
use chrono::Local;
use dotenv::dotenv;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::{debug, error, info, trace, LevelFilter};
use poise::{
    async_trait, command,
    serenity_prelude::{
        Cache, ChannelId, ChannelType, CreateEmbed, GatewayIntents, GuildChannel, Http,
        Mentionable, User,
    },
    Framework, FrameworkOptions,
};
use songbird::{
    input::{Input, Metadata, Restartable},
    Event, EventContext, EventHandler, SerenityInit, TrackEvent,
};
use std::{env, io, sync::Arc};

struct Data;
type Error = anyhow::Error;
type Context<'a> = poise::Context<'a, Data, Error>;
type FrameworkError<'a> = poise::FrameworkError<'a, Data, Error>;

struct NowPlaying {
    guild_name: String,
    cache: Arc<Cache>,
    http: Arc<Http>,
    channel: ChannelId,
}

fn base_embed(e: &mut CreateEmbed) -> &mut CreateEmbed {
    e.color(0x0789f0)
}

fn song_embed<'e, 's>(e: &'e mut CreateEmbed, song: &'s Metadata) -> &'e mut CreateEmbed {
    todo!()
}

#[async_trait]
impl EventHandler for NowPlaying {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let EventContext::Track(&[(_, handle)]) = ctx else {
            return None;
        };

        let metadata = handle.metadata();

        if let Some(ref title) = metadata.title {
            trace!("Now playing `{}` in {}.", title, self.guild_name);
        } else {
            trace!("Now playing a new song in {}.", self.guild_name);
        }

        if let Err(e) = self
            .channel
            .send_message(&self.http, |m| {
                m.embed(|e| song_embed(base_embed(e), metadata))
            })
            .await
        {
            error!(
                "Error sending `Now Playing` notification in {}: {e}",
                self.channel
                    .name(&self.cache)
                    .await
                    .unwrap_or_else(|| self.channel.to_string())
            )
        };

        None
    }
}

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
    let Some(manager) = songbird::get(ctx.serenity_context()).await else {
        return Err(anyhow!(SONGBIRD_MANAGER_ERR));
    };
    let guild_id = ctx.guild_id().unwrap();
    let guild_name = guild_id.name(ctx).unwrap_or_else(|| guild_id.to_string());

    let handler_lock = if let Some(handler_lock) = manager.get(guild_id) {
        handler_lock
    } else {
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
        let (handler_lock, res) = manager.join(guild_id, voice_channel).await;
        res?;

        {
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(
                Event::Track(TrackEvent::Play),
                NowPlaying {
                    channel: ctx.channel_id(),
                    cache: ctx.serenity_context().cache.clone(),
                    http: ctx.serenity_context().http.clone(),
                    guild_name: guild_name.clone(),
                },
            );
        }

        handler_lock
    };

    trace!(
        "{} ran a YouTube search for `{}`.",
        format_user_for_log(ctx.author()),
        song
    );

    let song: Input = Restartable::ytdl_search(song, true).await?.into();

    if let Some(title) = &song.metadata.title {
        debug!("Enqueued `{title}` in {guild_name}.");
    } else {
        debug!("Enqueued a song in {guild_name}.");
    }

    let mut handler = handler_lock.lock().await;
    handler.enqueue_source(song);

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
        debug!(
            "Left {}.",
            guild_id.name(ctx).unwrap_or_else(|| guild_id.to_string()),
        );
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
