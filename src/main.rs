#![feature(trait_alias)]

use std::env;
use evlog::{LogEventConsolePrinter, Logger};
use serenity::Client;
use crate::db::dbclient::DBClient;
use crate::handler::{BotData, BotHandler};
use crate::runtime::{get_logger, set_logger};

mod handler;
mod runtime;
mod db;
mod commands;
mod helpers;
mod support;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let appl: u64 = env::var("EASYVOTE_APPL").expect("expected EASYVOTE_APPL").parse().expect("application ID is invalid");
    let token = env::var("EASYVOTE_TOKEN").expect("expected EASYVOTE_TOKEN");
    let db_url = env::var("DATABASE_URL").expect("expected DATABASE_URL");

    let mut logger = Logger::default();
    logger.register(LogEventConsolePrinter::default());
    set_logger(logger);

    let db_client = DBClient::new(&db_url).await
        .expect("failed to connect to database");

    let data = handler::BotData::new(db_client).await;

    let mut client = Client::builder(&token)
        .event_handler(BotHandler {})
        .application_id(appl)
        .await
        .unwrap_or_else(|e| {
            get_logger().error_with_err("Client initialization error.", &e, None);
            panic!("{}", e)
        });
    client.data.write().await.insert::<BotData>(data);

    if let Err(e) = client.start_shards(2).await {
        get_logger().error_with_err("Client error.", e, None);
    }
}
