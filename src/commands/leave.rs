use anyhow::anyhow;
use log::debug;
use poise::command;

use crate::types::*;

/// Leave the voice channel.
#[command(slash_command, guild_only)]
pub(crate) async fn leave(ctx: Context<'_>) -> Result<()> {
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
