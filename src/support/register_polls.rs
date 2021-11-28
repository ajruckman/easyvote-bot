use serenity::client::Context;
use serenity::model::guild::Guild;
use serenity::model::interactions::application_command::{ApplicationCommand, ApplicationCommandOptionType};
use serenity::model::prelude::application_command::ApplicationCommandType;
use sqlx::PgPool;

pub async fn register_polls(conn: &PgPool, ctx: &Context, guild: &Guild) -> anyhow::Result<()> {
    let existing_cmds = guild.get_application_commands(&ctx).await?;

    let vote_cmd = existing_cmds.iter().find(|v| v.name == "vote");
    match vote_cmd {
        None => {}
        Some(v) => {
            guild.delete_application_command(&ctx, v.id).await?;
            println!("Deleted {}", v.id);
        }
    }

    let polls = crate::db::model::list_active_polls(conn, *guild.id.as_u64()).await?;

    // match vote_cmd {
    //     None => {
    guild.create_application_command(&ctx, |c| {
        c
            .name("vote")
            .description("Vote on a poll");

        for poll in polls {
            println!("{}", poll.id);
            c.create_option(|opt| {
                opt.name(poll.name)
                    .description("The poll to vote on")
                    .kind(ApplicationCommandOptionType::SubCommand);

                for i in 0..poll.options.len() {
                    opt.create_sub_option(|opt_sub| {
                        opt_sub.name(format!("pick-{}", i + 1))
                            .description(format!("Your choice for {}", i + 1))
                            .required(true)
                            .kind(ApplicationCommandOptionType::String);

                        for poll_opt in &poll.options {
                            opt_sub.add_string_choice(&poll_opt.option, &poll_opt.option);
                        }

                        opt_sub
                    });
                }

                opt
            });
        }

        // c.create_option(|o| {
        //     o.name("poll")
        //         .description("The poll to vote on")
        //         .kind(ApplicationCommandOptionType::SubCommand);
        //
        //     for poll in polls {
        //         o.create_sub_option(|c_sub| {
        //             c_sub
        //                 .name(poll.name)
        //                 .description(poll.question)
        //                 .required(true)
        //                 .kind(ApplicationCommandOptionType::SubCommand);
        //
        //
        //
        //             c_sub
        //         });
        //     }
        //
        //     o
        // });

        c
    }).await?;
    // }
    // Some(v) => {}
    // }

    Ok(())
}
