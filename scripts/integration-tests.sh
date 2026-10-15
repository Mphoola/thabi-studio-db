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

# Throwaway SSH keys for the tunnel tests: one plain, one with a passphrase.
keys="$dir/.ssh"
if [[ ! -f "$keys/id_plain" ]]; then
  mkdir -p "$keys"
  passphrase="$(grep THABI_IT_PASSWORD "$env_file" | cut -d= -f2-)"
  ssh-keygen -q -t ed25519 -N "" -C thabi-it-plain -f "$keys/id_plain"
  ssh-keygen -q -t ed25519 -N "$passphrase" -C thabi-it-locked -f "$keys/id_locked"
fi
export THABI_IT_AUTHORIZED_KEYS="$(cat "$keys/id_plain.pub" "$keys/id_locked.pub")"
export THABI_IT_SSH_KEY_DIR="$keys"

if [[ "${1:-}" == "down" ]]; then
  "${compose[@]}" down -v
  exit 0
fi

"${compose[@]}" up -d --build --wait

set -a
# shellcheck source=/dev/null
source "$env_file"
set +a
export THABI_IT_PG_PORT="${THABI_IT_PG_PORT:-55432}"
export THABI_IT_MYSQL_PORT="${THABI_IT_MYSQL_PORT:-53306}"
export THABI_IT_MSSQL_PORT="${THABI_IT_MSSQL_PORT:-51433}"
export THABI_IT_SSH_PORT="${THABI_IT_SSH_PORT:-52222}"

cargo test --manifest-path "$root/src-tauri/Cargo.toml" --test drivers --test ssh --test queries -- --ignored --test-threads=1 "$@"
