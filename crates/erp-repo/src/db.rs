use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn connect_lazy(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new().connect_lazy(database_url)?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Reuses a pool the caller already opened. Concurrent writes then share
    /// one connection limit and can wait on the same row lock.
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }
}
