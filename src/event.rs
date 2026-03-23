use crate::UserData;
use crate::commands::modmail::{
    CREATE_SELECT_MENU_ID, ModmailOpenReason, ModmailThread, create_interaction_menu,
    waitress_embed,
};
use crate::error::Error;
use base64::Engine;
use chrono::Utc;
use log::{error, info, warn};
use poise::FrameworkContext;
use serenity::all::{
    Attachment, CacheHttp, Channel, ChannelId, ComponentInteraction, ComponentInteractionDataKind,
    Context, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, CreateThread, EditMessage, Embed, FullEvent,
    Interaction, Message, MessageBuilder, MessageId, MessageReaction, ReactionType, RoleId,
    Timestamp, User,
};
use std::ops::Sub;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

pub async fn handle_event(
    context: &Context,
    event: &FullEvent,
    _framework: FrameworkContext<'_, Arc<UserData>, Error>,
    data: &Arc<UserData>,
) -> Result<(), Error> {
    match event {
        FullEvent::MessageDelete {
            channel_id,
            deleted_message_id,
            ..
        } => {
            handle_message_deletion(context, data.clone(), deleted_message_id, channel_id).await?;
        }
        FullEvent::MessageDeleteBulk {
            channel_id,
            multiple_deleted_messages_ids,
            ..
        } => {
            for deleted_message in multiple_deleted_messages_ids {
                if let Err(why) =
                    handle_message_deletion(context, data.clone(), deleted_message, channel_id)
                        .await
                {
                    error!("Error during handling bulk message deletion: {why}");
                }
            }
            return Ok(());
        }
        FullEvent::MessageUpdate { event, new, .. } => {
            handle_message_edit(
                context,
                data.clone(),
                event.id,
                event.channel_id,
                new.clone(),
            )
            .await?;
        }
        FullEvent::ReactionAdd { add_reaction } => {
            info!("handling reaction add");
            if should_ignore_event(
                context,
                data.clone(),
                &add_reaction.message_id,
                &add_reaction.channel_id,
            )
            .await?
            {
                warn!("ignoring message {}", add_reaction.message_id);
                return Ok(());
            }

            let sending_channel = match data.config.read().await.starboard.sending_channel {
                Some(ch) => ch,
                None => {
                    warn!("sending channel not set. ignoring message edit event!");
                    return Ok(());
                }
            };

            let (mut msg_build, embed) = {
                let message = match context
                    .http()
                    .get_message(add_reaction.channel_id, add_reaction.message_id)
                    .await
                {
                    Ok(msg) => msg,
                    Err(why) => {
                        error!(
                            "failed to get message {}/{} - {}.",
                            add_reaction.channel_id, add_reaction.message_id, why
                        );
                        return Ok(());
                    }
                };

                let requirement =
                    get_channel_requirements(data.clone(), add_reaction.channel_id).await;
                let message_reaction = message_reactions(
                    &data.config.read().await.starboard.emoji,
                    &message.reactions,
                );

                if message_reaction < requirement {
                    info!(
                        "not enough reactions on message {} to add to starboard: {}<{}",
                        add_reaction.message_id.get(),
                        message_reaction,
                        requirement
                    );
                    return Ok(());
                }

                create_new_starboard_embed_message(
                    &message.reactions,
                    &message.link(),
                    &message.author,
                    &message.content,
                    &message.attachments,
                    &message.embeds,
                    &message.timestamp,
                    message.id,
                )
            };

            if let Some(board) = data
                .database
                .original_to_board(add_reaction.message_id.get())
                .await?
            {
                info!(
                    "updating board message {} from reaction added to {}/{}",
                    board, add_reaction.channel_id, add_reaction.message_id
                );

                let mut board_message = context
                    .http()
                    .get_message(sending_channel.into(), board.into())
                    .await?;

                let new_message = EditMessage::new().content(msg_build.build()).embed(embed);

                board_message.edit(context, new_message).await?;

                return Ok(());
            } else {
                // create new board message
                info!(
                    "creating new board message from reaction added to {}/{}",
                    add_reaction.channel_id, add_reaction.message_id
                );

                let new_message = CreateMessage::new().content(msg_build.build()).embed(embed);

                if let Channel::Guild(guild_ch) =
                    context.http().get_channel(sending_channel.into()).await?
                {
                    info!("sending message to #{} ({})", guild_ch.name, guild_ch.id);
                    let sent = guild_ch.send_message(context, new_message).await?;
                    data.database
                        .record_new(add_reaction.message_id.get(), sent.id.get())
                        .await?;
                    info!(
                        "sent message {} to #{}, logged sucessfully!",
                        sent.id, guild_ch.name
                    );
                } else {
                    error!("failed to get channel - {sending_channel} is not guild channel",);
                    return Ok(());
                }
            }
        }
        FullEvent::InteractionCreate { interaction } => {
            if let Interaction::Component(component) = interaction {
                if component.data.custom_id == CREATE_SELECT_MENU_ID {
                    create_new_modmail_thread_channel(context, data.clone(), component).await?;
                }
            }
        }
        FullEvent::Ready { data_about_bot } => {
            info!(
                "Ready as {:?}, v{:?}, in guilds {:?}, current application {:?}",
                data_about_bot.user.id,
                data_about_bot.version,
                data_about_bot.guilds,
                data_about_bot.application.id
            );
            return Ok(());
        }
        FullEvent::Resume { .. } => {
            info!("Resumed Connection to Discord");
            return Ok(());
        }
        FullEvent::Ratelimit { data } => {
            warn!(
                "Ratelimited {}! for {} seconds, is global: {}",
                data.path,
                data.timeout.as_secs(),
                data.global
            );
            return Ok(());
        }
        _ => return Ok(()),
    }

    Ok(())
}

