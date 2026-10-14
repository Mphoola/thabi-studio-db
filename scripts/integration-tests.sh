#!/usr/bin/env bash
# Runs the database driver integration tests against local Docker databases.
#   scripts/integration-tests.sh          start databases, run tests
#   scripts/integration-tests.sh down     stop and remove the databases
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/tests/integration"
env_file="$dir/.env"
compose=(docker compose -f "$dir/docker-compose.yml" --env-file "$env_file")

# A throwaway password, generated once per checkout and never committed.
# SQL Server needs upper, lower, digit and symbol.
if [[ ! -f "$env_file" ]]; then
  printf 'THABI_IT_PASSWORD=Th1!%s\n' "$(openssl rand -hex 12)" > "$env_file"
fi

if [[ "${1:-}" == "down" ]]; then
  "${compose[@]}" down -v
  exit 0
fi

"${compose[@]}" up -d --wait

set -a
# shellcheck source=/dev/null
source "$env_file"
set +a
export THABI_IT_PG_PORT="${THABI_IT_PG_PORT:-55432}"
export THABI_IT_MYSQL_PORT="${THABI_IT_MYSQL_PORT:-53306}"
export THABI_IT_MSSQL_PORT="${THABI_IT_MSSQL_PORT:-51433}"

cargo test --manifest-path "$root/src-tauri/Cargo.toml" --test drivers -- --ignored --test-threads=1 "$@"
