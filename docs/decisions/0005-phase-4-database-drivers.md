# 0005 — Phase 4 database drivers

**Status:** accepted for the Phase 4 proof of concept · **Date:** 2026-10-14 · Parts superseded by 0006 (sessions, TLS modes, errors).

## Context

Phase 4 needs drivers for MySQL, PostgreSQL, and Microsoft SQL Server, all
behind Rust-owned connection lifecycle services. The selected crates must
build with the repository's Rust 1.96 toolchain, support asynchronous I/O, and
leave room for verified TLS and SSH tunnelling without exposing credentials to
the frontend. The existing SQLite app-data layer remains on `rusqlite`.

## Decisions

1. **Use SQLx 0.9 for MySQL and PostgreSQL.** It provides maintained async
   drivers for both engines, typed engine-specific APIs, and Tokio integration.
   SQLx removed its SQL Server driver in 0.7, so it is not the SQL Server
   implementation.
2. **Use Tiberius 0.13 for SQL Server.** It provides an async TDS client and
   accepts an injected async read/write transport, which avoids binding the
   connection layer to one socket implementation. The selected `tds80` and
   `rustls` features target current SQL Server protocol/TLS support without
   enabling the crate's default native-TLS and Windows-auth features.
3. **Use Tokio as the shared runtime.** SQLx uses its Tokio runtime feature.
   Tokio networking and synchronization are explicit dependencies for the
   connection lifecycle; Tauri continues to own the desktop runtime.
4. **Use rustls with native platform roots for SQLx.** The MySQL/PostgreSQL
   adapters must configure verification explicitly and must never enable
   insecure certificate bypasses. Tiberius must likewise use verified
   certificates; its `trust_cert` mode is not an acceptable default.
5. **Keep crate features narrow.** SQLx defaults are disabled to avoid
   unneeded macros, migrations, JSON support, and unrelated database drivers.
   Tiberius defaults are disabled to avoid Windows authentication and native
   TLS in this password-authentication MVP.
6. **Do not normalize away engine-specific capabilities.** Share lifecycle
   operations and result DTOs only where their semantics match. Preserve
   engine-specific behavior, types, and limitations in adapters.

## Feasibility notes

- `cargo info` reports SQLx 0.9.0 with Rust 1.94 minimum and Tiberius 0.13.0
  with Rust 1.88 minimum, below the project's Rust 1.96 toolchain.
- SQLx's upstream README documents PostgreSQL/MySQL support and explicitly
  notes SQL Server was removed. Its docs describe the Tokio runtime and rustls
  TLS backends.
- Tiberius docs describe runtime-independent TDS over an injected async
  transport and support for Tokio TCP streams. Its feature list includes
  TDS 8.0, rustls, and a SQL Browser feature for Tokio. With the rustls feature,
  its default trust source validates against system roots; `trust_cert` is
  explicitly not used.
- The Rust connection service now opens and health-checks all three driver
  types, enforces hostname/certificate verification, and stores live
  connections in process memory. SSH-enabled profiles are rejected until the
  Phase 5 tunnel transport exists rather than falling back to direct TCP.
- The application-level certificate-verification configuration, connect and
  query timeouts, result mapping, cancellation behavior, and all three live
  database connections remain to be proven in this phase. Connect and health
  checks currently use bounded timeouts; query execution/result mapping and
  cancellation are not yet implemented. No driver is considered
  production-ready based on dependency selection alone.
- Live integration tests need local database services. They must use
  environment-provided test configuration and must not commit credentials.

## References

- [SQLx documentation](https://docs.rs/sqlx/0.9.0/sqlx/)
- [SQLx repository and supported databases](https://github.com/launchbadge/sqlx)
- [Tiberius documentation](https://docs.rs/tiberius/0.13.0/tiberius/)
- [Tiberius repository](https://github.com/prisma/tiberius)
