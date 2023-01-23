use anyhow::anyhow;
use log::error;
use poise::command;

use crate::{format::song_embed, types::*};

/// Remove a song from the queue.
#[command(slash_command, guild_only)]
pub(crate) async fn remove(
    ctx: Context<'_>,
    #[description = "Song number (based on queue) to remove"] n: usize,
) -> Result<()> {
    let guild_id = ctx.guild_id().unwrap();
    let Some(manager) = songbird::get(ctx.serenity_context()).await else {
        return Err(anyhow!(SONGBIRD_MANAGER_ERR));
    };

    let Some(handler_lock) = manager.get(guild_id) else {
        ctx.send(|m| m.content("I'm not in a voice channel.").ephemeral(true)).await?;
        return Ok(());
    };

    let song = {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.is_empty() {
            ctx.send(|m| m.content("I'm not playing any songs.").ephemeral(true))
                .await?;
            return Ok(());
        }
        if n >= queue.len() {
            ctx.send(|m| m.content("Invalid song number.").ephemeral(true))
                .await?;
            return Ok(());
        }

        let mut song = None;
        queue.modify_queue(|q| {
            let removed_song = q.remove(n).unwrap();
            if let Err(e) = removed_song.stop() {
                error!("Error while stopping track: {e}");
            }
            song = Some(removed_song)
        });
        song.unwrap()
    };

    ctx.send(|m| {
        let metadata = song.metadata();
        m.content(format!("Removed *{}*.", metadata.title.as_ref().unwrap()))
            .embed(|e| song_embed(e, metadata))
    })
    .await?;

    Ok(())
}
