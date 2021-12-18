use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use evlog::meta;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, UserId};
use serenity::model::interactions::application_command::{ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType};
use serenity::model::Permissions;
use serenity::model::prelude::application_command::ApplicationCommandInteractionDataOption;

use crate::db;
use crate::db::dbclient::DBClient;
use crate::db::schema::Poll;
use crate::handler::BotData;
use crate::helpers::{command_opt, command_resp};
use crate::runtime::get_logger;
use crate::support::numbers::num_word;

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
                    .name("ranks")
                    .description("How many choices to allow (suggested: 3, min: 2, max: 20)")
                    .required(true)
                    .kind(ApplicationCommandOptionType::Integer))
                .create_sub_option(|opt| opt
                    .name("opt-1")
                    .description("Option 1")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String))
                .create_sub_option(|opt| opt
                    .name("opt-2")
                    .description("Option 2")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String))
                .create_sub_option(|opt| opt
                    .name("opt-3")
                    .description("Option 3")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String));

            for i in 4..=20 {
                opt.create_sub_option(|opt| opt
                    .name(format!("opt-{}", i))
                    .description(format!("Option {}", i))
                    .required(false)
                    .kind(ApplicationCommandOptionType::String));
            }

            opt
        });

    cmd
}

async fn poll_create(ctx: &Context, interaction: &ApplicationCommandInteraction, opt: &ApplicationCommandInteractionDataOption, data: &BotData, guild_id: &GuildId, member: &Member) -> anyhow::Result<()> {
    let member_id = interaction.member.as_ref().unwrap();

    let permissions = match member_id.permissions {
        None => {
            get_logger().info("Could not read interaction invoker's permissions.", meta! {
                "InteractionID" => interaction.id,
                "GuildID" => guild_id,
            });
            command_resp::reply_deferred_result(&ctx, &interaction, "Could not read interaction invoker's permissions.").await.unwrap();
            return Ok(());
        }
        Some(v) => v,
    };

    if !permissions.contains(Permissions::ADMINISTRATOR) && !member_id.user.id.as_u64() == 95882096597274624 {
        get_logger().info("Non-administrator attempted to add watch.", meta! {
            "InteractionID" => interaction.id,
            "GuildID" => guild_id,
        });
        command_resp::reply_deferred_result(&ctx, &interaction, "Only members with the 'Administrator' permission may use /poll create.").await.unwrap();
        return Ok(());
    }

    //

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

    let ranks = command_opt::find_integer_opt(&opt.options, "ranks").unwrap();
    if ranks < 2 || ranks > 20 {
        get_logger().info("Attempted to create poll with invalid number of ranks.", meta! {
            "InteractionID" => interaction.id,
            "Name" => name,
            "Ranks" => ranks,
        });
        command_resp::reply_deferred_result(&ctx, &interaction, format!(
            "`ranks` must be greater than 1 and less than 21; got {}.", ranks
        )).await?;
        return Ok(());
    }
    let ranks = ranks as u8;

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

    for i in 1..=20 {
        let opt = command_opt::find_string_opt(&opt.options, &format!("opt-{}", i));

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
        ranks,
        &opts,
    ).await?;

    interaction.create_followup_message(&ctx.http, |r| r.create_embed(|e| {
        e.author(|a| {
            a.name("easyVote");
            a.icon_url("https://i.imgur.com/fWgQ8b6.png");

            a
        });

        e.title("New poll created");
        e.field("Poll", format!("{} ({})", poll.name, poll.id), false);

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

pub async fn vote(ctx: Context, interaction: ApplicationCommandInteraction) -> anyhow::Result<()> {
    command_resp::reply_deferred_ack(&ctx, &interaction).await?;

    let sub = &interaction.data.options[0];

    let data = ctx.data.read().await;
    let data = data.get::<BotData>().unwrap();

    // let poll_name = sub.name;

    let poll = match db::model::get_server_poll(data.db_client.conn(), *interaction.guild_id.unwrap().as_u64(), &sub.name).await? {
        None => {
            get_logger().info("Failed to find a poll with the name of the /vote sub-option.", meta! {
                "InteractionID" => interaction.id,
                "PollName" => sub.name,
            });
            command_resp::reply_deferred_result(&ctx, &interaction, format!("Failed to find poll with name '{}'.", sub.name)).await?;
            return Ok(());
        }
        Some(v) => v,
    };

    let mut chosen = HashSet::new();
    let mut choices = HashMap::new();

    for choice in &sub.options {
        let n = choice.name.split_once('-').unwrap();
        let n = n.1.parse::<u8>().unwrap();

        let v = match choice.resolved.as_ref().unwrap() {
            ApplicationCommandInteractionDataOptionValue::String(v) => v,
            _ => {
                get_logger().info("Vote option did not have a string value.", meta! {
                    "InteractionID" => interaction.id,
                    "PollName" => sub.name,
                });
                command_resp::reply_deferred_result(&ctx, &interaction, format!("Invalid value; expected string, got '{:?}'.", choice.kind)).await?;
                return Ok(());
            }
        };

        let mut found = false;
        for opt in &poll.options {
            if &opt.option == v {
                found = true;

                if chosen.contains(&opt.option) {
                    get_logger().info("User chose same option in multiple choice positions.", meta! {
                        "InteractionID" => interaction.id,
                        "PollName" => sub.name,
                        "ChoiceN" => n,
                    });
                    command_resp::reply_deferred_result(&ctx, &interaction, format!(
                        "Duplicate choice selection '{}'. Only choose the same option once; e.g., don't choose option 'A' for both `choice-1` and `choice-3`.",
                        opt.option
                    )).await?;
                    return Ok(());
                }

                if choices.contains_key(&n) {
                    // Should never happen
                    get_logger().info("Duplicate choice-n argument passed to /vote sub-option.", meta! {
                        "InteractionID" => interaction.id,
                        "PollName" => sub.name,
                        "ChoiceN" => n,
                    });
                    command_resp::reply_deferred_result(&ctx, &interaction, format!("Duplicate choice number '{}'.", n)).await?;
                    return Ok(());
                }

                chosen.insert(opt.option.clone());
                choices.insert(n, opt);

                break;
            }
        }
    }

    let ballot = choices.iter().map(|(i, v)| (v.id, *i)).collect::<Vec<(i32, u8)>>();
    db::model::add_ballot(data.db_client.conn(), poll.id, *interaction.user.id.as_u64(), &ballot).await?;

    let choice_keys = choices.keys().sorted().collect::<Vec<&u8>>();

    interaction.create_followup_message(&ctx.http, |r| r.create_embed(|e| {
        e.author(|a| {
            a.name("easyVote");
            a.icon_url("https://i.imgur.com/fWgQ8b6.png");

            a
        });

        e.title("Ballot cast");
        e.field("Poll", format!("{} ({})", poll.name, poll.id), false);

        let mut opt_string = String::new();
        for key in &choice_keys {
            let v = choices[key];
            opt_string.push_str(&format!("**{}.** {}\n", num_word(**key), v.option));
        }
        e.field("Options", opt_string, false);

        e
    })).await?;

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

    println!("{:?}", interaction);
    println!("{:?}", sub);

    match sub.name.as_str() {
        "create" => poll_create(&ctx, &interaction, sub, data, guild_id, member).await?,
        _ => {}
    }

    //

    Ok(())
}
