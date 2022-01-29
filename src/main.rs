#![feature(trait_alias)]

use std::env;
use evlog::{LogEventConsolePrinter, Logger};
use itertools::Itertools;
use serenity::Client;
use tallystick::Quota;
use tallystick::stv::Tally;
use crate::db::dbclient::DBClient;
use crate::handler::{BotData, BotHandler};
use crate::runtime::{get_logger, set_logger};

mod handler;
mod runtime;
mod db;
mod commands;
mod helpers;
mod support;
mod stv;

async fn tally(db: &DBClient) {
    // let name = command_opt::find_required(&ctx, &interaction, &opt.options, command_opt::find_string_opt, "name").await?.unwrap();
    let seats = 3;

    let poll = match db::model::get_server_poll(db.conn(), 303056270150074368, "admin-2022").await {
        Ok(v) => match v {
            None => {
                return;
            }
            Some(v) => v,
        }
        Err(e) => {
            return;
        }
    };

    let ballots = match db::model::get_valid_ballots(db.conn(), poll.id).await {
        Ok(v) => v,
        Err(e) => {
            return;
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

        println!("{}", stv_vote.join("+"));

        stv_votes.push(stv_vote);
    }

    let mut stv_election = Tally::<String, usize>::new(seats, Quota::Droop);
    // let stv_election = stv::Election::new(stv_candidates, stv_votes, seats);

    for vote in &stv_votes {
        stv_election.add_ref(vote);
    }

    let stv_results = stv_election.winners();
    // let stv_results = match stv_election.results() {
    //     Ok(v) => v,
    //     Err(e) => {
    //         return;
    //     }
    // };

    let mut winners = Vec::new();
    for vote in stv_results.winners {
        winners.push((vote.candidate, vote.rank));
        // winners.push((opt.as_str().to_owned(), *votes));
    }
    winners.sort_by_key(|(_, votes)| *votes as i64);

    //

    let mut res_string = String::new();

    let mut last = usize::MAX;
    let mut curr = 0;
    for (opt, rank) in winners {
        if rank < last {
            curr += 1;
            last = rank;
        }
        res_string.push_str(&format!("**{}**. **{}**\n", rank, opt));
    }

    println!("{}", res_string);
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let appl: u64 = env::var("EASYVOTE_APPL").expect("expected EASYVOTE_APPL").parse().expect("application ID is invalid");
    let token = env::var("EASYVOTE_TOKEN").expect("expected EASYVOTE_TOKEN");
    let db_url = env::var("EASYVOTE_DATABASE_URL").expect("expected EASYVOTE_DATABASE_URL");

    let mut logger = Logger::default();
    logger.register(LogEventConsolePrinter::default());
    set_logger(logger);

    let db_client = DBClient::new(&db_url).await
        .expect("failed to connect to database");

    tally(&db_client).await;

    return;

    // let data = handler::BotData::new(db_client).await;
    //
    // let mut client = Client::builder(&token)
    //     .event_handler(BotHandler {})
    //     .application_id(appl)
    //     .await
    //     .unwrap_or_else(|e| {
    //         get_logger().error_with_err("Client initialization error.", &e, None);
    //         panic!("{}", e)
    //     });
    // client.data.write().await.insert::<BotData>(data);
    //
    // if let Err(e) = client.start_shards(2).await {
    //     get_logger().error_with_err("Client error.", e, None);
    // }
}
