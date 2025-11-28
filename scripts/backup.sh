#!/bin/bash
# Database Backup Script for Matcha Time API

set -e

# Configuration
BACKUP_DIR="${BACKUP_DIR:-./backups/postgres}"
DB_CONTAINER="${DB_CONTAINER:-matcha-time-postgres-prod}"
DB_NAME="${DB_NAME:-matcha_db}"
DB_USER="${DB_USER:-matcha_user}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Generate backup filename with timestamp
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_FILE="$BACKUP_DIR/${DB_NAME}_${TIMESTAMP}.sql.gz"

log "Starting database backup..."

# Check if container is running
if ! docker ps | grep -q "$DB_CONTAINER"; then
    error "Database container $DB_CONTAINER is not running"
fi

# Perform backup
log "Backing up $DB_NAME from $DB_CONTAINER..."
docker exec -t "$DB_CONTAINER" pg_dump -U "$DB_USER" "$DB_NAME" | gzip > "$BACKUP_FILE"

# Verify backup
if [ -f "$BACKUP_FILE" ]; then
    SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
    log "Backup created: $BACKUP_FILE ($SIZE)"
else
    error "Backup failed - file not created"
fi

# Test backup integrity
log "Verifying backup integrity..."
if gunzip -t "$BACKUP_FILE" 2>/dev/null; then
    log "Backup integrity verified ✓"
else
    error "Backup file is corrupted"
fi

# Delete backups older than retention period
log "Cleaning up old backups (retention: $RETENTION_DAYS days)..."
DELETED=$(find "$BACKUP_DIR" -name "${DB_NAME}_*.sql.gz" -mtime +$RETENTION_DAYS -delete -print | wc -l)

if [ "$DELETED" -gt 0 ]; then
    log "Deleted $DELETED old backup(s)"
else
    log "No old backups to delete"
fi

# Show backup summary
log "========================================="
log "Backup Summary"
log "========================================="
log "Backup file: $BACKUP_FILE"
log "Size: $SIZE"
log "Retention: $RETENTION_DAYS days"
log "Total backups: $(ls -1 "$BACKUP_DIR"/${DB_NAME}_*.sql.gz | wc -l)"
log "========================================="

log "Backup completed successfully! ✓"
