use poise::command;

use crate::types::*;

#[command(prefix_command, owners_only, hide_in_help)]
pub(crate) async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}
