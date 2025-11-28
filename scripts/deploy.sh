#!/bin/bash
# Production Deployment Script for Matcha Time API

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="compose.prod.yaml"
ENV_FILE=".env.production"
BACKUP_DIR="./backups"
LOG_FILE="./deployment.log"

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
    exit 1
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."

    if ! command -v docker &> /dev/null; then
        error "Docker is not installed. Please install Docker first."
    fi

    if ! command -v docker-compose &> /dev/null; then
        error "Docker Compose is not installed. Please install Docker Compose first."
    fi

    if [ ! -f "$ENV_FILE" ]; then
        error "Environment file $ENV_FILE not found. Copy .env.production.example and configure it."
    fi

    log "Prerequisites check passed ✓"
}

# Backup database before deployment
backup_database() {
    log "Creating database backup..."

    mkdir -p "$BACKUP_DIR/postgres"

    TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
    BACKUP_FILE="$BACKUP_DIR/postgres/pre_deploy_${TIMESTAMP}.sql.gz"

    if docker-compose -f "$COMPOSE_FILE" ps | grep -q "postgres"; then
        docker exec matcha-time-postgres-prod pg_dump -U matcha_user matcha_db | gzip > "$BACKUP_FILE" || warn "Backup failed, continuing anyway"

        if [ -f "$BACKUP_FILE" ]; then
            log "Backup created: $BACKUP_FILE ($(du -h "$BACKUP_FILE" | cut -f1))"
        fi
    else
        warn "Database not running, skipping backup"
    fi
}

# Build new image
build_image() {
    log "Building Docker image..."
    docker build -t matcha-time-api:latest . || error "Build failed"
    log "Image built successfully ✓"
}

# Deploy with zero downtime
deploy() {
    log "Deploying to production..."

    # Pull latest images
    docker-compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" pull

    # Start new containers
    docker-compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" up -d --remove-orphans

    log "Deployment complete ✓"
}

# Health check
health_check() {
    log "Running health checks..."

    sleep 5  # Wait for container to start

    MAX_RETRIES=30
    RETRY_COUNT=0

    while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
        if curl -f http://localhost:3000/health/ready &> /dev/null; then
            log "Health check passed ✓"
            return 0
        fi

        RETRY_COUNT=$((RETRY_COUNT + 1))
        log "Waiting for service to be ready... ($RETRY_COUNT/$MAX_RETRIES)"
        sleep 2
    done

    error "Health check failed after $MAX_RETRIES attempts"
}

# Cleanup old images
cleanup() {
    log "Cleaning up old images..."
    docker image prune -f || warn "Cleanup failed"
    log "Cleanup complete ✓"
}

# Rollback function
rollback() {
    warn "Rolling back to previous version..."
    docker-compose -f "$COMPOSE_FILE" --env-file "$ENV_FILE" down

    # Restore from backup if available
    LATEST_BACKUP=$(ls -t "$BACKUP_DIR/postgres"/pre_deploy_*.sql.gz 2>/dev/null | head -1)
    if [ -n "$LATEST_BACKUP" ]; then
        log "Restoring database from $LATEST_BACKUP"
        gunzip -c "$LATEST_BACKUP" | docker exec -i matcha-time-postgres-prod psql -U matcha_user -d matcha_db
    fi

    error "Rollback complete. Please investigate the issue."
}

# Show deployment summary
show_summary() {
    echo ""
    log "========================================="
    log "Deployment Summary"
    log "========================================="
    log "Status: Success ✓"
    log "Time: $(date)"
    log ""
    log "Endpoints:"
    log "  - API: http://localhost:3000"
    log "  - Health: http://localhost:3000/health"
    log "  - Readiness: http://localhost:3000/health/ready"
    log "  - Metrics: http://localhost:3000/metrics"
    log ""
    log "Useful commands:"
    log "  View logs: docker-compose -f $COMPOSE_FILE logs -f api"
    log "  Stop: docker-compose -f $COMPOSE_FILE down"
    log "  Restart: docker-compose -f $COMPOSE_FILE restart api"
    log "========================================="
}

# Main deployment flow
main() {
    log "Starting Matcha Time API deployment..."

    # Trap errors and rollback
    trap rollback ERR

    check_prerequisites
    backup_database
    build_image
    deploy
    health_check
    cleanup
    show_summary

    log "Deployment successful! 🚀"
}

# Run main function
main "$@"
