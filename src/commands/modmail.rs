use log::warn;
use poise::{Context, CreateReply};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, CreateSelectMenu,
    CreateSelectMenuKind, CreateSelectMenuOption, EditThread, MessageId, RoleId, UserId,
};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;

use crate::database::Database;
use crate::error::Error;
use crate::{UserData, log_channel};

pub const CREATE_SELECT_MENU_ID: &str = "MADAMOISELLE_CAFE_WAITTHEY";

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum ModmailOpenReason {
    Moderation,
    Suggestion,
    Other,
}

impl Display for ModmailOpenReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ModmailOpenReason::Moderation => "MODERATION",
            ModmailOpenReason::Suggestion => "SUGGESTION",
            ModmailOpenReason::Other => "OTHER",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ModmailOpenReason {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MODERATION" => Ok(ModmailOpenReason::Moderation),
            "SUGGESTION" => Ok(ModmailOpenReason::Suggestion),
            "OTHER" => Ok(ModmailOpenReason::Other),
            _ => Err(Error::BadReason),
        }
    }
}

impl Into<String> for ModmailOpenReason {
    fn into(self) -> String {
        format!("{self}")
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Serialize, Deserialize)]
pub enum ModmailStatus {
    Open,
    Resolved,
    WontFix,
}

impl Display for ModmailStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ModmailStatus::Open => "OPEN",
            ModmailStatus::Resolved => "RESOLVED",
            ModmailStatus::WontFix => "WONTFIX",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ModmailStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OPEN" => Ok(ModmailStatus::Open),
            "RESOLVED" => Ok(ModmailStatus::Resolved),
            "WONTFIX" => Ok(ModmailStatus::WontFix),
            _ => Err(Error::BadStatus),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModmailThread {
    pub thread_id: ChannelId,
    pub creator_id: UserId,
    pub creation_date: i64,
    pub status: ModmailStatus,
    pub reason: ModmailOpenReason,
    pub resolution_date: Option<i64>,
    pub closing_notes: Option<String>,
    pub closing_user: Option<UserId>,
}

impl ModmailThread {
    pub fn new(
        thread_id: ChannelId,
        creator_id: UserId,
        creation_date: i64,
        reason: ModmailOpenReason,
    ) -> Self {
        Self {
            thread_id,
            creator_id,
            creation_date,
            status: ModmailStatus::Open,
            reason,
            resolution_date: None,
            closing_notes: None,
            closing_user: None,
        }
    }

