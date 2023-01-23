mod event;
mod format;
mod logger;
mod types;

use std::env;

use anyhow::{anyhow, Result};
use log::{debug, error, trace};
use poise::{
    command,
    serenity_prelude::{ChannelType, GatewayIntents, GuildChannel, Mentionable},
    Framework, FrameworkOptions,
};
use songbird::{
    input::{Input, Restartable},
    Event, SerenityInit, TrackEvent,
};

use event::NowPlaying;
use format::{format_user_for_log, now_playing_message, song_embed};
use logger::{log_command, setup_logger};
use types::{Context, Data, FrameworkError};

const SONGBIRD_MANAGER_ERR: &str = "Failed to acquire Songbird manager.";

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

/// Add a song to the queue.
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
    let mut first_play = false;

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

        first_play = true;

        {
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(
                Event::Track(TrackEvent::Play),
                NowPlaying::new(
                    ctx.channel_id(),
                    guild_name.clone(),
                    ctx.serenity_context().cache.clone(),
                    ctx.serenity_context().http.clone(),
                ),
            );
        }

        handler_lock
    };

    ctx.defer().await?;

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

    ctx.send(|m| {
        m.content(if let Some(title) = &song.metadata.title {
            if first_play {
                format!("Now playing *{title}*.")
            } else {
                format!("Queued *{title}*.")
            }
        } else if first_play {
            "Now playing a new song.".to_string()
        } else {
            "Queued a new song.".to_string()
        })
        .embed(|e| song_embed(e, &song.metadata))
    })
    .await?;

    let mut handler = handler_lock.lock().await;
    handler.enqueue_source(song);

    Ok(())
}

/// View the song currently playing.
#[command(slash_command, guild_only, rename = "nowplaying")]
async fn now_playing(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let Some(manager) = songbird::get(ctx.serenity_context()).await else {
        return Err(anyhow!(SONGBIRD_MANAGER_ERR));
    };

    let Some(handler_lock) = manager.get(guild_id) else {
        ctx.send(|m| m.content("I'm not in a voice channel.").ephemeral(true)).await?;
        return Ok(());
    };

    let handler = handler_lock.lock().await;
    let Some(song) = handler.queue().current() else {
        ctx.send(|m| m.content("I'm not playing a song.").ephemeral(true)).await?;
        return Ok(());
    };

    ctx.send(|m| now_playing_message(m, song.metadata()))
        .await?;
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

pub async fn start() -> Result<()> {
    debug!("Initializing framework...");
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
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async move {
                trace!("Setting up framework data...");
                Ok(Data)
            })
        })
        .client_settings(|c| c.register_songbird());

    debug!("Framework initialized. Starting.");
    framework.run().await?;

    Ok(())
}
