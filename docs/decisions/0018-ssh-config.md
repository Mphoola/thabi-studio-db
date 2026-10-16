# 0018 — Reading `~/.ssh/config`

**Status:** accepted · **Date:** 2026-10-15 · Phase 11, `plan.md` §7b

A connection form asking for an SSH host, user, port and key file repeats what
the user may already have written down. If they type a `Host` alias from their
own SSH config instead, Thabi Studio now uses the same settings `ssh` would.

## Decisions

1. **The file is only ever read.** Thabi Studio never writes to
   `~/.ssh/config`, and never edits it — not even to record a trusted host key.
2. **Only an exact alias resolves.** Wildcard and negated patterns (`*`, `?`,
   `!`) are ignored on purpose: a broad block like `Host *` must not silently
   decide how one connection connects. If the alias isn't there, nothing
   happens and the user types their host by hand as before.
3. **`Include` is not followed.** An include can pull in files the user didn't
   mean for this app to read, and deciding which are safe would be guesswork.
   The form says where the settings came from, so an include-only setup shows
   no result rather than a surprising one.
4. **`ssh_config` semantics for precedence.** The first value seen for a
   keyword wins, so a later broad block cannot override an earlier specific
   one. Quoted values keep their spaces (`User "second name"`), and `~` is
   expanded in `IdentityFile`.
5. **What is read:** `HostName`, `User`, `Port`, `IdentityFile` and one
   `ProxyJump` hop. Unknown keywords are ignored, as are patterns that name
   more than one host.
6. **The user sees the resolution before using it.** Typing an alias shows
   what it resolves to and which file it came from, with **Use These
   Settings** to fill the form. Nothing is applied silently. A `ProxyJump` is
   spelled out: "Thabi Studio connects to <jump> first, then on to this
   server."

## Connecting through a jump host

A `ProxyJump` from the alias is followed when connecting: Thabi Studio opens a
tunnel to the jump host, forwards through it to the SSH server the alias names,
and then carries the database connection as usual. The jump tunnel is owned by
the tunnel that uses it (`Tunnel::hop`), so closing the connection closes both.

- **One hop only.** A jump host that itself has a `ProxyJump` is not chained;
  two hops would need more host-key prompts and more ways to get it wrong.
- **Sign-in to the jump host** uses the same method as the target connection
  (password, key file, or agent), with the jump's own `User`, `Port` and
  `IdentityFile` when its config entry names them.
- **Host keys are checked on both hops**, separately: the jump is a different
  server from the one behind it, and each needs its own confirmation.
- **The alias resolves at connect time too**, not only in the form, so a saved
  connection keeps working when the user's config supplies the details.

## Verified

- Unit: `ssh::config` tests cover exact matches, case-insensitivity, quoted
  values, wildcards and negations being ignored, `Include` being skipped,
  first-block precedence, `~` expansion, `Keyword=value`, and parsing a
  `ProxyJump` target with a user and port.
- Integration: `a_proxy_jump_config_reaches_the_server_through_a_bastion` runs
  a real query through two bastions in Docker, the second reached only through
  the first; `closing_a_jumped_tunnel_closes_the_jump` proves closing the
  connection tears the jump tunnel down as well. Both bastions are the same
  image and the jump reaches the target by its Compose service name, so no
  extra port is published.
