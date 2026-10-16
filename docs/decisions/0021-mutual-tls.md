# 0021 — Mutual TLS with a client certificate

**Status:** accepted · **Date:** 2026-10-15 · Phase 11, `plan.md` §7b

Some servers do not accept a password alone: they ask the client to identify
itself during the TLS handshake with a certificate. That is mutual TLS, and
the plan calls for it on MySQL and PostgreSQL, with SQL Server documented for
what its driver supports.

## What the drivers actually support

Checked against the crates in `Cargo.lock` rather than assumed:

| Engine     | Support                                                            |
| ---------- | ------------------------------------------------------------------ |
| MySQL      | `MySqlConnectOptions::ssl_client_cert` / `ssl_client_key`          |
| PostgreSQL | `PgConnectOptions::ssl_client_cert` / `ssl_client_key`             |
| SQL Server | `Config::client_certificate(cert, key)`, with the `rustls` backend |

All three therefore support it. `plan.md` §7b expected SQL Server to be the
exception; Tiberius 0.13 with the `rustls` backend this app already uses reads
PEM and DER files directly, so SQL Server is supported too. What differs is
the shape: a SQL Server TDS 8.0 "strict" connection
(`ENCRYPT_CLIENT_CERT`) _requires_ a client certificate, where for MySQL and
PostgreSQL it is optional and offered when the server asks.

`client_certificate_pkcs12` exists in Tiberius but is gated behind the
`native-tls` and `vendored-openssl` backends, so a `.pfx` bundle cannot be
used in this build. The UI picks PEM and DER files, which is what the rustls
path accepts; no other format is offered rather than offering one that fails.

## Decisions

1. **File paths, never contents.** `tls_client_cert` and `tls_client_key` are
   paths, read at connect time and never copied — the same rule as SSH keys.
   Migration `0011` adds both columns.
2. **Both or neither.** Saving one without the other is refused with a field
   error on both, because half a pair otherwise fails as an opaque handshake
   error long after the form is gone.
3. **TLS has to be on.** A client certificate with `tlsMode` set to Off is
   refused: there is no handshake to present it in, and accepting it would
   suggest protection that isn't there.
4. **Both files must be readable regular files,** bounded like the SSH key
   check, so a stale or wrong path is caught in the form.
5. **The key's location is not logged.** `ConnectParams`' `Debug` prints the
   certificate path but redacts the key path, because the filename alone
   identifies whose key is in use. A test asserts the key's name never appears
   in a debug rendering.
6. **It sits inside the existing TLS section** rather than getting its own
   dialog, so the CA certificate and the client certificate are read together,
   and it only appears when TLS isn't off.

## Verified

- `client_certificate_files_reach_the_driver` saves a profile with a real
  certificate and key on disk, reads them back through `connect_params`, and
  checks both reach the driver as paths while the key's name stays out of the
  debug output. A profile without them carries none.
- Validation tests cover: both files accepted, either one alone refused, both
  refused with TLS off, and a missing or unreadable file named by field.
- `npm run check` and all 55 integration tests pass, confirming the TLS paths
  for all three engines are unchanged for connections without a client
  certificate.

## Not done

No integration test proves a handshake against a server demanding a client
certificate: the Docker images don't request one. The driver calls are the
documented APIs for the versions in `Cargo.lock`, and the failure mode if a
server rejects the certificate is the engine's own TLS error, which
`database::error` already maps and explains.
