use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub struct DBClient {
    conn_pool: PgPool,
}

impl DBClient {
    pub async fn new(db_url: &str) -> anyhow::Result<DBClient> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await?;

        Ok(DBClient {
            conn_pool: pool,
        })
    }

    #[must_use]
    pub fn conn(&self) -> &PgPool {
        &self.conn_pool
    }
}
