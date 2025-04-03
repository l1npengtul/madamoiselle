use crate::commands::{
    exclude_channel, set_emoji, set_ignore_older_than, set_override_requirement, set_requirement,
    set_sending_channel,
};
use crate::config::{Config, Override};
use crate::error::Error;
use crate::error::Error::SetConfigErr;
use crate::event::handle_event;
use crate::message_map::MessageMap;
use figment::Figment;
use figment::providers::{Format, Toml};
use log::{error, info, warn};
use poise::{CreateReply, FrameworkError, FrameworkOptions};
use redb::Database;
use serenity::all::colours::css::{DANGER, WARNING};
use serenity::all::{ClientBuilder, CreateEmbed, CreateEmbedFooter, GatewayIntents, Mentionable};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

mod commands;
mod config;
mod error;
mod event;
mod message_map;

pub struct UserData {
    message_map: MessageMap,
    config: RwLock<Config>,
}

impl UserData {
    pub fn message_map(&self) -> &MessageMap {
        &self.message_map
    }

    pub async fn set_emoji(&self, emoji: Vec<String>) {
        self.config.write().await.starboard.emoji = emoji;
    }

    pub async fn set_requirement(&self, minimum: i32) {
        self.config.write().await.starboard.requirement = minimum;
    }

    pub async fn set_sending_channel(&self, channel: u64) {
        self.config.write().await.starboard.sending_channel = Some(channel);
    }

    pub async fn add_exclude_channel(&self, channel: u64) {
        let mut cfg = self.config.write().await;

        if cfg.starboard.excluded_channels.contains(&channel) {
            return;
        }
        cfg.starboard.excluded_channels.push(channel);
    }

    pub async fn unexclude_channel(&self, channel: u64) {
        let mut cfg = self.config.write().await;
        if let Some(idx) = cfg
            .starboard
            .excluded_channels
            .iter()
            .position(|x| x == &channel)
        {
            cfg.starboard.excluded_channels.remove(idx);
        }
    }

    pub async fn set_override_channel_requirement(&self, ovrd: u64, requirement: i32) {
        let mut cfg = self.config.write().await;

        let over = match cfg.starboard.overrides.get_mut(&ovrd) {
            Some(v) => v,
            None => {
                cfg.starboard.overrides.insert(ovrd, Override::default());
                cfg.starboard.overrides.get_mut(&ovrd).unwrap()
            }
        };

        over.requirement = requirement;
    }

    pub async fn set_older_ignore(&self, duration: Duration) {
        self.config.write().await.starboard.ignore_older_than = Some(duration);
    }

    pub async fn write_config_to_disk(&self) -> Result<(), Error> {
        let config =
            toml::to_string(&self.config.read().await.clone()).map_err(|_| SetConfigErr)?;
        let mut file = File::create("madamoiselle.toml")
            .await
            .map_err(|_| SetConfigErr)?;
        file.write_all(config.as_bytes())
            .await
            .map_err(|_| SetConfigErr)?;

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let config: Config = Figment::new()
        .merge(Toml::file("madamoiselle.toml"))
        .extract()
        .expect("Failed to read configuration file.");

    let db_path = config
        .places
        .db_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("messages.db"));
    let database = match if db_path.exists() {
        // yes i know it is a bug if it isnt a file but idfc
        Database::open(&db_path)
    } else {
        Database::create(&db_path)
    } {
        Ok(db) => {
            info!("Opened database {:?}", db_path);
            db
        }
        Err(why) => {
            error!("Failed to open database: {:?}", why);
            exit(-1);
        }
    };

    let message_map = match MessageMap::new(database) {
        Ok(mm) => mm,
        Err(why) => {
            error!("Failed to open database: {:?}", why);
            exit(-1);
        }
    };

    let user_data = Arc::new(UserData {
        message_map,
        config: RwLock::new(config),
    });
    let user_data2 = user_data.clone();

    let poise = poise::Framework::builder()
        .setup(move |_context, _ready, _framework| {
            Box::into_pin(Box::new(async move { Ok(user_data) }))
        })
        .options(FrameworkOptions {
            commands: vec![
                set_emoji(),
                set_requirement(),
                exclude_channel(),
                set_override_requirement(),
                set_sending_channel(),
                set_ignore_older_than(),
            ],
            on_error: |err| Box::into_pin(Box::new(error_wrapper(err))),
            pre_command: |ctx| Box::into_pin(Box::new(pre_post_command(ctx, true))),
            post_command: |ctx| Box::into_pin(Box::new(pre_post_command(ctx, false))),
            event_handler: |ctx, event, framework, data| {
                Box::into_pin(Box::new(handle_event(ctx, event, framework, data)))
            },
            ..Default::default()
        })
        .build();

