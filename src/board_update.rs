use std::sync::Arc;

use log::{info, warn};
use serenity::{
    Client,
    all::{ChannelId, Message, MessageId, MessageReaction, ReactionType},
};

use crate::{UserData, error::Error};

pub async fn do_board_update(
    userdata: Arc<UserData>,
    client: &Client,
    channel_id: ChannelId,
    message_id: MessageId,
) -> Result<(), Error> {
    let message = client.http.get_message(channel_id, message_id).await?;

    Ok(())
}

pub async fn handle_message_edit(
    client: &Client,
    data: Arc<UserData>,
    message: Message,
) -> Result<(), Error> {
    let requirement = get_channel_requirements(data.clone(), message.channel_id).await;
    let message_reaction = message_reactions(
        &data.config.read().await.starboard.emoji,
        &message.reactions,
    );

    if message_reaction < requirement {
        info!(
            "not enough reactions on message {} to add to starboard: {}<{}",
            message.id.get(),
            message_reaction,
            requirement
        );
        return Ok(());
    }

    // see if we have this message logged
    if let Some(board) = data.database.original_to_board(message.id.get()).await? {
        info!(
            "Source message {} edited, updating message.",
            message.id.get()
        );

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
                error!("The starboard message {board} does not exist: {why} ");
                return Ok(());
            }
        }
    }

    Ok(())
}

pub async fn handle_message_deletion(
    client: &Client,
    data: Arc<UserData>,
    message: Message,
) -> Result<(), Error> {
    if let Some(board) = data.database.original_to_board(message.id.get()).await? {
        info!(
            "Source message {} deleted, stopping tracking of this message.",
            message.id.get()
        );
        data.database.remove_record_board(board).await?;

        message.delete(&client.http).await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
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
        .find(|attachment| attachment.width.is_some() && attachment.height.is_some())
    {
        embed = embed.image(&img_attachment.url);
    } else {
        if let Some(Some(image_embed)) = msg_embeds
            .iter()
            .find(|embed| embed.image.is_some())
            .map(|embed| &embed.image)
        {
            info!("got embed image: {}", image_embed.url);
            embed = embed.image(&image_embed.url);
        }

        if let Some(Some(embed_thumbnail)) = msg_embeds
            .iter()
            .find(|embed| embed.thumbnail.is_some())
            .map(|x| &x.thumbnail)
        {
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

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

fn message_reactions(allow_emojis: &[String], message_reactions: &[MessageReaction]) -> u64 {
    message_reactions
        .iter()
        .filter(|reaction| match &reaction.reaction_type {
            ReactionType::Unicode(emj) => {
                info!("{allow_emojis:?}, {emj}");
                let result = allow_emojis.contains(emj);
                info!("{result}");
                result
            }
            _ => false,
        })
        .fold(0, |acc, react| react.count + acc)
}

async fn get_channel_requirements(data: Arc<UserData>, channel: ChannelId) -> u64 {
    if let Some(Some(override_setting)) = data
        .config
        .read()
        .await
        .starboard
        .overrides
        .get(&channel.get().to_string())
        .map(|requirement| requirement.requirement)
    {
        override_setting
    } else {
        data.config.read().await.starboard.requirement
    }
}
