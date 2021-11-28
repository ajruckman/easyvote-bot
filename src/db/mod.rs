use sqlx::{Executor, Postgres};

pub mod dbclient;
pub mod schema;
pub mod model;

pub trait PGExec<'a> = Executor<'a, Database=Postgres>;
