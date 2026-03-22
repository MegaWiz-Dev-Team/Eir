#!/bin/bash
# ============================================================
# Eir (OpenEMR) — Local Deploy Script
# 
# Usage:
#   ./scripts/deploy.sh                  # Deploy production
#   ./scripts/deploy.sh --site sandbox   # Deploy sandbox
#   ./scripts/deploy.sh --dry-run        # Preview only
#   ./scripts/deploy.sh --migrate-only   # Run SQL migrations only
#
# This script runs ON the Eir server directly.
# It handles: git pull → PHP lint → SQL migrations → Apache reload
# ============================================================

set -euo pipefail

# ─── Config ───────────────────────────────────────────────────
EIR_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SITE="default"
DRY_RUN=false
MIGRATE_ONLY=false
MIGRATIONS_DIR="${EIR_ROOT}/sql/migrations"
MIGRATIONS_APPLIED_LOG="${EIR_ROOT}/sql/.migrations_applied"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ─── Parse Args ───────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case $1 in
        --site)       SITE="$2"; shift 2 ;;
        --dry-run)    DRY_RUN=true; shift ;;
        --migrate-only) MIGRATE_ONLY=true; shift ;;
        -h|--help)
            echo "Usage: $0 [--site default|sandbox] [--dry-run] [--migrate-only]"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ─── Read DB credentials from sqlconf.php ─────────────────────
SQLCONF="${EIR_ROOT}/sites/${SITE}/sqlconf.php"
if [[ ! -f "$SQLCONF" ]]; then
    echo -e "${RED}✗ sqlconf.php not found at: ${SQLCONF}${NC}"
    exit 1
fi

DB_HOST=$(php -r "include '${SQLCONF}'; echo \$host;")
DB_PORT=$(php -r "include '${SQLCONF}'; echo \$port;")
DB_USER=$(php -r "include '${SQLCONF}'; echo \$login;")
DB_PASS=$(php -r "include '${SQLCONF}'; echo \$pass;")
DB_NAME=$(php -r "include '${SQLCONF}'; echo \$dbase;")

