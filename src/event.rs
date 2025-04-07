use crate::UserData;
use crate::error::Error;
use log::{error, info, warn};
use poise::FrameworkContext;
use serenity::all::{Attachment, CacheHttp, Channel, ChannelId, Context, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, EditMessage, Embed, FullEvent, Message, MessageBuilder, MessageId, MessageReaction, ReactionType, Timestamp, User};
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
        FullEvent::MessageUpdate { event, new, .. } => {
            handle_message_edit(context, data.clone(), event.id, event.channel_id, new.clone()).await?;
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

            let requirements = if let Some(override_setting) = data
                .config
                .read()
                .await
                .starboard
                .overrides
                .get(&add_reaction.channel_id.get().to_string())
            {
                override_setting.requirement
            } else {
                data.config.read().await.starboard.requirement
            };

            let emojis = &data.config.read().await.starboard.emoji;

            let (mut msg_build, embed) = {
                let message = match context.http().get_message(add_reaction.channel_id, add_reaction.message_id).await
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

                info!("got message {}", message.id);
                info!("message embeds {:?}", message.embeds);
                // get total reactions
                let total_reaction = message
                    .reactions
                    .iter()
                    .filter(|reaction| {
                        info!("working on reaction {:?}", reaction);
                        match &reaction.reaction_type {
                            ReactionType::Unicode(emj) => {
                                info!("{:?}, {}", emojis, emj);
                                let result = emojis.contains(emj);
                                info!("{}", result);
                                result
                            },
                            _ => false,
                        }
                    })
                    .fold(0, |acc, react| { info!("react.count {} + acc {} = {}",  react.count, acc, react.count + acc); react.count + acc });

                if total_reaction < requirements {
                    info!("not enough reactions on message {} to add to starboard: {}<{}", add_reaction.message_id, total_reaction, requirements)
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
                    .message_map
                    .original_to_board(add_reaction.message_id.get())?
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
                    info!("creating new board message from reaction added to {}/{}",add_reaction.channel_id, add_reaction.message_id);

                    let new_message = CreateMessage::new().content(msg_build.build()).embed(embed);

                    if let Channel::Guild(guild_ch) = context.http().get_channel(sending_channel.into()).await? {
                        info!("sending message to #{} ({})", guild_ch.name, guild_ch.id);
                        let sent = guild_ch.send_message(context, new_message).await?;
                        data.message_map.record_new(add_reaction.message_id.get(), sent.id.get())?;
                        info!("sent message {} to #{}, logged sucessfully!", sent.id, guild_ch.name);
                    } else {
                        error!("failed to get channel - {} is not guild channel", sending_channel);
                        return Ok(())
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
            warn!("Ratelimited {}! for {} seconds, is global: {}", data.path, data.timeout.as_secs(), data.global);
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
        info!("message blacklisted ignoring,");
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
    let message_got = context.http().get_message(*channel, *message).await?;

    if let Some(older_than) = data.config.read().await.starboard.ignore_older_than {
        let older_time_stamp = Timestamp::now().to_utc().sub(older_than);
        if message_got.timestamp.to_utc() < older_time_stamp {
            info!("ignoring message {:?}/{:?} - too old!", channel, message_got.id);
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
    new_msg: Option<Message>,
) -> Result<(), Error> {
    if should_ignore_event(context, data.clone(), &message, &channel).await? {
        warn!("ignoring edit event for message {}", message);
        return Ok(());
    }

    let original_edited_message = match new_msg {
        Some(m) => m,
        None => {
            context.http().get_message(channel, message).await?
        }
    };

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
            .http()
            .get_message(sending_channel.into(), board.into())
            .await
        {
            Ok(mut message) => {
                let (mut new_msg, embed) = create_new_starboard_embed_message(
                    &original_edited_message.reactions,
                    &original_edited_message.link(),
                    &original_edited_message.author,
                    &original_edited_message.content,
                    &original_edited_message.attachments,
                    &original_edited_message.embeds,
                    &original_edited_message.timestamp,
                    original_edited_message.id,
                );

                let new_message = EditMessage::new().content(new_msg.build()).embed(embed);

                message.edit(context, new_message).await?;
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
        let message = match context
            .http()
            .get_message(data.config.read().await.starboard.sending_channel.unwrap_or_default().into(), board.into())
            .await
        {
            Ok(msg) => msg,
            Err(why) => {
                error!(
                            "failed to get message {}/{} - {}.",
                            channel, message, why
                        );
                return Ok(());
            }
        };

        message.delete(context).await?;
    }

    Ok(())
}

pub fn create_new_starboard_embed_message(
    msg_reactions: &[MessageReaction],
    msg_link: &str,
    author: &User,
    msg_content: &str,
    msg_attachments: &[Attachment],
    msg_embeds: &[Embed],
    msg_timestamp: &Timestamp,
    msg_id: MessageId,
) -> (MessageBuilder, CreateEmbed) {
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

    message.push_safe(msg_link);

    let mut embed = CreateEmbed::new();

    let mut embed_author = CreateEmbedAuthor::new(&author.name);
    if let Some(image_lnk) = author.avatar_url() {
        embed_author = embed_author.icon_url(image_lnk);
        embed = embed.author(embed_author);
    }

    if !msg_content.is_empty() {
        let trimmed_content = truncate(msg_content, 4096);
        embed = embed.description(trimmed_content);
    }

    // we keep going until the first attachment with width/height
    if let Some(img_attachment) = msg_attachments
        .iter()
        .filter(|attachment| attachment.width.is_some() && attachment.height.is_some())
        .nth(0)
    {
        embed = embed.image(&img_attachment.url);
    } else {
        if let Some(Some(image_embed)) = msg_embeds.iter().filter(|embed| embed.image.is_some()).nth(0).map(|embed| &embed.image) {
            info!("got embed image: {}", image_embed.url);
            embed = embed.image(&image_embed.url);
        }

        if let Some(Some(embed_thumbnail)) =  msg_embeds.iter().filter(|embed| embed.thumbnail.is_some()).nth(0).map(|x| &x.thumbnail) {
            info!("got embed thumbnail: {}", embed_thumbnail.url);
            embed = embed.image(&embed_thumbnail.url);
        }



        info!("looking for embed images...");

    }

    embed = embed
        .timestamp(msg_timestamp)
        .footer(CreateEmbedFooter::new(msg_id.get().to_string()));
    (message, embed)
}

// pub struct MessageGotten {
//     pub id: MessageId,
//     pub reactions: Vec<MessageReaction>,
//     pub link: String,
//     pub author: User,
//     pub content: String,
//     pub attachment: Vec<Attachment>,
//     pub timestamp: Timestamp,
//     pub embed: Vec<Embed>,
// }

// async fn get_message(context: &Context, channel_id: ChannelId, message_id: MessageId) -> Result<MessageGotten, Error> {
//     if let Some(msgref) = context.cache.message(channel_id, message_id) {
//         return Ok(MessageGotten {
//             id: msgref.id,
//             reactions: msgref.reactions.clone(),
//             link: msgref.link(),
//             author: msgref.author.clone(),
//             content: msgref.content.clone(),
//             attachment: msgref.attachments.clone(),
//             timestamp: msgref.timestamp,
//             embed: msgref.embeds.clone(),
//         })
//     }
//
//     match context.http().get_message(channel_id, message_id).await {
//         Ok(msg) => {
//             Ok(MessageGotten {
//                 id: msg.id,
//                 reactions: msg.reactions.clone(),
//                 link: msg.link(),
//                 author: msg.author.clone(),
//                 content: msg.content.clone(),
//                 attachment: msg.attachments.clone(),
//                 timestamp: msg.timestamp,
//                 embed: msg.embeds,
//             })
//         }
//         Err(why) => {
//             error!("Failed to get message {}/{} - {}", channel_id, message_id, why);
//             Err(Error::Discord(why))
//         }
//     }
// }

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
