use anyhow::anyhow;
use log::{debug, trace};
use poise::{
    command,
    serenity_prelude::{ChannelType, GuildChannel, Mentionable},
};
use songbird::{
    input::{Input, Restartable},
    Event, TrackEvent,
};

use crate::{
    event::NowPlaying,
    format::{format_user_for_log, song_embed},
    types::*,
};

/// Add a song to the queue.
#[command(slash_command, guild_only)]
pub(crate) async fn play(
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
    let title = song.metadata.title.as_ref().unwrap();

    debug!("Enqueued `{title}` in {guild_name}.");

    ctx.send(|m| {
        m.content(if first_play {
            format!("Now playing *{title}*.")
        } else {
            format!("Queued *{title}*.")
        })
        .embed(|e| song_embed(e, &song.metadata))
    })
    .await?;

    let mut handler = handler_lock.lock().await;
    handler.enqueue_source(song);

    Ok(())
}