# ─── Helpers ──────────────────────────────────────────────────
log_info()  { echo -e "${BLUE}[INFO]${NC}  $1"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

run_mysql() {
    mysql -h "$DB_HOST" -P "$DB_PORT" -u "$DB_USER" -p"$DB_PASS" "$DB_NAME" "$@"
}

# ─── Banner ───────────────────────────────────────────────────
echo ""
echo -e "${BLUE}══════════════════════════════════════════${NC}"
echo -e "${BLUE}  Eir (OpenEMR) Deploy — Site: ${YELLOW}${SITE}${NC}"
echo -e "${BLUE}  ${TIMESTAMP}${NC}"
if $DRY_RUN; then
    echo -e "${YELLOW}  ⚠ DRY RUN MODE — no changes will be made${NC}"
fi
echo -e "${BLUE}══════════════════════════════════════════${NC}"
echo ""

# ─── Step 1: Git Pull ────────────────────────────────────────
if ! $MIGRATE_ONLY; then
    log_info "Step 1/4: Pulling latest code..."

    cd "$EIR_ROOT"
    CURRENT_BRANCH=$(git branch --show-current)
    CURRENT_COMMIT=$(git rev-parse --short HEAD)

    if $DRY_RUN; then
        log_warn "[DRY RUN] Would run: git pull origin ${CURRENT_BRANCH}"
        git fetch origin "$CURRENT_BRANCH" --dry-run 2>&1 || true
    else
        git stash --quiet 2>/dev/null || true
        git pull origin "$CURRENT_BRANCH" --ff-only
        git stash pop --quiet 2>/dev/null || true
    fi

    NEW_COMMIT=$(git rev-parse --short HEAD)
    if [[ "$CURRENT_COMMIT" == "$NEW_COMMIT" ]]; then
        log_ok "Already up-to-date (${CURRENT_COMMIT})"
    else
        log_ok "Updated: ${CURRENT_COMMIT} → ${NEW_COMMIT}"
    fi

    # ─── Step 2: PHP Lint ─────────────────────────────────────
    log_info "Step 2/4: Running PHP syntax check on changed files..."

    CHANGED_PHP=$(git diff --name-only "${CURRENT_COMMIT}..${NEW_COMMIT}" -- '*.php' 2>/dev/null || echo "")
    if [[ -n "$CHANGED_PHP" ]]; then
        LINT_ERRORS=0
        while IFS= read -r file; do
            if [[ -f "$file" ]]; then
                if ! php -l "$file" > /dev/null 2>&1; then
                    log_error "Syntax error in: $file"
                    LINT_ERRORS=$((LINT_ERRORS + 1))
                fi
            fi
        done <<< "$CHANGED_PHP"

        if [[ $LINT_ERRORS -gt 0 ]]; then
            log_error "${LINT_ERRORS} PHP file(s) have syntax errors. Aborting deploy."
            exit 1
        fi
        log_ok "All changed PHP files pass syntax check"
    else
        log_ok "No PHP files changed"
    fi
else
    log_info "Skipping git pull and lint (--migrate-only)"
fi

# ─── Step 3: SQL Migrations ──────────────────────────────────
log_info "Step 3/4: Running SQL migrations..."

# Create tracking file if it doesn't exist
touch "$MIGRATIONS_APPLIED_LOG"

if [[ ! -d "$MIGRATIONS_DIR" ]]; then
    log_warn "No migrations directory found at ${MIGRATIONS_DIR}"
else
    MIGRATION_COUNT=0
    SKIP_COUNT=0

    for sql_file in "$MIGRATIONS_DIR"/*.sql; do
        [[ -f "$sql_file" ]] || continue
        
        BASENAME=$(basename "$sql_file")

        # Check if already applied
        if grep -qF "$BASENAME" "$MIGRATIONS_APPLIED_LOG" 2>/dev/null; then
            SKIP_COUNT=$((SKIP_COUNT + 1))
            continue
        fi

        if $DRY_RUN; then
            log_warn "[DRY RUN] Would apply: ${BASENAME}"
        else
            log_info "Applying migration: ${BASENAME} → ${DB_NAME}"
            if run_mysql < "$sql_file" 2>&1; then
                echo "${BASENAME}  # applied ${TIMESTAMP}" >> "$MIGRATIONS_APPLIED_LOG"
                log_ok "Applied: ${BASENAME}"
            else
                log_error "Failed to apply: ${BASENAME}"
                exit 1
            fi
        fi

        MIGRATION_COUNT=$((MIGRATION_COUNT + 1))
    done

    if [[ $MIGRATION_COUNT -eq 0 ]]; then
        log_ok "No new migrations to apply (${SKIP_COUNT} already applied)"
    else
        log_ok "Applied ${MIGRATION_COUNT} migration(s), skipped ${SKIP_COUNT}"
    fi
fi

# ─── Step 4: Reload Apache ───────────────────────────────────
if ! $MIGRATE_ONLY && ! $DRY_RUN; then
    log_info "Step 4/4: Reloading Apache..."

    if command -v apachectl &> /dev/null; then
        sudo apachectl graceful 2>/dev/null && log_ok "Apache reloaded" || log_warn "Apache reload failed (may need sudo)"
    elif command -v systemctl &> /dev/null; then
        sudo systemctl reload apache2 2>/dev/null && log_ok "Apache reloaded" || log_warn "Apache reload skipped"
    else
        log_warn "Could not find apachectl or systemctl — please reload Apache manually"
    fi
else
    log_info "Skipping Apache reload"
fi

# ─── Summary ─────────────────────────────────────────────────
echo ""
echo -e "${GREEN}══════════════════════════════════════════${NC}"
echo -e "${GREEN}  Deploy complete! ✅${NC}"
echo -e "${GREEN}  Site:     ${SITE}${NC}"
echo -e "${GREEN}  Database: ${DB_NAME}${NC}"
echo -e "${GREEN}  Branch:   $(git branch --show-current 2>/dev/null || echo 'N/A')${NC}"
echo -e "${GREEN}  Commit:   $(git rev-parse --short HEAD 2>/dev/null || echo 'N/A')${NC}"
echo -e "${GREEN}══════════════════════════════════════════${NC}"
echo ""
