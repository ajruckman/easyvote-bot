use std::collections::{HashMap, HashSet};

use evlog::meta;
use itertools::Itertools;
use once_cell::sync::{Lazy, OnceCell};
use regex::Regex;
use serenity::builder::CreateApplicationCommand;
use serenity::client::Context;
use serenity::model::guild::Member;
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::{ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType};
use serenity::model::Permissions;
use serenity::model::prelude::application_command::ApplicationCommandInteractionDataOption;

use crate::{db, stv};
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
        })
        .create_option(|opt| {
            opt
                .name("close")
                .description("Close a poll")
                .kind(ApplicationCommandOptionType::SubCommand)

                .create_sub_option(|opt| opt
                    .name("name")
                    .description("The name of the poll to close")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String))
        })
        .create_option(|opt| {
            opt
                .name("tally")
                .description("Compute poll results")
                .kind(ApplicationCommandOptionType::SubCommand)

                .create_sub_option(|opt| opt
                    .name("name")
                    .description("The name of the poll to tally")
                    .required(true)
                    .kind(ApplicationCommandOptionType::String))

                .create_sub_option(|opt| opt
                    .name("seats")
                    .description("The number of seats to calculate results for")
                    .required(true)
                    .kind(ApplicationCommandOptionType::Integer))
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

    let server_has_poll_name = match db::model::check_server_has_poll_name(data.db_client.conn(), *guild_id.as_u64(), &name).await {
        Ok(v) => v,
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to check for existing poll by name.").await?;
            return Err(e);
        }
    };

    if server_has_poll_name {
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

    let poll = match db::model::add_poll(
        data.db_client.conn(),
        *guild_id.as_u64(),
        *member.user.id.as_u64(),
        &name,
        &question,
        ranks,
        &opts,
    ).await {
        Ok(v) => v,
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to add poll to database.").await?;
            return Err(e);
        }
    };

    interaction.create_followup_message(&ctx.http, |r| r.create_embed(|e| {
        e.title("New poll created");
        e.thumbnail("https://i.imgur.com/fWgQ8b6.png");

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

async fn poll_close(ctx: &Context, interaction: &ApplicationCommandInteraction, opt: &ApplicationCommandInteractionDataOption, data: &BotData, guild_id: &GuildId, member: &Member) -> anyhow::Result<()> {
    let name = command_opt::find_required(&ctx, &interaction, &opt.options, command_opt::find_string_opt, "name").await?.unwrap();

    let closed = match db::model::close_poll(data.db_client.conn(), *guild_id.as_u64(), *member.user.id.as_u64(), &name).await {
        Ok(v) => v,
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to look up and close poll.").await?;
            return Err(e);
        }
    };

    if closed {
        command_resp::reply_deferred_result(&ctx, &interaction, format!("Voting closed for poll **'{}'**.", name)).await?;
    } else {
        command_resp::reply_deferred_result(&ctx, &interaction, format!("No open poll named **'{}'** was found.", name)).await?;
    }

    Ok(())
}

async fn poll_tally(ctx: &Context, interaction: &ApplicationCommandInteraction, opt: &ApplicationCommandInteractionDataOption, data: &BotData, guild_id: &GuildId) -> anyhow::Result<()> {
    let name = command_opt::find_required(&ctx, &interaction, &opt.options, command_opt::find_string_opt, "name").await?.unwrap();
    let seats = command_opt::find_required(&ctx, &interaction, &opt.options, command_opt::find_integer_opt, "seats").await?.unwrap();

    if seats < 1 {
        command_resp::reply_deferred_result(&ctx, &interaction, "Value for 'seats' must be at least 1.").await?;
        return Ok(());
    }
    let seats = seats as u64;

    let poll = match db::model::get_server_poll(data.db_client.conn(), *guild_id.as_u64(), &name).await {
        Ok(v) => match v {
            None => {
                command_resp::reply_deferred_result(&ctx, &interaction, format!("Failed to find poll with name **'{}'**.", name)).await?;
                return Ok(());
            }
            Some(v) => v,
        }
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to look up poll.").await?;
            return Err(e);
        }
    };

    let ballots = match db::model::get_valid_ballots(data.db_client.conn(), poll.id).await {
        Ok(v) => v,
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to look up valid ballots for poll.").await?;
            return Err(e);
        }
    };

    //

    let mut stv_candidates = Vec::new();
    for opt in &poll.options {
        stv_candidates.push(opt.option.clone());
    }

    let mut stv_votes = Vec::new();
    for ballot in &ballots {
        let mut stv_vote = Vec::new();

        for choice in ballot.choices.iter().sorted_by_key(|v| v.rank).map(|v| v.id_option) {
            for opt in &poll.options {
                if choice == opt.id {
                    stv_vote.push(opt.option.clone());
                    break;
                }
            }
        }

        stv_votes.push(stv_vote);
    }

    let stv_election = stv::Election::new(stv_candidates, stv_votes, seats);

    let stv_results = match stv_election.results() {
        Ok(v) => v,
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to tally ballots.").await?;
            return Err(e);
        }
    };

    let mut winners = Vec::new();
    for (opt, votes) in stv_results.elected() {
        winners.push((opt.as_str().to_owned(), *votes));
    }
    winners.sort_by_key(|(_, votes)| -(*votes as i64));

    //

    interaction.create_followup_message(&ctx.http, |r| r.create_embed(|e| {
        e.title("Poll results");
        e.thumbnail("https://i.imgur.com/fWgQ8b6.png");

        e.field("Poll", format!("{} ({})", poll.name, poll.id), false);

        let mut res_string = String::new();

        let mut last = u64::MAX;
        let mut curr = 0;
        for (opt, votes) in winners {
            if votes < last {
                curr += 1;
                last = votes;
            }
            res_string.push_str(&format!("**{}**. **{}** (cumulative votes: {})\n", curr, opt, votes));
        }

        e.field("Winners", res_string, false);

        e
    })).await?;

    Ok(())
}

