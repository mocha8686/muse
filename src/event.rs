use std::sync::Arc;

use log::{error, trace};
use poise::{
    async_trait,
    serenity_prelude::{Cache, ChannelId, Http},
};
use songbird::{Event, EventContext, EventHandler};

use crate::format::{base_embed, song_embed};

pub(crate) struct NowPlaying {
    cache: Arc<Cache>,
    channel: ChannelId,
    guild_name: String,
    http: Arc<Http>,
}

impl NowPlaying {
    pub(crate) fn new(
        channel: ChannelId,
        guild_name: String,
        cache: Arc<Cache>,
        http: Arc<Http>,
    ) -> Self {
        NowPlaying {
            channel,
            guild_name,
            cache,
            http,
        }
    }
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