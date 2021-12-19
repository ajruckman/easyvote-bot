use sqlx::{PgPool, query};
use tokio_stream::StreamExt;

use crate::db::schema::{Ballot, BallotChoice, Poll, PollOption};

pub async fn list_open_polls(
    conn: &PgPool,
    id_server: u64,
) -> anyhow::Result<Vec<Poll>> {
    let mut stream = query!("SELECT * FROM poll WHERE id_server=$1 AND open=TRUE;", id_server.to_string())
        .map(|r| Poll {
            id: r.id,
            time_created: r.time_created,
            id_server: r.id_server.parse::<u64>().unwrap(),
            id_created_by: r.id_created_by.parse::<u64>().unwrap(),
            open: r.open,
            name: r.name,
            question: r.question,
            ranks: r.ranks as u8,
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

pub async fn get_server_poll(conn: &PgPool, id_server: u64, name: &str) -> anyhow::Result<Option<Poll>> {
    let r = query!("SELECT * FROM poll WHERE id_server=$1 AND name=$2", id_server.to_string(), name)
        .fetch_optional(conn)
        .await?;

    let r = match r {
        None => return Ok(None),
        Some(v) => v,
    };

    let mut options = query!("SELECT * FROM poll_option WHERE id_poll = $1", r.id)
        .map(|row| {
            PollOption {
                id_poll: row.id_poll,
                id: row.id,
                option: row.option,
            }
        })
        .fetch(conn);

    let mut opt_result = Vec::new();
    while let Some(row) = options.try_next().await? {
        opt_result.push(row);
    }

    Ok(Some(Poll {
        id: r.id,
        time_created: r.time_created,
        id_server: id_server,
        id_created_by: r.id_created_by.parse::<u64>().unwrap(),
        open: r.open,
        name: r.name,
        question: r.question,
        ranks: r.ranks as u8,
        options: opt_result,
    }))
}

pub async fn add_poll(
    conn: &PgPool,
    id_server: u64,
    id_created_by: u64,
    name: &str,
    question: &str,
    ranks: u8,
    options: &[String],
) -> anyhow::Result<Poll> {
    let mut tx = conn.begin().await?;

    let r = query!(
        "INSERT INTO poll (time_created, id_server, id_created_by, open, name, question, ranks)
         VALUES (NOW(), $1, $2, TRUE, $3, $4, $5)
         RETURNING id, time_created;",
        id_server.to_string(), id_created_by.to_string(), name, question, ranks as i32)
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
        open: true,
        name: name.to_owned(),
        question: question.to_owned(),
        ranks: ranks,
        options: opt_result,
    })
}

pub async fn close_poll(
    conn: &PgPool,
    id_server: u64,
    id_created_by: u64,
    name: &str,
) -> anyhow::Result<bool> {
    let r = query!(
        "UPDATE poll SET open=FALSE WHERE id_server=$1 AND id_created_by=$2 AND name=$3 AND open=TRUE;",
        id_server.to_string(), id_created_by.to_string(), name)
        .execute(conn)
        .await?;

    Ok(r.rows_affected() > 0)
}

pub async fn add_ballot(
    conn: &PgPool,
    id_poll: i32,
    id_user: u64,
    choices: &[(i32, u8)],
) -> anyhow::Result<()> {
    let tx = conn.begin().await?;

    let ballot = query!(
        "INSERT INTO ballot (id_poll, id_user, time_created, invalidated)
         VALUES ($1, $2, NOW(), FALSE)
         RETURNING id;",
        id_poll, id_user.to_string())
        .fetch_one(conn)
        .await?;

    for choice in choices {
        query!(
            "INSERT INTO ballot_choice (id_ballot, id_option, rank)
             VALUES ($1, $2, $3);",
            ballot.id, choice.0, choice.1 as i32)
            .execute(conn)
            .await?;
    }

    tx.commit().await?;

    Ok(())
}

pub async fn get_valid_ballot(
    conn: &PgPool,
    id_poll: i32,
    id_user: u64,
) -> anyhow::Result<Option<Ballot>> {
    let tx = conn.begin().await?;

    let ballot = query!("SELECT * FROM ballot WHERE id_poll=$1 AND id_user=$2 AND invalidated=FALSE;", id_poll, id_user.to_string())
        .fetch_optional(conn)
        .await?;

    let ballot = match ballot {
        None => return Ok(None),
        Some(v) => v,
    };

    let mut r = Ballot {
        id: ballot.id,
        id_poll: ballot.id_poll,
        id_user: ballot.id_user.parse::<u64>().unwrap(),
        time_created: ballot.time_created,
        invalidated: ballot.invalidated,
        choices: Vec::new(),
    };

    let mut choices = query!("SELECT * FROM ballot_choice WHERE id_ballot=$1;", ballot.id)
        .map(|row| BallotChoice {
            id_ballot: row.id_ballot,
            id_option: row.id_option,
            rank: row.rank as u8,
        })
        .fetch(conn);

    for choice in choices.try_next().await? {
        r.choices.push(choice);
    }

    tx.commit().await?;

    Ok(Some(r))
}

pub async fn invalidate_ballot(conn: &PgPool, id_ballot: i32) -> anyhow::Result<()> {
    query!("UPDATE ballot SET invalidated=TRUE WHERE id=$1;", id_ballot).execute(conn).await?;

    Ok(())
}
