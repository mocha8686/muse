mod commands;
mod event;
mod format;
mod logger;
mod types;

use std::env;

use anyhow::Result;
use log::{debug, error, trace};
use poise::{serenity_prelude::GatewayIntents, Framework, FrameworkOptions};
use songbird::SerenityInit;

use commands::*;
use format::format_user_for_log;
use logger::{log_command, setup_logger};
use types::{Data, FrameworkError};

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

pub async fn start() -> Result<()> {
    debug!("Initializing framework...");
    setup_logger()?;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![play(), register(), now_playing(), leave(), queue()],
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
