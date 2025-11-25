# Migration Rollback Scripts

This directory contains down/rollback scripts for all database migrations.

## ⚠️ WARNING

Rolling back migrations will **lose data**! Always backup your database first:

```bash
pg_dump -U your_user your_database > backup_$(date +%Y%m%d_%H%M%S).sql
```

## How to Rollback a Migration

```bash
# Rollback migration 0006
psql -U your_user -d your_database -f rollback/0006_performance_and_cleanup_improvements_down.sql

# Rollback migration 0005
psql -U your_user -d your_database -f rollback/0005_refresh_tokens_down.sql
```

## Rollback Order

Always rollback in reverse order:

1. `0006_performance_and_cleanup_improvements_down.sql`
2. `0005_refresh_tokens_down.sql`
3. `0004_email_verification_down.sql`
4. `0003_password_reset_tokens_down.sql`
5. `0002_add_profile_picture_down.sql`
6. `0001_init_down.sql` (⚠️ **DESTROYS ALL DATA!**)

## After Rollback

You'll need to manually update the `_sqlx_migrations` table:

```sql
-- Check current migrations
SELECT * FROM _sqlx_migrations ORDER BY version;

-- Delete the rolled-back migration record
DELETE FROM _sqlx_migrations WHERE version = 6;  -- Replace with your version
```

## Why Not in migrations/ Directory?

These files are kept separate from the `migrations/` directory because:
- sqlx only looks for forward migrations
- Prevents accidental application as forward migrations
- Keeps the migration directory clean
- Follows common Rust migration patterns

## See Also

- [DATABASE_MAINTENANCE.md](../DATABASE_MAINTENANCE.md) - Full maintenance guide
- [QUICK_REFERENCE.md](../QUICK_REFERENCE.md) - Quick command reference
