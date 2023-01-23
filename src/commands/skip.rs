use anyhow::anyhow;
use log::error;
use poise::command;

use crate::{format::song_embed, types::*};

/// Skip through songs in the queue.
#[command(slash_command, guild_only)]
pub(crate) async fn skip(
    ctx: Context<'_>,
    #[description = "Number of songs to skip"] n: Option<usize>,
) -> Result<()> {
    let n = n.unwrap_or(1).max(1);
    let guild_id = ctx.guild_id().unwrap();
    let Some(manager) = songbird::get(ctx.serenity_context()).await else {
        return Err(anyhow!(SONGBIRD_MANAGER_ERR));
    };

    let Some(handler_lock) = manager.get(guild_id) else {
        ctx.send(|m| m.content("I'm not in a voice channel.").ephemeral(true)).await?;
        return Ok(());
    };

    let mut first_song = None;
    {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.is_empty() {
            ctx.send(|m| m.content("I'm not playing any songs.").ephemeral(true))
                .await?;
            return Ok(());
        }

        queue.modify_queue(|q| {
            for _ in 0..n {
                if let Some(s) = q.pop_front() {
                    if let Err(e) = s.stop() {
                        error!("Error while stopping track: {e}");
                    }
                    first_song = Some(s);
                }
            }
        });
    }

    if n == 1 {
        ctx.send(|m| {
            let metadata = first_song.as_ref().unwrap().metadata();
            m.content(format!("Skipped *{}*.", metadata.title.as_ref().unwrap()))
                .embed(|e| song_embed(e, metadata))
        })
        .await?;
    } else {
        ctx.send(|m| m.content(format!("Skipped {n} songs.")))
            .await?;
    }

    Ok(())
}
