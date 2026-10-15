# 0007 — SSH tunnels

**Status:** accepted · **Date:** 2026-10-15

## Decisions

1. **`russh` 0.64** (Apache-2.0, pure Rust, actively maintained) with the
   `ring` backend, which the TLS stack already uses, so there's no extra C
   crypto build. RSA and zlib compression are enabled for older servers.
2. **How the tunnel works.** Each database session gets its own SSH session
   and a listener on `127.0.0.1:<random port>`. Connections to it become
   `direct-tcpip` channels to the database host _as seen from the SSH
   server_. The listener never binds outside loopback. One channel is opened
   up front, so a wrong database host fails at connect time with its own
   message instead of a confusing driver error.
3. **Lifetime.** The tunnel belongs to the `Session`: closing the session,
   disconnecting, deleting or editing the profile, quitting the app, a
   failed driver connect and the idle/health-check closes all close it.
   An integration test checks the local port stops accepting afterwards.
4. **Host keys are never accepted silently.**
   - Trusted keys are those the user confirmed in Thabi Studio (SQLite
     table `ssh_known_hosts`, one key per host, port and key type) plus,
     read-only, the user's `~/.ssh/known_hosts`.
   - Unknown key: the connection fails with `ssh_host_unknown` and the key's
     SHA-256 fingerprint. The app asks "Trust “host”?" showing the
     fingerprint; only then is the key stored and the connection retried.
   - Changed key (a different key of the same type): `ssh_host_changed`,
     refused. The user can forget Thabi's saved key after a destructive
     confirmation; a key in `~/.ssh/known_hosts` must be removed by the user
     (`ssh-keygen -R`), and the app says so.
   - Host certificates aren't supported yet and are rejected.
5. **Sign-in methods.**
   - Password, stored in Keychain.
   - Private key file: only the _path_ is stored; the file is read when
     connecting and never copied. An optional passphrase is stored in
     Keychain. Files over 100 KB are refused.
   - SSH agent (`SSH_AUTH_SOCK`): tries each key the agent offers.
6. **TLS through a tunnel.** PostgreSQL and MySQL see the TLS peer as
   `127.0.0.1`, so "Verify certificate" checks the certificate chain but not
   the host name (`VerifyCa`); the SSH host key protects the path. SQL Server
   keeps full host-name verification because Tiberius lets us connect TCP to
   the tunnel while verifying the real host. The form explains this.
7. **Errors:** SSH host not found, refused, timeout, authentication
   rejected, key unreadable or passphrase needed, agent unavailable, and
   "the SSH server couldn't reach the database host" each have their own
   message. Secrets never appear in errors or `Debug` output.
8. **Security indicator.** The status bar shows the selected connection's
   protection: Verified TLS, Encrypted (unverified) or Not encrypted, plus
   "via SSH". Unencrypted remote connections are shown in warning colour.
9. Migration 0004 rebuilds `connection_profiles` (SQLite can't change a
   CHECK constraint) to allow `key` and `agent`, and adds `ssh_key_path`,
   `has_ssh_key_passphrase` and the `ssh_known_hosts` table.

## VPNs and private networks

No special support is needed: if your Mac can reach the database through a
VPN or private network, connect directly to its private address or name.
Use an SSH tunnel when the database is only reachable from a bastion host.

## Verified

`npm run test:integration` adds an Alpine OpenSSH bastion and 5 tests:
unknown-key prompt then success, changed key refused, all three engines
through the tunnel (password, plain key, passphrase-protected key), tunnel
closed after the session closes, and SSH failure messages (wrong password,
locked key without passphrase, unreachable database host, refused SSH port).
ssh-agent sign-in is implemented but not covered by automated tests.
