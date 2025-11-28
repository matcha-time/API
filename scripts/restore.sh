#!/bin/bash
# Database Restore Script for Matcha Time API

set -e

# Configuration
BACKUP_DIR="${BACKUP_DIR:-./backups/postgres}"
DB_CONTAINER="${DB_CONTAINER:-matcha-time-postgres-prod}"
DB_NAME="${DB_NAME:-matcha_db}"
DB_USER="${DB_USER:-matcha_user}"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check if backup file is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <backup_file.sql.gz>"
    echo ""
    echo "Available backups:"
    ls -lh "$BACKUP_DIR"/*.sql.gz 2>/dev/null || echo "No backups found in $BACKUP_DIR"
    exit 1
fi

BACKUP_FILE="$1"

# Check if backup file exists
if [ ! -f "$BACKUP_FILE" ]; then
    error "Backup file not found: $BACKUP_FILE"
fi

# Verify backup integrity
log "Verifying backup integrity..."
if ! gunzip -t "$BACKUP_FILE" 2>/dev/null; then
    error "Backup file is corrupted or invalid"
fi
log "Backup integrity verified ✓"

# Check if container is running
if ! docker ps | grep -q "$DB_CONTAINER"; then
    error "Database container $DB_CONTAINER is not running"
fi

# Warning prompt
warn "========================================="
warn "WARNING: Database Restore Operation"
warn "========================================="
warn "This will:"
warn "  1. Stop the API server"
warn "  2. Drop and recreate the database"
warn "  3. Restore from: $BACKUP_FILE"
warn "  4. Restart the API server"
warn ""
warn "ALL CURRENT DATA WILL BE LOST!"
warn "========================================="

read -p "Are you sure you want to continue? (type 'yes' to proceed): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    log "Restore cancelled"
    exit 0
fi

# Stop API to prevent writes during restore
log "Stopping API server..."
docker-compose -f compose.prod.yaml stop api || warn "API not running"

# Create a pre-restore backup
log "Creating pre-restore backup..."
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
PRE_RESTORE_BACKUP="$BACKUP_DIR/pre_restore_${TIMESTAMP}.sql.gz"
docker exec -t "$DB_CONTAINER" pg_dump -U "$DB_USER" "$DB_NAME" | gzip > "$PRE_RESTORE_BACKUP" || warn "Pre-restore backup failed"

# Drop existing connections
log "Terminating existing database connections..."
docker exec -t "$DB_CONTAINER" psql -U "$DB_USER" -d postgres -c \
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB_NAME' AND pid <> pg_backend_pid();"

# Drop and recreate database
log "Dropping and recreating database..."
docker exec -t "$DB_CONTAINER" psql -U "$DB_USER" -d postgres -c "DROP DATABASE IF EXISTS $DB_NAME;"
docker exec -t "$DB_CONTAINER" psql -U "$DB_USER" -d postgres -c "CREATE DATABASE $DB_NAME;"

# Restore from backup
log "Restoring database from backup..."
gunzip -c "$BACKUP_FILE" | docker exec -i "$DB_CONTAINER" psql -U "$DB_USER" -d "$DB_NAME"

# Verify restore
log "Verifying restore..."
TABLE_COUNT=$(docker exec -t "$DB_CONTAINER" psql -U "$DB_USER" -d "$DB_NAME" -t -c \
    "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';")

log "Tables restored: $TABLE_COUNT"

if [ "$TABLE_COUNT" -eq 0 ]; then
    error "Restore verification failed - no tables found"
fi

# Restart API
log "Restarting API server..."
docker-compose -f compose.prod.yaml start api

# Wait for API to be ready
log "Waiting for API to be ready..."
sleep 5

MAX_RETRIES=30
RETRY_COUNT=0

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if curl -f http://localhost:3000/health/ready &> /dev/null; then
        log "API is ready ✓"
        break
    fi

    RETRY_COUNT=$((RETRY_COUNT + 1))
    log "Waiting... ($RETRY_COUNT/$MAX_RETRIES)"
    sleep 2
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    error "API failed to start after restore"
fi

# Show restore summary
log "========================================="
log "Restore Summary"
log "========================================="
log "Restored from: $BACKUP_FILE"
log "Pre-restore backup: $PRE_RESTORE_BACKUP"
log "Tables restored: $TABLE_COUNT"
log "API status: Running ✓"
log "========================================="

log "Database restore completed successfully! ✓"
