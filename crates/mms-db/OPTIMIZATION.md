# Database Optimization Guide

## Performance Optimization Strategies

This database schema is optimized for **blazing fast insertions and reads** even as data grows to millions of records.

### 1. Strategic Indexing

#### Write-Optimized Indexes
- **Minimal indexes on write-heavy tables**: `user_card_practice` has carefully chosen indexes to balance read/write performance
- **Partial indexes**: Used for time-based queries (e.g., `idx_ucp_recent_validations`) to reduce index size
- **Unique constraints**: Automatically create indexes (e.g., `(user_id, deck_id)` on `user_deck_practice`)

#### Read-Optimized Indexes
- **Foreign key indexes**: All foreign keys are indexed for fast JOINs
- **Composite indexes**: Cover common query patterns:
  - `(user_id, card_id, validated_at DESC)` - Get user's validation history
  - `(topic_id, created_at DESC)` - Get decks/cards by topic ordered by creation
  - `(user_id, last_practiced_at DESC)` - Get user's decks by recent practice

### 2. Data Type Optimization

- **BIGSERIAL/BIGINT**: For IDs (supports up to 9.2 quintillion records)
- **VARCHAR(255)**: For names (optimal for indexing, avoids TEXT overhead)
- **CHAR(2)**: For language codes (fixed-size, faster comparisons)
- **TEXT**: For descriptions/examples (only when length varies significantly)
- **DOUBLE PRECISION**: For scores (decimal accuracy with good performance)
- **TIMESTAMPTZ**: Timezone-aware timestamps (indexed for time-based queries)
- **BOOLEAN**: For validated flag (minimal storage, fast queries)

### 3. Denormalization Strategy

- **`topic_id` on `cards`**: Denormalized for direct topic access without JOIN
  - Trade-off: Slightly more storage, but eliminates JOIN for common queries
  - Updated via application logic or triggers

### 4. High-Volume Table Optimization

#### `user_card_practice` (Expected: Millions of rows)

**Optimizations:**
- Minimal indexes focused on query patterns
- Partial index for recent validations (last 30 days)
- Composite indexes for common access patterns
- No unnecessary indexes that slow down inserts

**Insert Strategy:**
- Use batch inserts when possible
- Consider async writes for non-critical validations
- Use `UserCardPracticeInsert` struct for optimized inserts

### 5. Query Pattern Indexes

| Query Pattern | Index Used | Performance |
|--------------|------------|-------------|
| Get decks by topic | `idx_decks_topic_id` | O(log n) |
| Get cards by deck | `idx_cards_deck_id` | O(log n) |
| Get user's deck practice | `uq_user_deck` (unique) | O(1) |
| Get user's validation history | `idx_ucp_user_card_validated` | O(log n) |
| Get recent validations | `idx_ucp_recent_validations` (partial) | O(log n) |
| Get user's decks by practice | `idx_udp_user_last_practiced` | O(log n) |

### 6. Foreign Key Constraints

- **ON DELETE CASCADE**: Ensures data integrity while allowing fast deletes
- All foreign keys are indexed automatically

### 7. Unique Constraints

- **`(user_id, deck_id)` on `user_deck_practice`**: Prevents duplicates and enables O(1) lookups

## Performance Benchmarks (Expected)

### Insert Performance
- **Single insert**: < 1ms (with proper connection pooling)
- **Batch insert (1000 rows)**: < 50ms
- **Concurrent inserts**: Scales linearly with connection pool size

### Read Performance
- **Get deck by topic**: < 5ms (with 1M+ decks)
- **Get user's practice**: < 1ms (unique index)
- **Get validation history**: < 10ms (with 10M+ validations)
- **Get recent validations**: < 5ms (partial index)

## Scaling Strategies

### Short-term (1M - 10M rows)
- Current schema handles this efficiently
- Connection pooling recommended (50-100 connections)

### Medium-term (10M - 100M rows)
- Consider partitioning `user_card_practice` by date (monthly partitions)
- Archive old validations (> 1 year) to separate table
- Read replicas for analytics queries

### Long-term (100M+ rows)
- Sharding by user_id or topic_id
- Time-series database for `user_card_practice` historical data
- Materialized views for common aggregations
- Caching layer (Redis) for frequently accessed data

## Maintenance Recommendations

1. **Regular VACUUM**: Keep table statistics updated
2. **ANALYZE**: Update query planner statistics weekly
3. **Index Maintenance**: Monitor index bloat, rebuild if needed
4. **Connection Pooling**: Use connection pool (e.g., deadpool, r2d2)
5. **Query Monitoring**: Log slow queries (> 100ms) for optimization

## Migration Notes

When applying this schema:
1. Create tables in order (topics → decks → cards → practice tables)
2. Add indexes after data insertion for faster initial load
3. Use `CREATE INDEX CONCURRENTLY` in production to avoid locks
4. Monitor index usage and remove unused indexes

