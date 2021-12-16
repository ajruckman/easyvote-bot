use std::sync::Arc;

use evlog::meta;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, UserId};
use serenity::model::interactions::application_command::{ApplicationCommandInteraction, ApplicationCommandOptionType};
use serenity::model::prelude::application_command::ApplicationCommandInteractionDataOption;

use crate::db;
use crate::db::dbclient::DBClient;
use crate::handler::BotData;
use crate::helpers::{command_opt, command_resp};
use crate::runtime::get_logger;

pub const POLL: &str = "poll";

static VALIDATE_POLL_NAME: Lazy<Regex> = Lazy::new(|| Regex::new("^[a-z0-9-]+$").unwrap());

pub fn poll_builder(cmd: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    cmd.name(POLL)
        .description("Manage polls")
        .create_option(|opt| {
            opt
                .name("create")
                .description("Create a new poll")
                .kind(ApplicationCommandOptionType::SubCommand)

                .create_sub_option(|opt| opt
                    .name("name")
                    .description("Unique identifier for this poll; no whitespace, may only contain a-z, 0-9, and -")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String))
                .create_sub_option(|opt| opt
                    .name("question")
                    .description("The question you want users to vote on")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String))
                .create_sub_option(|opt| opt
                    .name("opt1")
                    .description("Option 1")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String))
                .create_sub_option(|opt| opt
                    .name("opt2")
                    .description("Option 2")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String));

            for i in 3..=23 {
                opt.create_sub_option(|opt| opt
                    .name(format!("opt{}", i))
                    .description(format!("Option {}", i))
                    .required(false)
                    .kind(ApplicationCommandOptionType::String));
            }

            opt
        });

    cmd
}

async fn poll_create(ctx: &Context, interaction: &ApplicationCommandInteraction, opt: &ApplicationCommandInteractionDataOption, data: &BotData, guild_id: &GuildId, member: &Member) -> anyhow::Result<()> {
    let name = command_opt::find_required(&ctx, &interaction, &opt.options, command_opt::find_string_opt, "name").await?.unwrap();
    let question = command_opt::find_required(&ctx, &interaction, &opt.options, command_opt::find_string_opt, "question").await?.unwrap();

    //

    let name = name.trim().to_lowercase();
    match VALIDATE_POLL_NAME.is_match(&name) {
        true => {}
        false => {
            get_logger().info("Invalid name passed to /poll create.", meta! {
                        "InteractionID" => interaction.id,
                        "Name" => name,
                    });
            command_resp::reply_deferred_result(&ctx, &interaction, format!(
                "Name '{}' is invalid; may only contain letters, numbers, and dashes (-)", name
            )).await?;
            return Ok(());
        }
    }

    //

    if db::model::check_server_has_poll_name(data.db_client.conn(), *guild_id.as_u64(), &name).await? {
        get_logger().info("Attempted to register duplicate server/name combo with /poll create.", meta! {
                    "InteractionID" => interaction.id,
                    "Name" => name,
                });
        command_resp::reply_deferred_result(&ctx, &interaction, format!(
            "A poll with the name `{}` has already been created in this server.", name
        )).await?;
        return Ok(());
    }

    //

    let mut opts = Vec::new();

    for i in 1..=23 {
        let opt = command_opt::find_string_opt(&opt.options, &format!("opt{}", i));

        match opt {
            None => {}
            Some(v) => {
                let v = v.trim();

                if v.is_empty() {
                    get_logger().info("Empty option passed to /poll create.", meta! {
                                "InteractionID" => interaction.id,
                                "OptionNumber" => i,
                            });
                    command_resp::reply_deferred_result(&ctx, &interaction, format!("Option `{}` was empty.", i)).await?;
                    return Ok(());
                }

                opts.push(v.to_owned());
            }
        }
    }

    if opts.len() < 2 {
        get_logger().info("Fewer than 2 options passed to /poll create.", meta! {
                    "InteractionID" => interaction.id,
                    "Name" => name,
                });
        command_resp::reply_deferred_result(&ctx, &interaction, "At least 2 options must be passed to /poll create.").await?;
        return Ok(());
    }

    let poll = db::model::add_poll(
        data.db_client.conn(),
        *guild_id.as_u64(),
        *member.user.id.as_u64(),
        &name,
        &question,
        &opts,
    ).await?;

    interaction.create_followup_message(&ctx.http, |r| r.create_embed(|e| {
        e.author(|a| {
            a.name("easyVote");
            a.icon_url("https://i.imgur.com/fWgQ8b6.png");

            a
        });

        e.title("New poll created");
        e.field("ID", poll.id, false);

        let mut opt_string = String::new();
        for (i, opt) in opts.iter().enumerate() {
            opt_string.push_str(&format!("**{}.** {}\n", i + 1, opt));
        }
        e.field("Options", opt_string, false);

        e
    })).await?;

    let guild = ctx.http.get_guild(*guild_id.as_u64()).await?;
    crate::support::register_polls::register_polls(data.db_client.conn(), &ctx, &guild).await.unwrap();

    Ok(())
}

pub async fn poll(ctx: Context, interaction: ApplicationCommandInteraction) -> anyhow::Result<()> {
    command_resp::reply_deferred_ack(&ctx, &interaction).await?;

    let data = ctx.data.read().await;
    let data = data.get::<BotData>().unwrap();

    let guild_id = interaction.guild_id.as_ref().unwrap();
    let member = interaction.member.as_ref().unwrap();

    //

    let sub = &interaction.data.options[0];

    match sub.name.as_str() {
        "create" => {
            poll_create(&ctx, &interaction, sub, data, guild_id, member).await?;
        }
        _ => {}
    }

    //

    Ok(())
}
