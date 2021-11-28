use sqlx::{PgPool, query};
use tokio_stream::StreamExt;

use crate::db::schema::{Poll, PollOption};

pub async fn list_active_polls(
    conn: &PgPool,
    id_server: u64,
) -> anyhow::Result<Vec<Poll>> {
    let mut stream = query!("SELECT * FROM poll WHERE id_server=$1", id_server.to_string())
        .map(|r| Poll {
            id: r.id,
            time_created: r.time_created,
            id_server: r.id_server.parse::<u64>().unwrap(),
            id_created_by: r.id_created_by.parse::<u64>().unwrap(),
            active: r.active,
            name: r.name,
            question: r.question,
            options: Vec::new(),
        })
        .fetch(conn);

    let mut result = Vec::new();
    while let Some(mut row) = stream.try_next().await? {
        let mut stream = query!("SELECT * FROM poll_option WHERE id_poll=$1", row.id)
            .map(|r| PollOption {
                id_poll: r.id_poll,
                id: r.id,
                option: r.option.to_owned(),
            })
            .fetch(conn);

        while let Some(opt) = stream.try_next().await? {
            row.options.push(opt);
        }

        result.push(row);
    }

    Ok(result)
}

pub async fn check_server_has_poll_name(conn: &PgPool, id_server: u64, name: &str) -> anyhow::Result<bool> {
    let r = query!("SELECT EXISTS(SELECT 1 FROM poll WHERE id_server=$1 AND name=$2) AS known", id_server.to_string(), name)
        .fetch_one(conn)
        .await?;

    Ok(r.known.unwrap())
}

pub async fn add_poll(
    conn: &PgPool,
    id_server: u64,
    id_created_by: u64,
    name: &str,
    question: &str,
    options: &[String],
) -> anyhow::Result<Poll> {
    let mut tx = conn.begin().await?;

    let r = query!(
        "INSERT INTO poll (time_created, id_server, id_created_by, active, name, question)
         VALUES (NOW(), $1, $2, TRUE, $3, $4)
         RETURNING id, time_created;",
        id_server.to_string(), id_created_by.to_string(), name, question)
        .fetch_one(&mut tx)
        .await?;

    let mut opt_result = Vec::new();

    for option in options {
        let option_r = query!(
            "INSERT INTO poll_option (id_poll, option)
             VALUES ($1, $2)
             RETURNING id;",
            r.id, option)
            .fetch_one(&mut tx)
            .await?;

        opt_result.push(PollOption {
            id_poll: r.id,
            id: option_r.id,
            option: option.to_string(),
        });
    }

    tx.commit().await?;

    Ok(Poll {
        id: r.id,
        time_created: r.time_created,
        id_server: id_server,
        id_created_by: id_created_by,
        active: true,
        name: name.to_owned(),
        question: question.to_owned(),
        options: opt_result,
    })
}

pub async fn add_ballot(
    conn: &PgPool,
    id_poll: i32,
    id_user: u64,
    options: &[(i32, u8)],
) -> anyhow::Result<()> {
    // let tx = conn.begin().await?;

    for option in options {
        query!(
        "INSERT INTO ballot (id_poll, id_user, time_created, id_option, rank)
         VALUES ($1, $2, NOW(), $3, $4);",
        id_poll, id_user.to_string(), option.0, option.1 as i32)
            .execute(conn)
            .await?;
    }

    // tx.commit().await?;

    Ok(())
}