pub async fn should_ignore_event(
    context: &Context,
    data: Arc<UserData>,
    message: &MessageId,
    channel: &ChannelId,
) -> Result<bool, Error> {
    // if data.database.is_blacklisted(message.get())? {
    //     info!("message blacklisted ignoring,");
    //     return Ok(true);
    // }

    let sending_channel = match data.config.read().await.starboard.sending_channel {
        Some(ch) => ch,
        None => {
            warn!("sending channel not set. ignoring message edit event!");
            return Ok(true);
        }
    };

    if channel.get() == sending_channel
        || data
            .config
            .read()
            .await
            .starboard
            .excluded_channels
            .contains(&channel.get())
    {
        info!(
            "ignoring event for message {} - in sending channel or other ignored channel {}",
            message.get(),
            channel.get()
        );
        return Ok(true);
    }

    // check if it meets requirements
    let message_got = context.http().get_message(*channel, *message).await?;

    if let Some(older_than) = data.config.read().await.starboard.ignore_older_than {
        let older_time_stamp = Timestamp::now().to_utc().sub(older_than);
        if message_got.timestamp.to_utc() < older_time_stamp {
            info!(
                "ignoring message {:?}/{:?} - too old!",
                channel, message_got.id
            );
            return Ok(true);
        }
    }
    Ok(false)
}

async fn create_new_modmail_thread_channel(
    context: &Context,
    data: Arc<UserData>,
    component: &ComponentInteraction,
) -> Result<(), Error> {
    let modmail_reason = match &component.data.kind {
        ComponentInteractionDataKind::StringSelect { values } => {
            if values.len() > 1 || values.is_empty() {
                return Err(Error::BadInteraction);
            }
            ModmailOpenReason::from_str(&values[0])?
        }
        _ => {
            crate::log_channel(
                context,
                &data,
                format!("??? invalid select type for {}", component.user.id),
            )
            .await;
            return Err(Error::BadInteraction);
        }
    };

    let roles = data
        .config
        .read()
        .await
        .modmail
        .roles
        .iter()
        .map(|id| RoleId::new(*id))
        .map(|role_id| format!("<@&{}>", role_id.get()))
        .collect::<Vec<String>>()
        .join(", ");

    crate::log_channel(
        context,
        &data,
        format!(
            "handling create modmail for user {}, type {}, {}",
            component.user.id, modmail_reason, &roles,
        ),
    )
    .await;
    let guild_channel = context
        .http()
        .get_channel(component.channel_id)
        .await?
        .guild()
        .ok_or(Error::NotFound)?;
    let name = format!(
        "{}_{}-{}",
        component.user.name,
        modmail_reason,
        base64::prelude::BASE64_STANDARD.encode(Utc::now().timestamp().to_le_bytes())
    );
    let thread = guild_channel
        .create_thread(
            context.http(),
            CreateThread::new(name)
                .audit_log_reason(&modmail_reason.to_string())
                .invitable(false),
        )
        .await?;
    thread
        .id
        .add_thread_member(context.http(), component.user.id)
        .await?;

    let modmail = ModmailThread::new(
        thread.id,
        component.user.id,
        component.id.created_at().unix_timestamp(),
        modmail_reason,
    );
    data.database.create_new_modmail(&modmail).await?;

    let bot_pfp = context
        .http()
        .get_current_user()
        .await?
        .default_avatar_url();

    component
        .create_response(
            context.http(),
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(format!(
                        "Your modmail has been opened here: <#{}>",
                        modmail.thread_id.get()
                    ))
                    .ephemeral(true),
            ),
        )
        .await?;

    let user = format!("<@{}>", component.user.id.get());
    for _ in 0..3 {
        sleep(Duration::from_secs(3));
        let msg_snd_result = thread.send_message(context.http(), CreateMessage::new()
        .add_embed(CreateEmbed::new()
            .title("Welcome to the Madamoiselle Cafe")
            .description(format!("Please await to be seated. Your ticket for {modmail_reason} will be dealt with soon. We will bring out your complementary Flesh Coffee very soon. Note: You can use /resolve to close this modmail."))
            .color((236, 212, 0))
            .author(CreateEmbedAuthor::from(&component.user))
            .footer(CreateEmbedFooter::new("MADAMOISELLE CAFE - SERVING FLESH COFFEE EST. 2018").icon_url(&bot_pfp)))
        .content(format!("{roles} {user}"))).await;
        if msg_snd_result.is_ok() {
            break;
        } else {
            crate::log_channel(
                context,
                &data,
                format!(
                    "Created modmail <#{}> for user {}, reason {}, sending msg failed!!!!",
                    modmail.thread_id.get(),
                    user,
                    modmail_reason
                ),
            )
            .await;
        }
    }
    // reset
    let mut component_msg = component.message.clone();
    component_msg
        .edit(
            context.http(),
            EditMessage::new()
                .embed(waitress_embed(&bot_pfp))
                .select_menu(create_interaction_menu()),
        )
        .await?;
    crate::log_channel(
        context,
        &data,
        format!(
            "Created modmail <#{}> for user {}, reason {}",
            modmail.thread_id.get(),
            user,
            modmail_reason
        ),
    )
    .await;
    Ok(())
}
