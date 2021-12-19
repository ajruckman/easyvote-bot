use std::collections::HashMap;
use std::sync::Arc;

use evlog::meta;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::guild::Guild;
use serenity::model::id::GuildId;
use serenity::model::interactions::{Interaction, InteractionResponseType, InteractionType};
use serenity::prelude::TypeMapKey;

use crate::commands;
use crate::db::dbclient::DBClient;
use crate::runtime::get_logger;

pub struct BotData {
    pub db_client: Arc<DBClient>,
}

impl BotData {
    pub async fn new(db_client: DBClient) -> Self {
        let db_client = Arc::new(db_client);

        Self {
            db_client,
        }
    }
}

impl TypeMapKey for BotData {
    type Value = BotData;
}

pub struct BotHandler {}

#[async_trait]
impl EventHandler for BotHandler {
    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {}

    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: bool) {
        get_logger().info("Guild ready.", meta![
            "ID" => guild.id,
            "Name" => guild.name,
        ]);

        let data = ctx.data.read().await;
        let data = data.get::<BotData>().unwrap();

        let existing_cmds = guild.get_application_commands(&ctx).await.unwrap();

        let existing_map = existing_cmds.iter()
            .map(|v| (v.name.clone(), v))
            .collect::<HashMap<_, _>>();

        for cmd in commands::COMMANDS {
            let whitelisted = match cmd.whitelisted_servers {
                None => true,
                Some(servers) => servers.iter().any(|v| v.as_u64() == guild.id.as_u64()),
            };

            if !whitelisted {
                get_logger().debug("Command is not allowed in this server.", meta! {
                    "GuildID" => guild.id,
                    "GuildName" => guild.name,
                    "Command" => cmd.name
                });
                continue;
            }

            if existing_map.contains_key(cmd.name) {
                if !cmd.re_register {
                    get_logger().debug("Command already registered in this server.", meta! {
                        "GuildID" => guild.id,
                        "GuildName" => guild.name,
                        "Command" => cmd.name
                    });
                    continue;
                }
            }

            let created = guild.create_application_command(&ctx.http, |c| {
                (cmd.builder)(c)
            }).await.unwrap();

            get_logger().debug("Registered command in server.", meta! {
                "GuildID" => guild.id,
                "GuildName" => guild.name,
                "Command" => cmd.name,
                "ID" => created.id
            });
        }

        let partial_guild = ctx.http.get_guild(*guild.id.as_u64()).await.unwrap();
        crate::support::register_polls::register_polls(data.db_client.conn(), &ctx, &partial_guild).await.unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(interaction) = interaction {
            let guild = ctx.cache.guild(interaction.guild_id.unwrap()).await.unwrap();

            if interaction.kind == InteractionType::Ping {
                get_logger().info("Interaction ping.", meta! {
                    "GuildID" => guild.id,
                    "GuildName" => guild.name,
                    "InteractionID" => interaction.id
                });

                interaction.create_interaction_response(ctx.http.as_ref(), |r| {
                    r.kind(InteractionResponseType::Pong)
                }).await.unwrap();
            } else if interaction.kind == InteractionType::ApplicationCommand {
                get_logger().info("Interaction ping.", meta! {
                    "GuildID" => guild.id,
                    "GuildName" => guild.name,
                    "InteractionID" => interaction.id,
                    "CommandID" => interaction.data.id,
                    "CommandName" => interaction.data.name
                });

                let handler = match commands::get_handler(&interaction.data.name) {
                    None => return,
                    Some(v) => v,
                };

                let interaction_id = interaction.id;
                let command_id = interaction.data.id.clone();
                let command_name = interaction.data.name.clone();

                let r: anyhow::Result<()> = handler(ctx, interaction).await;
                match r {
                    Ok(()) => {}
                    Err(e) => {
                        get_logger().error("Error occurred in interaction processor.", meta! {
                            "GuildID" => guild.id,
                            "GuildName" => guild.name,
                            "InteractionID" => interaction_id,
                            "CommandID" => command_id,
                            "CommandName" => command_name,
                            "Error" => e,
                        });
                    }
                }
            }
        }
    }
}
