pub mod models;
pub mod repositories;

use std::time::Duration;

use anyhow::Context;
use sqlx::{PgPool, Postgres, migrate::MigrateDatabase, postgres::PgPoolOptions};

/// Create a PostgreSQL connection pool.
pub async fn create_pool(database_url: &str, max_connections: u32) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .context("failed to connect to database")?;

    Ok(pool)
}

/// Ensure the database exists and run migrations in this crate's `migrations/` folder.
///
/// When `create_if_missing` is true, the database will be created automatically if it
/// does not exist. Set this to false in production to fail loudly on misconfiguration
/// instead of silently creating an empty database.
pub async fn ensure_db_and_migrate(
    database_url: &str,
    pool: &PgPool,
    create_if_missing: bool,
) -> anyhow::Result<()> {
    if create_if_missing {
        let exists = Postgres::database_exists(database_url).await?;
        if !exists {
            Postgres::create_database(database_url).await?;
        }
    }

    // Run migrations bundled at compile time from `migrations/`
    sqlx::migrate!().run(pool).await?;

    Ok(())
}