    pub fn values(
        &self,
    ) -> (
        i64,
        i64,
        i64,
        String,
        String,
        Option<i64>,
        Option<&String>,
        Option<i64>,
    ) {
        (
            self.thread_id.get() as i64,
            self.creator_id.get() as i64,
            self.creation_date,
            self.status.to_string(),
            self.reason.to_string(),
            self.resolution_date,
            self.closing_notes.as_ref(),
            self.closing_user.map(|x| x.get() as i64),
        )
    }
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_log_channel(
    context: Context<'_, Arc<UserData>, Error>,
    #[description = "New Channel"] new_channel: Option<ChannelId>,
) -> Result<(), Error> {
    let new_channel = new_channel.unwrap_or(context.channel_id()).get();
    let new_guild_channel = match context
        .http()
        .get_channel(ChannelId::new(new_channel))
        .await
    {
        Ok(ch) => match ch.guild() {
            Some(g) => g,
            None => {
                context
                    .reply("The requested channel is not a guild channel")
                    .await?;
                return Ok(());
            }
        },
        Err(why) => return Err(Error::Discord(why)),
    };
    new_guild_channel
        .send_message(
            context.http(),
            CreateMessage::new().content("Set log channel here."),
        )
        .await?;
    context.reply("Set new log channel.").await?;
    context.data().set_modmail_log_channel(new_channel).await;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn set_modmail_channel(
    context: Context<'_, Arc<UserData>, Error>,
    #[description = "New Channel"] new_channel: Option<ChannelId>,
) -> Result<(), Error> {
    let new_channel = new_channel.unwrap_or(context.channel_id()).get();
    let new_guild_channel = match context
        .http()
        .get_channel(ChannelId::new(new_channel))
        .await
    {
        Ok(ch) => match ch.guild() {
            Some(g) => g,
            None => {
                context
                    .reply("The requested channel is not a guild channel")
                    .await?;
                return Ok(());
            }
        },
        Err(why) => return Err(Error::Discord(why)),
    };
    if let Some(bot_msg) = context.data().config.read().await.modmail.bot_message {
        if let Some(old_channel_id) = context.data().config.read().await.modmail.channel {
            if let Ok(message) = context
                .http()
                .get_message(ChannelId::new(old_channel_id), MessageId::new(bot_msg))
                .await
            {
                context
                    .send(
                        context
                            .reply_builder(CreateReply::default())
                            .content("Warning: Deleted the old message.")
                            .reply(true)
                            .ephemeral(true),
                    )
                    .await?;
                context.data().clear_modmail_bot_message().await;
                message.delete(context.http()).await?;
            }
        }
    }
    context.data().set_modmail_send_channel(new_channel).await;
    // create new message
    let bot_pfp = context.cache().current_user().default_avatar_url();
    let new_message = new_guild_channel
        .send_message(
            context.http(),
            CreateMessage::new()
                .embed(waitress_embed(&bot_pfp))
                .select_menu(create_interaction_menu()),
        )
        .await?;

    context
        .data()
        .set_modmail_bot_message(new_message.id.get())
        .await;

    context
        .send(
            context
                .reply_builder(CreateReply::default())
                .content(format!("OK - created new message {}", new_message.link()))
                .reply(true)
                .ephemeral(true),
        )
        .await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn modmail_auto_roles(context: Context<'_, Arc<UserData>, Error>) -> Result<(), Error> {
    let roles = context
        .data()
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
    warn!("getting roles");
    context.reply(format!("Current Roles: {roles}")).await?;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn modmail_add_role(
    context: Context<'_, Arc<UserData>, Error>,
    role: RoleId,
) -> Result<(), Error> {
    let roles = context.data().config.read().await.modmail.roles.clone();
    warn!("adding role...");
    if !roles.contains(&role.get()) {
        let mut new_roles = roles.clone();
        new_roles.push(role.get());
        warn!("set data");
        context.data().set_modmail_roles(new_roles).await;
        warn!("reply");
        context
            .reply(format!("Added new role <@&{}>", role.get()))
            .await?;
    } else {
        context.reply("Role already exists").await?;
    }
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn modmail_remove_role(
    context: Context<'_, Arc<UserData>, Error>,
    role: RoleId,
) -> Result<(), Error> {
    let roles = context.data().config.read().await.modmail.roles.clone();
    warn!("removing role...");
    if let Some(index) = roles.iter().position(|item| item == &role.get()) {
        let mut new_roles = roles.clone();
        new_roles.remove(index);
        context.data().set_modmail_roles(new_roles).await;
        context
            .reply(format!("Removed role <@&{}>", role.get()))
            .await?;
    } else {
        context.reply("Could not find role").await?;
    }
    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn resolve(context: Context<'_, Arc<UserData>, Error>) -> Result<(), Error> {
    let mut channel = match context.guild_channel().await {
        Some(c) => c,
        None => {
            context.reply("Can only be used in a guild").await?;
            return Ok(());
        }
    };
    modify_modmail_status(
        &context.data().database,
        context.channel_id(),
        ModmailStatus::Resolved,
        None,
        context.author().id,
        context.created_at().unix_timestamp(),
    )
    .await?;
    context.reply("Closing this modmail now.").await?;
    channel
        .edit_thread(
            context.http(),
            EditThread::new()
                .archived(true)
                .locked(true)
                .audit_log_reason("User requested thread close"),
        )
        .await?;
    log_channel(
        context.serenity_context(),
        context.data(),
        format!(
            "Closing modmail <#{}>, <@{}> says the issue was resolved.",
            channel.id.get(),
            context.author().id.get()
        ),
    )
    .await;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn mark_resolved(
    context: Context<'_, Arc<UserData>, Error>,
    notes: Option<String>,
) -> Result<(), Error> {
    let mut channel = match context.guild_channel().await {
        Some(c) => c,
        None => {
            context.reply("Can only be used in a guild").await?;
            return Ok(());
        }
    };
    modify_modmail_status(
        &context.data().database,
        context.channel_id(),
        ModmailStatus::Resolved,
        notes.clone(),
        context.author().id,
        context.created_at().unix_timestamp(),
    )
    .await?;
    context.reply("Marking this modmail as resolved.").await?;
    channel
        .edit_thread(
            context.http(),
            EditThread::new()
                .archived(true)
                .locked(true)
                .audit_log_reason("Moderator request thread mark as close"),
        )
        .await?;
    
    log_channel(
        context.serenity_context(),
        context.data(),
        format!(
            "Closing modmail <#{}>, moderator <@{}> marked this as resolved. Reason: {:?}",
            channel.id.get(),
            context.author().id.get(),
            notes
        ),
    )
    .await;
    Ok(())
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn mark_wontfix(
    context: Context<'_, Arc<UserData>, Error>,
    notes: Option<String>,
) -> Result<(), Error> {
    let mut channel = match context.guild_channel().await {
        Some(c) => c,
        None => {
            context.reply("Can only be used in a guild").await?;
            return Ok(());
        }
    };
    modify_modmail_status(
        &context.data().database,
        context.channel_id(),
        ModmailStatus::WontFix,
        notes.clone(),
        context.author().id,
        context.created_at().unix_timestamp(),
    )
    .await?;
    context.reply("Marking this modmail as Won't Fix.").await?;
    channel
        .edit_thread(
            context.http(),
            EditThread::new()
                .archived(true)
                .locked(true)
                .audit_log_reason("Moderator requested thread to mark as wontfix"),
        )
        .await?;
    log_channel(
        context.serenity_context(),
        context.data(),
        format!(
            "Closing modmail <#{}>, moderator <@{}> marked this as WONTFIX. Reason: {:?}",
            channel.id.get(),
            context.author().id.get(),
            notes
        ),
    )
    .await;
    Ok(())
}

pub async fn modify_modmail_status(
    database: &Database,
    modmail_id: ChannelId,
    modmail_status: ModmailStatus,
    notes: Option<String>,
    closing_user: UserId,
    closing_time: i64,
) -> Result<(), Error> {
    let mut modmail = match database.find_modmail_by_id(modmail_id).await? {
        Some(m) => m,
        None => return Err(Error::NotFound),
    };

    if let Some(notes) = notes {
        modmail.closing_notes = Some(notes);
    }

    modmail.status = modmail_status;
    modmail.closing_user = Some(closing_user);
    modmail.resolution_date = Some(closing_time);

    database.set_modmail(modmail).await?;

    Ok(())
}

pub fn create_interaction_menu() -> CreateSelectMenu {
    CreateSelectMenu::new(
        CREATE_SELECT_MENU_ID,
        CreateSelectMenuKind::String {
            options: vec![
                CreateSelectMenuOption::new("Suggestion", ModmailOpenReason::Suggestion),
                CreateSelectMenuOption::new("Moderation", ModmailOpenReason::Moderation),
            ],
        },
    )
}

pub fn waitress_embed(pfp: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title("Call for a Waitress")
        .description("Call for Support at the Madamoiselle Cafe")
        .footer(
            CreateEmbedFooter::new("Madamoiselle Cafe - Serving Flesh Coffee Est. 2018")
                .icon_url(pfp),
        )
}
