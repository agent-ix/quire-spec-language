# Current-head integration lane

FR-058 (ADR-011 §7.1 T-12, [#215](https://github.com/agent-ix/quire-spec-language/issues/215)).

## What this is

QSL's own root `Cargo.toml`/`Cargo.lock` resolve `quire-contract-ir`,
`quire-contract-runtime` and `quire-contract-codegen` at one pinned git
revision each -- the **exact-pin lane**, unchanged by anything here. This
directory is a second, separate lane that resolves those three dependencies
at each repository's current default-branch head instead, so a cross-repository
incompatibility is visible before it is pinned.

This crate is **not** a member of `../../Cargo.toml`'s `[workspace]`. Running
it never touches the root `Cargo.lock`.

## Local invocation

From the repository root:

```bash
# Vendor/refresh the local clones the lane's [patch] entries need (see "Why
# a vendored clone" below), then refresh this lane's own Cargo.lock to each
# dependency's current head (never the root workspace's Cargo.lock). Re-run
# any time to pick up a new IR/RT head.
cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
  prepare --vendor-root integration/current-head/.vendor \
  --manifest integration/current-head/Cargo.toml

# Build and test the lane at current head (needs network access to fetch
# quire-contract-runtime's and quire-contract-codegen's default branch).
cargo test --manifest-path integration/current-head/Cargo.toml

# Record the exact commit resolved for QSL and for each of IR/RT/CG.
cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
  revision-log --qsl . --manifest integration/current-head/Cargo.toml \
  --vendor-root integration/current-head/.vendor

# Confirm the intentionally incompatible fixture still fails, with a stable
# diagnostic (FR-058-AC-3).
cargo run --manifest-path integration/current-head/tool/Cargo.toml -- \
  check-incompatible-fixture --manifest integration/current-head/fixtures/incompatible/Cargo.toml
```

`make integration-current-head-prepare`, `make integration-current-head` and
`make integration-current-head-incompatible-fixture` (repository root
`Makefile`) run these.

### Why a vendored clone, not a plain `branch = "main"` dependency

quire-contract-codegen is declared directly at `branch = "main"`, which is
enough for it: nothing else in this lane's graph pins a conflicting revision
of it. quire-contract-ir and quire-contract-runtime are different: each is
reachable through more than one path to a *different* pinned `rev` of the
same repository elsewhere in the graph (QSL's own root `Cargo.toml` pins IR;
quire-contract-codegen's own manifest pins both IR and RT at its own, older
revisions), and moving *those* transitive dependencies to head is exactly the
point of this lane. Cargo refuses a `[patch]` whose replacement is a
different branch/rev/tag of the *same* git URL ("patches must point to
different sources") -- patch is for redirecting to a genuinely different
source. Redirecting to a local path clone of the current head is Cargo's
supported mechanism for this, so `tool/`'s `prepare` subcommand keeps
`.vendor/quire-contract-ir` and `.vendor/quire-contract-runtime` (both at
current head) fresh, and the lane's `[patch]` table points IR, RT and
quire-spec-language itself (a direct path patch, since QSL's own manifest is
already a path dependency of this lane) at those local sources. This is what
converges the lane's own `Cargo.lock` to exactly one revision per repository
(#249 review R3; `make arch-lint-duplicate-revisions-lane` checks it).

## What it checks (FR-058)

- **Schema/API incompatibility** and **stale generated artifacts**: the
  representative contract test (`tests/contract.rs`) calls one real, public
  entry point of each of the four repositories, tied together where
  practical by shared data (QSL's real parse digest feeds
  quire-contract-runtime's identity type). A breaking signature, type or
  schema change on any repository's head fails this test or fails the build
  outright.
- **Missing version support**: an incompatible Rust edition/feature/toolchain
  requirement on a dependency's head fails the build.
- **Silent fallback to released deps**: none of the three dependencies has a
  version fallback -- each is a git dependency only, so a resolution failure
  is a lane failure, never a quiet substitution.
- **Staleness**: `prepare` refreshes this lane's own `Cargo.lock` via `cargo
  update` on every run, so it never re-resolves a stale, previously-committed
  revision (#249 review HIGH-1). `revision-log` additionally compares each
  resolved sha against `git ls-remote <url> main` and fails on a mismatch, so
  a `Cargo.lock` that fell behind head cannot silently pass as current.
- **Intentionally incompatible fixture**: `fixtures/incompatible/` patches
  quire-contract-ir's `quire-contract-model` package to
  `stub-quire-contract-model/`, a local, deliberately empty stand-in crate
  that declares none of the types quire-spec-language's real source imports
  from it. Building it is expected to fail to compile with unresolved-import
  errors; running it through `check-incompatible-fixture` turns that failure
  into the stable marker line `FR-058-AC-3: quire-contract-ir patched to a
  deliberately empty stub crate is incompatible with quire-spec-language at
  head`. The marker is only emitted when the build's stderr actually contains
  a real `error[E0` diagnostic; an unrelated build failure (a missing
  manifest, a toolchain error) is reported as a distinct failure instead of
  being misreported as the expected one (#249 review HIGH-3).

### A real finding this lane already surfaced

Building this lane's own manifest at real current heads (not the
intentionally incompatible fixture) currently fails: real
quire-contract-codegen head does not compile against real
quire-contract-runtime head (`error[E0560]: struct CounterexamplePacket has
no field named witness`, RT `bounded_kani_corpus.rs:196` -- RT's real head
removed or renamed that field, leaving only `source`). This is exactly the
class of incompatibility this lane exists to catch, reported here as real,
current evidence; it is a CG/RT concern to fix, not this lane's or QSL's, and
the ownership procedure below applies to it directly.

## What it deliberately does not attempt

`tests/contract.rs` does not build a full checked-package v2 round trip
(QSL emits a package, quire-contract-codegen generates oracle code from it,
quire-contract-runtime executes and accounts for the result). QSL's real
job/model/fixture construction for that pipeline lives in `tests/support/`,
internal to QSL's own test tree, and quire-contract-codegen's and
quire-contract-runtime's deeper APIs (bound-oracle generation, campaign
accounting, verdicts) are intentionally constructible only from their own
crates' internal producers (a real generated-oracle package, a real LLVM
export, a real evaluated verdict), not from arbitrary external data. Wiring
QSL's internal fixture machinery through this separate, external lane crate
is out of this change's proportional scope. The contract test instead
exercises each crate's simplest genuinely public, unrestricted surface,
which still fails immediately if that surface's shape changes incompatibly.

## Open design questions this lane does not settle

Reported to [#215](https://github.com/agent-ix/quire-spec-language/issues/215)
rather than decided here:

1. **Where hosted CI runs this lane.** AGENTS.md's existing rule ("checks run
   locally until stable... every hosted workflow must be manual-dispatch
   only") applies to any workflow that would run it, but this change adds no
   such workflow. Today the lane is local-only, via the Makefile target
   above.
2. **How QSpec artifacts would be sourced at head.** This lane covers QSL,
   quire-contract-ir, quire-contract-runtime and quire-contract-codegen only.
   QSpec artifacts (the vendored trees `revendor`/`revendor-check` manage
   under `QSPEC_CLONE`) are consumed unchanged, at whatever revision the root
   manifest already vendors; moving that sourcing to head is unresolved.

## Ownership and update procedure

Owned by whoever owns ADR-011 T-12 (currently tracked under #215 and its
successors). When a run fails:

1. Read the failure: a compile error names the incompatible crate and symbol;
   a contract-test failure names which of the four repositories' surface
   changed.
2. Check the revision log (`revision-log` above) against the last known-good
   log to see which repository's head moved.
3. Open an issue against the repository whose head introduced the break. The
   exact-pin lane's pinned revisions are the source of truth for what QSL
   ships, and are never advanced solely because this lane failed.