    let mut client = ClientBuilder::new(
        user_data2
            .config
            .read()
            .await
            .discord
            .token
            .as_ref()
            .expect("Expected token!"),
        GatewayIntents::GUILDS,
    )
    .framework(poise)
    .await
    .expect("Failed to log in to discord!");

    client.start().await.unwrap();

    // shutdown
    info!("Bot Shutdown!");
    user_data2.write_config_to_disk().await.unwrap();
}

async fn pre_post_command<U>(context: poise::Context<'_, U, Error>, pre: bool) {
    if pre {
        info!("Handling pre command {:?}", context.command())
    } else {
        info!("Handling post command {:?}", context.command())
    }
}

async fn error_wrapper<U>(error: FrameworkError<'_, U, error::Error>) {
    if let Err(why) = error_handler(error).await {
        error!("Failed to handle error: {:?}", why)
    }
}

async fn error_handler<U>(error: FrameworkError<'_, U, error::Error>) -> Result<(), Error> {
    const MAYBE_BOT_ERROR: &str =
        "If you believe this is an error on the bot's end, please contact a developer.";

    const BOT_ERROR: &str =
        "This isn't supposed to happen! If you have the time, please contact a developer.";

    match error {
        FrameworkError::Setup { error, .. } => error!("Failed to complete setup: {error:#}"),

        FrameworkError::EventHandler { error, event, .. } => error!(
            "Failed to handle event {:?}: {error:#}",
            event.snake_case_name(),
        ),

        FrameworkError::Command { error, ctx, .. } => {
            let invocation_string = ctx.invocation_string();

            let description = format!("```\n{error:?}\n```");

            error!("An error occurred whilst executing {invocation_string:?}: {error:#}");

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("An internal error has occurred")
                            .description(description)
                            .footer(CreateEmbedFooter::new(BOT_ERROR))
                            .color(DANGER),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::SubcommandRequired { ctx } => {
            warn!(
                "User attempted to invoke a command, which requires a subcommand, without a subcommand: {:?}",
                ctx.invocation_string()
            );

            let prefix = ctx.prefix();

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("Subcommand required")
                            .description(format!(
                                "You must specify one of the following subcommands:\n\n{}",
                                ctx.command()
                                    .subcommands
                                    .iter()
                                    .map(|subcommand| {
                                        if prefix == ctx.framework().bot_id.mention().to_string() {
                                            format!("- {prefix} `{}`", subcommand.qualified_name)
                                        } else {
                                            format!("- `{prefix}{}`", subcommand.qualified_name)
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            ))
                            .color(WARNING),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::CommandPanic { ctx, .. } => {
            ctx.send(

                CreateReply::default()

                    .embed(

                        CreateEmbed::new()

                            .title("Panicked")

                            .description("A really bad error happened and the bot panicked! You should contact a bot developer and tell them to check the logs.")

                            .color(DANGER),

                    )

                    .reply(true)

                    .ephemeral(true),

            )

                .await?;
        }

        FrameworkError::ArgumentParse {
            error, input, ctx, ..
        } => {
            let invocation_string = ctx.invocation_string();

            let description = match input {
                Some(input) => {
                    format!(
                        "Failed to parse {input:?} from {invocation_string:?} into an argument: {error}"
                    )
                }

                None => {
                    format!("Failed to parse an argument from {invocation_string:?}: {error}")
                }
            };

            warn!("{description}");

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("Failed to parse argument")
                            .description(description)
                            .footer(CreateEmbedFooter::new(MAYBE_BOT_ERROR))
                            .color(WARNING),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::CommandStructureMismatch {
            description, ctx, ..
        } => {
            error!(
                "Mismatch between registered command and poise command for `/{}`: {description}",
                ctx.command.qualified_name,
            );

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("Command structure mismatch")
                            .description(format!("```\n{description}\n```"))
                            .footer(CreateEmbedFooter::new(BOT_ERROR))
                            .color(DANGER),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::CooldownHit {
            remaining_cooldown,

            ctx,
            ..
        } => {
            warn!("User hit cooldown with {:?}", ctx.invocation_string());

            ctx.send(

                CreateReply::default()

                    .embed(

                        CreateEmbed::new()

                            .title("Cooldown hit")

                            .description(format!("You must wait **~{} seconds** before you can use this command again.", remaining_cooldown.as_secs()))

                            .color(WARNING),

                    )

                    .reply(true)

                    .ephemeral(true),

            )

                .await?;
        }

        FrameworkError::MissingBotPermissions {
            missing_permissions,

            ctx,
            ..
        } => {
            warn!(
                "Bot is lacking permissions for {:?}: {missing_permissions}",
                ctx.invocation_string()
            );

            ctx.send(

                CreateReply::default()

                    .embed(

                        CreateEmbed::new()

                            .title("Lacking bot permissions")

                            .description(format!("The bot requires the following permissions to execute this command: **{missing_permissions}**"))

                            .color(WARNING),

                    )

                    .reply(true)

                    .ephemeral(true),

            )

                .await?;
        }

        FrameworkError::MissingUserPermissions {
            missing_permissions,

            ctx,
            ..
        } => match missing_permissions {
            Some(missing_permissions) => {
                warn!(
                    "User is lacking permissions for {:?}: {missing_permissions}",
                    ctx.invocation_string(),
                );

                ctx.send(

                    CreateReply::default()

                        .embed(

                            CreateEmbed::new()

                                .title("Lacking user permissions")

                                .description(format!("You must have the following permissions to execute this command: **{missing_permissions}**"))

                                .color(WARNING),

                        )

                        .reply(true)

                        .ephemeral(true),

                )

                    .await?;
            }

            None => {
                warn!(
                    "User is lacking permissions for {:?}",
                    ctx.invocation_string(),
                );

                ctx.send(

                    CreateReply::default()

                        .embed(

                            CreateEmbed::new()

                                .title("Lacking user permissions")

                                .description("You do not have the permissions needed to execute this command")

                                .color(WARNING),

                        )

                        .reply(true)

                        .ephemeral(true),

                )

                    .await?;
            }
        },

        FrameworkError::NotAnOwner { ctx, .. } => {
            warn!(
                "Non owner attempted to invoke {:?}",
                ctx.invocation_string(),
            );

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("Owner only command")
                            .description("You must be an owner to use this command.")
                            .color(WARNING),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::GuildOnly { ctx, .. } => {
            warn!(
                "User attempted to invoke {:?} outside of a guild",
                ctx.invocation_string(),
            );

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("Server only command")
                            .description("You cannot use this command outside of a server.")
                            .color(WARNING),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::DmOnly { ctx, .. } => {
            warn!(
                "User attempted to invoke {:?} outside of DMs",
                ctx.invocation_string(),
            );

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("DMs only command")
                            .description("You cannot use this command outside of DMs.")
                            .color(WARNING),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::NsfwOnly { ctx, .. } => {
            warn!(
                "User attempted to invoke {:?} outside of an NSFW channel",
                ctx.invocation_string(),
            );

            ctx.send(
                CreateReply::default()
                    .embed(
                        CreateEmbed::new()
                            .title("NSFW command")
                            .description("You cannot use this command outside of an NSFW channel.")
                            .color(WARNING),
                    )
                    .reply(true)
                    .ephemeral(true),
            )
            .await?;
        }

        FrameworkError::CommandCheckFailed { error, ctx, .. } => match error {
            Some(error) => {
                error!("Check errored for {:?}: {error:#}", ctx.invocation_string());

                ctx.send(
                    CreateReply::default()
                        .embed(
                            CreateEmbed::new()
                                .title("Failed to perform check")
                                .description(format!("```\n{error:?}\n```"))
                                .footer(CreateEmbedFooter::new(BOT_ERROR))
                                .color(DANGER),
                        )
                        .reply(true)
                        .ephemeral(true),
                )
                .await?;
            }

            None => {
                warn!("Check failed for {:?}", ctx.invocation_string());
            }
        },

        FrameworkError::DynamicPrefix { error, msg, .. } => {
            error!("Dynamic prefix failed for {msg:?}: {error:#}");
        }

        FrameworkError::UnknownCommand {
            prefix,

            msg_content,
            ..
        } => {
            warn!("Recognized prefix {prefix:?} but did not recognize command {msg_content:?}");
        }

        FrameworkError::UnknownInteraction { interaction, .. } => {
            warn!(
                "Received interaction for an unknown command: {:?}",
                interaction.data.name,
            );
        }

        other => {
            warn!(
                "Not prepared to handle unfamiliar kind of error, falling back to default `on_error` function"
            );

            poise::builtins::on_error(other).await?;
        }
    }

    Ok(())
}
