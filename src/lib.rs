pub(crate) mod commands;
pub(crate) mod event;
pub(crate) mod format;
pub(crate) mod logger;
pub(crate) mod types;

use std::env;

use anyhow::Result;
use log::{error, info, trace};
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
    info!("Initializing framework...");
    setup_logger()?;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![play(), register(), now_playing(), leave(), queue(), skip()],
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
        .client_settings(SerenityInit::register_songbird);

    info!("Framework initialized. Starting.");
    framework.run().await?;

    Ok(())
}
