use chrono::{DateTime, Utc};

pub struct Poll {
    pub id: i32,
    pub time_created: DateTime<Utc>,
    pub id_server: u64,
    pub id_created_by: u64,
    pub active: bool,
    pub name: String,
    pub question: String,
    pub ranks: u8,
    pub options: Vec<PollOption>,
}

pub struct PollOption {
    pub id_poll: i32,
    pub id: i32,
    pub option: String,
}

pub struct Ballot {
    pub id: i32,
    pub id_poll: i32,
    pub id_user: u64,
    pub time_created: DateTime<Utc>,
    pub invalidated: bool,
    pub choices: Vec<BallotChoice>,
}

pub struct BallotChoice {
    pub id_ballot: i32,
    pub id_option: i32,
    pub rank: u8,
}
