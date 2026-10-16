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

# Throwaway certificates for the mutual-TLS test: a CA, a server certificate
# for 127.0.0.1, and a client certificate for the user "thabi".
mtls="$dir/.mtls"
if [[ ! -f "$mtls/client.key" ]]; then
  mkdir -p "$mtls"
  # Every certificate is X.509 v3 with explicit extensions: rustls refuses
  # v1 certificates, which is what openssl makes without them.
  openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=thabi-it-ca" \
    -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -keyout "$mtls/ca.key" -out "$mtls/ca.crt" 2>/dev/null
  openssl req -newkey rsa:2048 -nodes -subj "/CN=localhost" \
    -keyout "$mtls/server.key" -out "$mtls/server.csr" 2>/dev/null
  printf 'basicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n' > "$mtls/server.ext"
  printf 'basicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=clientAuth\n' > "$mtls/client.ext"
  openssl x509 -req -in "$mtls/server.csr" -CA "$mtls/ca.crt" -CAkey "$mtls/ca.key" \
    -CAcreateserial -days 3650 -extfile "$mtls/server.ext" -out "$mtls/server.crt" 2>/dev/null
  openssl req -newkey rsa:2048 -nodes -subj "/CN=thabi" \
    -keyout "$mtls/client.key" -out "$mtls/client.csr" 2>/dev/null
  openssl x509 -req -in "$mtls/client.csr" -CA "$mtls/ca.crt" -CAkey "$mtls/ca.key" \
    -CAcreateserial -days 3650 -extfile "$mtls/client.ext" -out "$mtls/client.crt" 2>/dev/null
  # The server only lets in clients whose certificate the CA signed.
  printf 'local all all trust\nhostssl all all all cert\nhost all all all reject\n' > "$mtls/pg_hba.conf"
  chmod 644 "$mtls"/*.crt "$mtls/pg_hba.conf" "$mtls/server.key"
fi
export THABI_IT_MTLS_DIR="$mtls"

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
export THABI_IT_SSH_JUMP_PORT="${THABI_IT_SSH_JUMP_PORT:-52223}"
export THABI_IT_PG_MTLS_PORT="${THABI_IT_PG_MTLS_PORT:-55433}"

cargo test --manifest-path "$root/src-tauri/Cargo.toml" --test drivers --test ssh --test queries -- --ignored --test-threads=1 "$@"