pub async fn vote(ctx: Context, interaction: ApplicationCommandInteraction) -> anyhow::Result<()> {
    command_resp::reply_deferred_ack(&ctx, &interaction).await?;

    let sub = &interaction.data.options[0];
    let id_user = *interaction.user.id.as_u64();
    // let id_user = Utc::now().time().num_seconds_from_midnight() as u64;

    let data = ctx.data.read().await;
    let data = data.get::<BotData>().unwrap();

    //

    let poll = match db::model::get_server_poll(data.db_client.conn(), *interaction.guild_id.unwrap().as_u64(), &sub.name).await {
        Ok(v) => match v {
            None => {
                get_logger().info("Failed to find a poll with the name of the /vote sub-option.", meta! {
                    "InteractionID" => interaction.id,
                    "PollName" => sub.name,
                });
                command_resp::reply_deferred_result(&ctx, &interaction, format!("Failed to find poll with name **'{}'**.", sub.name)).await?;
                return Ok(());
            }
            Some(v) => v,
        }
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to find poll by name.").await?;
            return Err(e);
        }
    };

    if !poll.open {
        get_logger().info("User attempted to vote on closed poll.", meta! {
            "InteractionID" => interaction.id,
            "PollID" => poll.id,
            "PollName" => poll.name,
        });
        command_resp::reply_deferred_result(&ctx, &interaction, format!("Voting is closed for poll **'{}'**.", sub.name)).await?;
        return Ok(());
    }

    //

    if !ALLOWED_VOTER_IDS.contains(&id_user) {
        get_logger().info("Ineligible user attempted to vote.", meta! {
            "InteractionID" => interaction.id,
            "PollID" => poll.id,
            "PollName" => poll.name,
            "UserID" => id_user,
        });
        command_resp::reply_deferred_result(&ctx, &interaction, format!(
            "You are not eligible to vote in elections. \
            Members may only vote if they sent at least 250 messages between \
            October 31, 2021 at 11:30 AM EST, and January 23, 2022 at 11:30 AM EST."
        )).await?;
        return Ok(());
    }

    //

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
                    "PollID" => poll.id,
                    "PollName" => poll.name,
                });
                command_resp::reply_deferred_result(&ctx, &interaction, format!("Invalid value; expected string, got '{:?}'.", choice.kind)).await?;
                return Ok(());
            }
        };

        for opt in &poll.options {
            if &opt.option == v {
                if chosen.contains(&opt.option) {
                    get_logger().info("User chose same option in multiple choice positions.", meta! {
                        "InteractionID" => interaction.id,
                        "PollID" => poll.id,
                        "PollName" => poll.name,
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
                        "PollID" => poll.id,
                        "PollName" => poll.name,
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

    //

    let existed = match db::model::get_valid_ballot(data.db_client.conn(), poll.id, id_user).await {
        Ok(v) => v,
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to find user's existing ballot for poll.").await?;
            return Err(e);
        }
    };
    match &existed {
        None => {}
        Some(v) => match db::model::invalidate_ballot(data.db_client.conn(), v.id).await {
            Ok(()) => {}
            Err(e) => {
                command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to invalidate user's existing ballot for poll.").await?;
                return Err(e);
            }
        }
    }

    //

    let ballot = choices.iter().map(|(i, v)| (v.id, *i)).collect::<Vec<(i32, u8)>>();
    match db::model::add_ballot(data.db_client.conn(), poll.id, id_user, &ballot).await {
        Ok(()) => {}
        Err(e) => {
            command_resp::reply_deferred_result(&ctx, &interaction, "Error occurred upon attempt to add ballot.").await?;
            return Err(e);
        }
    }

    let choice_keys = choices.keys().sorted().collect::<Vec<&u8>>();

    interaction.create_followup_message(&ctx.http, |r| r.create_embed(|e| {
        e.title("Ballot cast");
        e.thumbnail("https://i.imgur.com/fWgQ8b6.png");

        e.field("Poll", format!("{} ({})", poll.name, poll.id), false);

        match &existed {
            None => {}
            Some(v) => {
                e.field("Replaced ballot ID", format!("{} (from {})", v.id, v.time_created), false);
            }
        }

        let mut opt_string = String::new();
        for key in &choice_keys {
            let v = choices[key];
            opt_string.push_str(&format!("**{}.** {}\n", num_word(**key), v.option));
        }
        e.field("Choices", opt_string, false);

        e
    })).await?;

    Ok(())
}

pub async fn poll(ctx: Context, interaction: ApplicationCommandInteraction) -> anyhow::Result<()> {
    command_resp::reply_deferred_ack(&ctx, &interaction).await?;

    let guild_id = interaction.guild_id.as_ref().unwrap();
    let member = interaction.member.as_ref().unwrap();

    //

    let sub = &interaction.data.options[0];

    println!("{:?}", interaction);
    println!("{:?}", sub);

    let data = ctx.data.read().await;
    let data = data.get::<BotData>().unwrap();

    match sub.name.as_str() {
        "create" => poll_create(&ctx, &interaction, sub, data, guild_id, member).await?,
        "close" => poll_close(&ctx, &interaction, sub, data, guild_id, member).await?,
        "tally" => poll_tally(&ctx, &interaction, sub, data, guild_id).await?,
        _ => {}
    }

    //

    Ok(())
}

static ALLOWED_VOTER_IDS: Lazy<HashSet<u64>> = Lazy::new(|| {
    let mut allowed = HashSet::new();

    allowed.insert(350021640223981569);
    allowed.insert(468969078879551489);
    allowed.insert(565813691891712004);
    allowed.insert(241280648436908032);
    allowed.insert(822597873321508924);
    allowed.insert(705993505784201227);
    allowed.insert(191609253683920896);
    allowed.insert(313414684613279748);
    allowed.insert(95882096597274624);
    allowed.insert(712171518347444225);
    allowed.insert(384172870344900609);
    allowed.insert(403179220445954052);
    allowed.insert(478676609914765322);
    allowed.insert(925092899719352382);
    allowed.insert(693660661300985908);
    allowed.insert(276910354091474944);
    allowed.insert(633770126407237643);
    allowed.insert(166316524188073984);
    allowed.insert(292953664492929025);
    allowed.insert(312052072105115648);
    allowed.insert(881847125397372938);
    allowed.insert(428973268771536907);
    allowed.insert(496465935574499328);
    allowed.insert(127246301518757888);
    allowed.insert(280620013046464513);
    allowed.insert(889947779336638464);
    allowed.insert(655541486422589443);
    allowed.insert(181679004669968384);
    allowed.insert(234030425490718720);
    allowed.insert(293325579380326400);
    allowed.insert(155149108183695360);
    allowed.insert(226470678331785217);
    allowed.insert(846513264518234122);
    allowed.insert(304435494211682305);
    allowed.insert(831177153819181116);
    allowed.insert(838115755434311690);
    allowed.insert(837041495785209976);
    allowed.insert(526062632688680970);
    allowed.insert(881226255138173029);
    allowed.insert(272133320614084609);
    allowed.insert(808344274109857832);
    allowed.insert(215636790172712961);
    allowed.insert(711375037265084497);
    allowed.insert(689024279601479696);
    allowed.insert(537353774205894676);
    allowed.insert(305325955512008704);
    allowed.insert(880812757799305307);
    allowed.insert(442732803474456576);
    allowed.insert(464468304896065536);
    allowed.insert(479371794093047808);
    allowed.insert(614109280508968980);
    allowed.insert(917103954251354112);
    allowed.insert(926200130212876328);
    allowed.insert(589928914399526918);
    allowed.insert(186125711537340417);
    allowed.insert(618196763588952065);
    allowed.insert(482457186375827457);
    allowed.insert(502561547458183189);
    allowed.insert(377877206526984203);
    allowed.insert(520904266555195393);
    allowed.insert(129794058917904385);
    allowed.insert(643228492598607872);
    allowed.insert(541318786259746816);
    allowed.insert(910227052723052587);
    allowed.insert(561318847785730062);
    allowed.insert(397546088908849166);
    allowed.insert(700796664276844612);
    allowed.insert(918002895570489394);
    allowed.insert(182232330088218624);
    allowed.insert(383680177932075010);

    allowed
});
