use crate::error::Error;
use crate::{UserData, shutdown_stuff};
use log::{info, warn};
use poise::Context;
use std::sync::Arc;

pub mod board;
pub mod modmail;

#[poise::command(prefix_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn stop(context: Context<'_, Arc<UserData>, Error>) -> Result<(), Error> {
    let _ = context.reply("LINQing myself...").await;
    info!("going to eepy!");
    shutdown_stuff(context.data().clone()).await;
    Ok(())
}

#[poise::command(prefix_command, default_member_permissions = "ADMINISTRATOR")]
pub async fn register(context: Context<'_, Arc<UserData>, Error>) -> Result<(), Error> {
    warn!("registering commands!!");
    poise::builtins::register_application_commands_buttons(context).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
pub async fn ping(context: Context<'_, Arc<UserData>, Error>) -> Result<(), Error> {
    context
        .reply(format!(
            "Pong! Took {}ms.",
            context.ping().await.as_millis()
        ))
        .await?;
    Ok(())
}
