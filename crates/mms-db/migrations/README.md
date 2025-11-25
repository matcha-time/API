# Database Maintenance Guide

This guide covers regular maintenance tasks for the Matcha Time database.

## Table of Contents

- [Automatic Cleanup](#automatic-cleanup)
- [Manual Cleanup](#manual-cleanup)
- [Scheduled Jobs](#scheduled-jobs)
- [Performance Monitoring](#performance-monitoring)
- [Migration Management](#migration-management)

---

## Automatic Cleanup

### Token Cleanup Triggers

As of migration `0006`, the database automatically cleans up expired tokens whenever new tokens are created. This provides **opportunistic cleanup** without requiring scheduled jobs.

**What happens automatically:**

- When a new password reset token is created → expired/used password reset tokens are deleted
- When a new email verification token is created → expired/used email verification tokens are deleted
- When a new refresh token is created → expired refresh tokens are deleted

**Why this works:**

- Token creation is already an async operation (sending emails, etc.)
- The cleanup query uses indexed `expires_at` columns for fast execution
- Prevents unbounded table growth without needing cron jobs

---

## Manual Cleanup

### Clean Up All Expired Tokens

Run the master cleanup function to remove all expired tokens and get statistics:

```sql
SELECT * FROM cleanup_all_expired_tokens();
```

**Returns:**

```aasci
 password_reset_cleaned | email_verification_cleaned | refresh_tokens_cleaned | total_cleaned
------------------------+----------------------------+------------------------+---------------
                     42 |                        137 |                     89 |           268
```

### Clean Up Individual Token Types

```sql
-- Password reset tokens only
SELECT cleanup_expired_password_reset_tokens();

-- Email verification tokens only
SELECT cleanup_expired_email_verification_tokens();

-- Refresh tokens only
SELECT cleanup_expired_refresh_tokens();
```

### Refresh Stale Deck Progress

If a user's `cards_due_today` count seems stale (not updated recently):

```sql
-- Refresh all stale deck progress for a user
SELECT refresh_stale_deck_progress('user-uuid-here');

-- Or refresh a specific deck
SELECT refresh_deck_progress('user-uuid-here', 'deck-uuid-here');
```

**Recommended:** Call `refresh_stale_deck_progress(user_id)` when loading the user's dashboard to ensure accurate stats.

---

## Scheduled Jobs

### Option 1: PostgreSQL pg_cron Extension (Recommended)

Install pg_cron:

```sql
CREATE EXTENSION pg_cron;
```

Schedule daily cleanup at 2 AM:

```sql
-- Clean up expired tokens daily
SELECT cron.schedule(
    'cleanup-expired-tokens',
    '0 2 * * *',  -- Every day at 2 AM
    'SELECT cleanup_all_expired_tokens();'
);

-- View scheduled jobs
SELECT * FROM cron.job;

-- View job run history
SELECT * FROM cron.job_run_details ORDER BY start_time DESC LIMIT 10;
```

### Option 2: Application-Level Scheduler

If pg_cron is not available, implement in your Rust application using `tokio-cron-scheduler`:

```rust
use tokio_cron_scheduler::{JobScheduler, Job};

pub async fn setup_maintenance_jobs(pool: &PgPool) -> Result<JobScheduler, Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;

    // Clone pool for the job closure
    let pool_clone = pool.clone();

    // Schedule cleanup job daily at 2 AM
    sched.add(Job::new_async("0 0 2 * * *", move |_uuid, _lock| {
        let pool = pool_clone.clone();
        Box::pin(async move {
            match sqlx::query("SELECT cleanup_all_expired_tokens()")
                .execute(&pool)
                .await
            {
                Ok(_) => tracing::info!("Token cleanup completed successfully"),
                Err(e) => tracing::error!("Token cleanup failed: {}", e),
            }
        })
    })?)?;

    sched.start().await?;
    Ok(sched)
}
```

### Option 3: External Cron Job

Add to your server's crontab:

```bash
# Run cleanup daily at 2 AM
0 2 * * * psql -U your_user -d your_database -c "SELECT cleanup_all_expired_tokens();"
```

---

## Performance Monitoring

### Check Index Usage

```sql
-- View index usage statistics
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan as scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;
```

### Check Table Sizes

```sql
-- View table sizes
SELECT
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### Monitor Token Table Growth

```sql
-- Count tokens by type and status
SELECT
    'password_reset' as token_type,
    COUNT(*) FILTER (WHERE expires_at < NOW()) as expired,
    COUNT(*) FILTER (WHERE used_at IS NOT NULL) as used,
    COUNT(*) FILTER (WHERE expires_at >= NOW() AND used_at IS NULL) as active
FROM password_reset_tokens
UNION ALL
SELECT
    'email_verification' as token_type,
    COUNT(*) FILTER (WHERE expires_at < NOW()) as expired,
    COUNT(*) FILTER (WHERE used_at IS NOT NULL) as used,
    COUNT(*) FILTER (WHERE expires_at >= NOW() AND used_at IS NULL) as active
FROM email_verification_tokens
UNION ALL
SELECT
    'refresh_tokens' as token_type,
    COUNT(*) FILTER (WHERE expires_at < NOW()) as expired,
    NULL as used,
    COUNT(*) FILTER (WHERE expires_at >= NOW()) as active
FROM refresh_tokens;
```

**Expected results:**

- Active tokens: varies with user activity
- Expired/used tokens: should be 0 or very low (due to automatic cleanup)

If you see high numbers of expired tokens, run manual cleanup and investigate why automatic cleanup isn't working.

---

## Migration Management

### Running Migrations

Apply the latest migration:

```bash
# Apply all pending migrations
sqlx migrate run

# Apply specific migration
psql -U your_user -d your_database -f crates/mms-db/migrations/0006_performance_and_cleanup_improvements.sql
```

### Rolling Back Migrations

Each migration has a corresponding rollback script in the `../rollback/` directory:

```bash
# Rollback last migration (0006)
psql -U your_user -d your_database -f crates/mms-db/rollback/0006_performance_and_cleanup_improvements_down.sql

# Rollback refresh tokens (0005)
psql -U your_user -d your_database -f crates/mms-db/rollback/0005_refresh_tokens_down.sql
```

**WARNING:** Rolling back migrations will lose data! Always backup first:

```bash
pg_dump -U your_user your_database > backup_$(date +%Y%m%d_%H%M%S).sql
```

### Migration Order

If you need to rollback multiple migrations, do it in reverse order:

1. `0006_performance_and_cleanup_improvements_down.sql`
2. `0005_refresh_tokens_down.sql`
3. `0004_email_verification_down.sql`
4. `0003_password_reset_tokens_down.sql`
5. `0002_add_profile_picture_down.sql`
6. `0001_init_down.sql` (⚠️ **DESTROYS ALL DATA!**)

---

## Troubleshooting

### Problem: Token tables growing too large

**Diagnosis:**

```sql
SELECT COUNT(*) FROM password_reset_tokens WHERE expires_at < NOW();
SELECT COUNT(*) FROM email_verification_tokens WHERE expires_at < NOW();
SELECT COUNT(*) FROM refresh_tokens WHERE expires_at < NOW();
```

**Solution:**

```sql
-- Manual cleanup
SELECT cleanup_all_expired_tokens();

-- Check triggers are enabled
SELECT tgname, tgenabled
FROM pg_trigger
WHERE tgname LIKE '%cleanup%';
```

### Problem: Slow login performance

**Diagnosis:**

```sql
EXPLAIN ANALYZE
SELECT * FROM users WHERE email = 'test@example.com';
```

Should show `Index Scan using idx_users_email`. If it shows `Seq Scan`, the index is missing.

**Solution:**

```sql
-- Ensure index exists
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Force analyze to update statistics
ANALYZE users;
```

### Problem: Inaccurate cards_due_today

**Diagnosis:**

```sql
-- Check when deck progress was last updated
SELECT deck_id, cards_due_today, updated_at
FROM user_deck_progress
WHERE user_id = 'user-uuid-here'
ORDER BY updated_at DESC;
```

**Solution:**

```sql
-- Refresh all stale progress
SELECT refresh_stale_deck_progress('user-uuid-here');
```

**Prevention:** Call `refresh_stale_deck_progress(user_id)` when loading the dashboard.

---

## Performance Benchmarks

After applying migration 0006, you should see:

| Query | Before | After |
|-------|--------|-------|
| Login by email | Full table scan | ~1-2ms (indexed) |
| Token lookup | ~10-50ms | ~1-2ms (indexed) |
| Token cleanup | Full table scan | ~5-10ms (indexed) |
| Practice session | ~2-5ms | ~2ms (already optimized) |

Run `EXPLAIN ANALYZE` on critical queries to verify performance improvements.
