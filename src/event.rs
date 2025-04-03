use crate::UserData;
use crate::error::Error;
use log::{error, info, warn};
use poise::FrameworkContext;
use serenity::all::{
    Attachment, ChannelId, Context, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, EditMessage,
    FullEvent, MessageBuilder, MessageId, MessageReaction, ReactionType, Timestamp, User,
};
use std::ops::Sub;
use std::sync::Arc;

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
                    error!("Error during handling bulk message deletion: {:?}", why);
                }
                return Ok(());
            }
        }
        FullEvent::MessageUpdate { event, .. } => {
            handle_message_edit(context, data.clone(), event.id, event.channel_id).await?;
        }
        FullEvent::ReactionAdd { add_reaction } => {
            if should_ignore_event(
                context,
                data.clone(),
                &add_reaction.message_id,
                &add_reaction.channel_id,
            )
            .await?
            {
                return Ok(());
            }

            let sending_channel = match data.config.read().await.starboard.sending_channel {
                Some(ch) => ch,
                None => {
                    warn!("sending channel not set. ignoring message edit event!");
                    return Ok(());
                }
            };

            let requirements = if let Some(override_setting) = data
                .config
                .read()
                .await
                .starboard
                .overrides
                .get(&add_reaction.channel_id.get())
            {
                override_setting.requirement
            } else {
                data.config.read().await.starboard.requirement
            };

            let emojis = &data.config.read().await.starboard.emoji;

            let (new_message, total_reaction) = {
                let message = match context
                    .cache
                    .message(add_reaction.channel_id, add_reaction.message_id)
                {
                    Some(msg) => msg,
                    None => {
                        warn!(
                            "ignoring message {}/{} - not in message cache.",
                            add_reaction.channel_id, add_reaction.message_id
                        );
                        return Ok(());
                    }
                };
                // get total reactions
                let total_reaction = message
                    .reactions
                    .iter()
                    .filter(|reaction| match &reaction.reaction_type {
                        ReactionType::Unicode(emj) => emojis.contains(emj),
                        _ => false,
                    })
                    .fold(0, |acc, react| acc + react.count);
                (
                    create_new_starboard_embed_message(
                        &message.reactions,
                        &message.link(),
                        &message.author,
                        &message.content,
                        &message.attachments,
                        &message.timestamp,
                        message.id,
                    ),
                    total_reaction,
                )
            };

            if total_reaction.ge(&(requirements as u64)) {
                if let Some(board) = data
                    .message_map
                    .original_to_board(add_reaction.message_id.get())?
                {
                    info!(
                        "updating board message {} from reaction added to {}/{}",
                        board, add_reaction.channel_id, add_reaction.message_id
                    );
                    let mut board_message = context
                        .http
                        .get_message(sending_channel.into(), board.into())
                        .await?;
                    board_message.edit(context, new_message).await?;
                    return Ok(());
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
    if data.message_map.is_blacklisted(message.get())? {
        return Ok(true);
    }

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
    let (timestamp, message_id) = match context.cache.message(channel, message) {
        Some(msg) => (msg.timestamp, msg.id),
        None => {
            warn!(
                "ignoring message {}/{} - not in message cache.",
                channel, message
            );
            return Ok(true);
        }
    };

    if let Some(older_than) = data.config.read().await.starboard.ignore_older_than {
        let older_time_stamp = Timestamp::now().to_utc().sub(older_than);
        if timestamp.to_utc() > older_time_stamp {
            info!("ignoring message {:?}/{:?} - too old!", channel, message_id);
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn handle_message_edit(
    context: &Context,
    data: Arc<UserData>,
    message: MessageId,
    channel: ChannelId,
) -> Result<(), Error> {
    if should_ignore_event(context, data.clone(), &message, &channel).await? {
        return Ok(());
    }

    // see if we have this message logged
    if let Some(board) = data.message_map.original_to_board(message.get())? {
        info!("Source message {} edited, updating message.", message.get());

        let sending_channel = match data.config.read().await.starboard.sending_channel {
            Some(ch) => ch,
            None => {
                warn!("sending channel not set. ignoring message edit event!");
                return Ok(());
            }
        };
        match context
            .http
            .get_message(sending_channel.into(), board.into())
            .await
        {
            Ok(mut message) => {
                let editmsg = create_new_starboard_embed_message(
                    &message.reactions,
                    &message.link(),
                    &message.author,
                    &message.content,
                    &message.attachments,
                    &message.timestamp,
                    message.id,
                );
                message.edit(context, editmsg).await?;
            }
            Err(why) => {
                error!(
                    "The starboard message {:?} does not exist: {:?} ",
                    board, why
                );
                return Ok(());
            }
        }
    }

    Ok(())
}

pub async fn handle_message_deletion(
    context: &Context,
    data: Arc<UserData>,
    message: &MessageId,
    channel: &ChannelId,
) -> Result<(), Error> {
    if data.message_map.is_blacklisted(message.get())? {
        data.message_map.unblacklist(message.get())?;
    }

    if should_ignore_event(context, data.clone(), message, channel).await? {
        return Ok(());
    }

    // check if message is logged
    if let Some(orig) = data.message_map.board_to_original(message.get())? {
        info!(
            "Starboard message {} deleted, stopping tracking of this message.",
            message.get()
        );
        data.message_map.blacklist(orig)?;
    }
    if let Some(board) = data.message_map.original_to_board(message.get())? {
        info!(
            "Source message {} deleted, stopping tracking of this message.",
            message.get()
        );
        data.message_map.remove_record_board(board)?;
    }

    Ok(())
}

pub fn create_new_starboard_embed_message(
    msg_reactions: &[MessageReaction],
    msg_link: &str,
    author: &User,
    msg_content: &str,
    msg_attachments: &[Attachment],
    msg_timestamp: &Timestamp,
    msg_id: MessageId,
) -> EditMessage {
    let mut message = MessageBuilder::new();
    msg_reactions.iter().for_each(|reaction| {
        match &reaction.reaction_type {
            ReactionType::Unicode(string) => {
                message.push(format!("{}: {}", string, reaction.count));
            }
            _ => {
                info!("unhandled reaction type case when creating starboard embed for message {}. tell peng to update this damn bot.", msg_id.get())
            }
        };
        message.push_line_safe("");
    });

    message.push(msg_link);

    let mut embed = CreateEmbed::new();

    let mut embed_author = CreateEmbedAuthor::new(&author.name);
    if let Some(image_lnk) = author.avatar {
        embed_author = embed_author.icon_url(image_lnk.to_string());
        embed = embed.author(embed_author);
    }

    if !msg_content.is_empty() {
        embed = embed.description(msg_content);
    }

    // we keep going until the first attachment with width/height
    if let Some(img_attachment) = msg_attachments
        .iter()
        .filter(|attachment| attachment.width.is_some() && attachment.height.is_some())
        .nth(0)
    {
        embed = embed.image(&img_attachment.url);
    }

    embed = embed
        .timestamp(msg_timestamp)
        .footer(CreateEmbedFooter::new(msg_id.get().to_string()));

    EditMessage::new().content(message.build()).embed(embed)
}
