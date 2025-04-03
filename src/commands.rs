use crate::UserData;
use crate::error::Error;
use duration_str::parse;
use poise::{Context, CreateReply};
use std::sync::Arc;

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_emoji(
    context: Context<'_, Arc<UserData>, Error>,
    #[description = "Emojis"] emojis: String,
) -> Result<(), Error> {
    let emoji_list = emojis
        .split(" ")
        .into_iter()
        .filter(|x| !x.is_empty())
        .map(|x| x.to_string())
        .collect::<Vec<String>>();

    context
        .send(CreateReply::default().content(format!(
            "Successfully added emojis to bot: {:?}",
            &emoji_list
        )))
        .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_requirement(
    context: Context<'_, Arc<UserData>, Error>,
    #[description = "Global Minimum for Starboard Posting"] minimum: i32,
) -> Result<(), Error> {
    context.data().set_requirement(minimum).await;
    context
        .send(CreateReply::default().content(format!("Successfully set new minimum: {}", minimum)))
        .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn exclude_channel(context: Context<'_, Arc<UserData>, Error>) -> Result<(), Error> {
    context
        .data()
        .add_exclude_channel(context.channel_id().get())
        .await;
    context
        .send(CreateReply::default().content("Successfully excluded this channel."))
        .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_override_requirement(
    context: Context<'_, Arc<UserData>, Error>,
    #[description = "Global Minimum for Starboard Posting"] minimum: i32,
) -> Result<(), Error> {
    context
        .data()
        .set_override_channel_requirement(context.channel_id().get(), minimum)
        .await;
    context.data().write_config_to_disk().await?;
    context
        .send(CreateReply::default().content(format!(
            "Successfully set new minimum for this channel: {}",
            minimum
        )))
        .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_sending_channel(context: Context<'_, Arc<UserData>, Error>) -> Result<(), Error> {
    context
        .data()
        .set_sending_channel(context.channel_id().get())
        .await;
    context.data().write_config_to_disk().await?;
    context
        .send(CreateReply::default().content("Successfully set this as the starboard channel."))
        .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_ignore_older_than(
    context: Context<'_, Arc<UserData>, Error>,
    #[description = "Duration"] older_than: String,
) -> Result<(), Error> {
    let older_than_duration = match parse(older_than) {
        Ok(d) => d,
        Err(why) => return Err(Error::BadDuration(why)),
    };

    context
        .data()
        .set_older_ignore(older_than_duration.clone())
        .await;
    context.data().write_config_to_disk().await?;
    context
        .send(CreateReply::default().content(format!(
            "Will ignore message older than {:?}.",
            older_than_duration
        )))
        .await?;

    Ok(())
}
