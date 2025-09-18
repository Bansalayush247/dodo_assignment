#!/usr/bin/env bash
set -euo pipefail

DB_URL="${DATABASE_URL:-postgres://postgres:postgres@db:5432/transaction_service}"

echo "Waiting for database..."

# Use pg_isready if available, otherwise fall back to psql loop.
if c                          ommand -v pg_isready >/dev/null 2>&1; then
  until pg_isready -q -d "$DB_URL"; do
    sleep 1
  done
else
  # psql accepts a libpq connection string; try connecting in a loop
  until psql "$DB_URL" -c '\q' >/dev/null 2>&1; do
    sleep 1
  done
fi

echo "Applying migrations..."
for f in /app/migrations/*_create_*.sql; do
  # only apply up migrations (files with .down.sql are rollback files)
  if [[ "$f" != *.down.sql ]]; then
    echo "Applying $f"
    # Use psql with connection string; forward stdout/stderr so failures are visible
    psql "$DB_URL" -f "$f"
  fi
done

echo "Starting app..."
exec /app/transaction-service
