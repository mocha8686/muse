use std::time::Duration;

use anyhow::anyhow;
use log::{error, warn};
use poise::{
    command,
    serenity_prelude::{CollectComponentInteraction, InteractionResponseType},
};

use crate::{
    format::{queue_message, queue_message_edit},
    types::*,
};

/// View the current queue.
#[command(slash_command, guild_only)]
pub(crate) async fn queue(
    ctx: Context<'_>,
    #[description = "Queue page"] page: Option<usize>,
) -> Result<()> {
    let mut page = page.unwrap_or(0);
    let guild_id = ctx.guild_id().unwrap();
    let Some(manager) = songbird::get(ctx.serenity_context()).await else {
        return Err(anyhow!(SONGBIRD_MANAGER_ERR));
    };

    let Some(handler_lock) = manager.get(guild_id) else {
        ctx.send(|m| m.content("I'm not in a voice channel.").ephemeral(true)).await?;
        return Ok(());
    };

    let queue = {
        let handler = handler_lock.lock().await;
        handler.queue().current_queue()
    };

    let reply_handle = if queue.is_empty() {
        ctx.send(|m| m.content("The queue is empty.").ephemeral(true))
            .await?;
        return Ok(());
    } else {
        ctx.send(|m| {
            let (m, new_page) = queue_message(m, &queue, page, false);
            page = new_page;
            m
        })
        .await?
    };

    while let Some(interaction) = CollectComponentInteraction::new(ctx)
        .author_id(ctx.author().id)
        .message_id(reply_handle.message().await?.id)
        .timeout(Duration::from_secs(60))
        .await
    {
        match &*interaction.data.custom_id {
            "first" => page = 0,
            "previous" => page = page.saturating_sub(1),
            "next" => page = page.saturating_add(1),
            "last" => page = usize::MAX,
            other => {
                warn!("Unknown interaction `{other}`,");
                continue;
            }
        }

        let mut msg = interaction.message.clone();
        msg.edit(ctx, |m| {
            let (m, new_page) = queue_message_edit(m, &queue, page);
            page = new_page;
            m
        })
        .await?;

        if let Err(e) = interaction
            .create_interaction_response(ctx, |r| {
                r.kind(InteractionResponseType::DeferredUpdateMessage)
            })
            .await
        {
            error!("Error while creating interaction response for queue: {e}");
        };
    }

    reply_handle
        .edit(ctx, |m| queue_message(m, &queue, page, true).0)
        .await?;

    Ok(())
}
