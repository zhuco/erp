#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   BACKUP_DIR=/var/backups/erp BACKUP_RETENTION_DAYS=14 DATABASE_URL=postgres://... ./backup_postgres.sh
#
# Retention: remove backups older than BACKUP_RETENTION_DAYS (default 7 days).

BACKUP_DIR="${BACKUP_DIR:-./backups}"
BACKUP_RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-7}"
DATABASE_URL="${DATABASE_URL:-postgres://erp:erp@localhost:5432/erp}"

timestamp="$(date -u +"%Y%m%dT%H%M%SZ")"
mkdir -p "${BACKUP_DIR}"
backup_file="${BACKUP_DIR}/erp_${timestamp}.dump"

umask 077
pg_dump --format=custom --file "${backup_file}" --dbname "${DATABASE_URL}"

find "${BACKUP_DIR}" -type f -name "erp_*.dump" -mtime +"${BACKUP_RETENTION_DAYS}" -delete
