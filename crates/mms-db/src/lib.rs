pub mod models;

use anyhow::Context;
use sqlx::{PgPool, Postgres, migrate::MigrateDatabase, postgres::PgPoolOptions};

/// Create a PostgreSQL connection pool.
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .context("failed to connect to database")?;

    Ok(pool)
}

/// Ensure the database exists and run migrations in this crate's `migrations/` folder.
pub async fn ensure_db_and_migrate(database_url: &str, pool: &PgPool) -> anyhow::Result<()> {
    // Ensure database exists (no-op if it already does)
    let exists = Postgres::database_exists(database_url).await?;
    if !exists {
        Postgres::create_database(database_url).await?;
    }

    // Run migrations bundled at compile time from `migrations/`
    sqlx::migrate!().run(pool).await?;

    Ok(())
}
