---
id: SR-331
title: "Code and Rust review of the compiled protocol reader and encoder"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/; tests/protocol_artifact.rs; tests/support/protocol_artifact/mod.rs; src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Code, rust-review and rust-style recheck of the `protocol_artifact` reader,
encoder and wire records at 23a5892, against the original review at 51506ed. The
corrections (08e401c, c531056, 23a5892) add 1,350 test lines and 102 source
lines. Three of the five original findings are fixed with regression tests, one
is withdrawn on shared-requirement evidence, and the evidence finding drops from
high to medium: the fixtures now exercise all four declaration bodies, all eight
control operations and the binding-kind families, while channels, send/receive
and compensation records remain shipped but unexecuted.

## Verdict

**CONDITIONAL** — no high finding. The exercised reader is sound and now covers
most of the refusal logic the module exists to provide; a named residual surface
is still unexecuted.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Residual unexercised surface: no fixture populates `channels` or `compensations`, so `Event::Send`/`Receive`, `Ordering::Fifo`, channel message types and delivery intervals, and the compensation registration/attempt/retry/recovery records never run; seven of eighteen value operations (`Enum`, `Pre`, `Size`, `Contains`, `Query`, `Parent`, `Reaches`) are likewise never constructed | tests/support/protocol_artifact/mod.rs:1343-1344; src/protocol_artifact/validate/control.rs:171; src/protocol_artifact/validate/control.rs:705 | correct-requirement-no-evidence |
| FND-002 | low | The self-`Origin::Selected` allowance is unconditional, so the reader admits a provenance marker on a leaf or single-origin value that the solver cannot emit (it produces `Selected` only where two child origins differ) | src/protocol_artifact/validate.rs:774-781; src/checking/composed/solver/origins.rs:126-136 | missing-requirement |
| FND-003 | low | `Locus` is a two-`u32` record cloned per declaration and per value into `work.locus` on the hot validation path; the `record!` macro derives only `Clone`, so the copy is structural rather than intentional | src/protocol_artifact/models.rs:193-197; src/protocol_artifact/wire.rs:207 | missing-requirement |

## Resolved since the original review

**Intake precedence (was medium).** `read` now calls `headers` before `selected`,
so a same-shaped `quire.compiled-protocol/2` offer carrying one extra dependency
refuses as `Unsupported(Wire)` rather than `Invalid(Inventory)`. The contract
publishes the precedence rule and
`unknown_wire_is_classified_before_its_foreign_dependency_inventory` asserts it.

**Stale canonical locus (was low).** `encoding::bytes` and `encode_candidate` now
clear `work.locus` before the package-wide pass, so an `OutputBytes`/`ByteWork`
exhaustion in canonical output no longer reports the last inspected value's
region. Source-owned validation keeps its actual locus (`models::validate` sets it
per declaration and per value). The one-short limit test asserts both
`refused.locus()` and `exhaustion.locus` are `None` for `OutputBytes`.

**Unreachable in library code (was low).** The local-table lookup is now one
exhaustive `match (kind, &declaration.body)`; the shared
`Control|Role|Channel|Compensation` arm and its `unreachable!()` are gone, and a
protocol-only handle against a predicate/state/temporal body returns
`Invalid::Owner` structurally. `src/protocol_artifact/` now contains no `unwrap`,
`expect`, `panic!`, `unreachable!`, `todo!`, `dbg!`, `unsafe` or `allow(...)`.

## Withdrawn

**Compensation `forward_effect` uniqueness and commit/recovery exclusion (was
medium).** The finding invented a global static uniqueness rule that the accepted
shared requirements do not state. `quire-specification` FR-056 registers "exactly
the compensation obligations paired with that effect" — multiple authored
obligations per effect are admitted, and FR-056-AC-3's uniqueness is per replayed
runtime effect identity. FR-057 and `proposals/quire-v1/protocol-contract.md:98-100`
make commit ordering a check against observations, and do not ban declaring a
recovery relation alongside a commit. The contract now states both explicitly.
Withdrawn; the runtime obligations stay with FR-056/FR-057 and the full family
checks stay producer obligations under FR-042 plus the shared FR-050–059.

## Rust review lanes

**Idioms and style.** Unchanged and still conformant: one `thiserror` type per
boundary with `#[non_exhaustive]` typed classification enums, no `Box<dyn Error>`,
`String` errors or `anyhow`; `///` on every public item, `//!` headers citing
FR-038/FR-042, SPDX AGPL-3.0-only on every file; borrowed-in/owned-out signatures
with `Copy` selection structs; exhaustive `match` over every closed enum with no
catch-all absorbing a future variant. `models::validate` now takes
`&[AdmittedModel]` and `&[SuppliedDependency]` rather than the whole `Expected`,
which narrows the borrow without creating a swap hazard (the slice types differ).
`timeout_authority` is idiomatic — `matches!`, a `let ... else` on the subject,
`continue` for non-candidates — and charges `visit()` per element rather than
using `contains`, which keeps the scan metered.

**Panic and unsafe surface.** `unsafe_code = "forbid"` holds. Every raw index is
still preceded by a bounds check on the same path; `acyclic` re-checks
`child >= graph.len()`; `compare_edges` uses `checked_mul`; charging uses
`checked_add`/`saturating_add`. The last `unreachable!` is gone. `timeout_authority`
indexes `declarations[owner]` with an owner already validated by the caller loop.

**Untrusted input and resource bounds.** Unchanged: `deny_unknown_fields` on every
payload record and tagged alternative, struct variants guarded on empty tags,
positional-array substitutes refused through `serde_object`. Thirteen dimensions
clamp to hard maxima, honor zero, charge before allocation, use
`try_reserve_exact`, and return a typed `Exhaustion` naming dimension, prior usage,
requested work and limit. The `disable_recursion_limit`/`Depth`-charge coupling now
carries the comment the original review asked for.

**Seams and test quality.** No `#[cfg(test)]` branch or test-only feature alters
production behavior. Tests reach only the public API; the model is still admitted
through the real `model_source::read(...).admit(...)` frontend and definition/rule
bytes still come from registered immutable resources. The new fixtures are
hand-authored wire packages over real source text — which is the honest shape for
a reader test and also the independent oracle for the structural causal-edge
expansion: the fixture lists 33 edges by hand and the reader must derive exactly
those, with the `RepeatProgress` maximum mutation proving the comparison bites.
Assertions stay exact (`failure()` compares the whole `Error` value); the new
suites are strongly one-axis, 18 typed mutations across the two new tests each
naming a single expected discriminant. No `is_err`-only assertion, tautology,
sleep, `#[ignore]` or `#[should_panic]` was added. The four repo-wide `#[ignore]`s
are the pre-existing IT-004 and LC04 lanes, each with a stated reason.

## Gates

Root-supplied frozen-tree results, serialized under `flock /tmp/quire-heavy-check.lock`
with `CARGO_BUILD_JOBS=1`, `CARGO_TARGET_DIR=/tmp/formalization-a-language-target`,
`nice -n10`, `--locked` and one test thread; source lib mtime touched inside the
lock. All green, verified by reading the logs rather than assuming:

| Gate | Result | Log |
| --- | --- | --- |
| clippy `--all-targets --no-default-features -D warnings` | 0 warnings | `clippy-minimal` |
| clippy `--all-targets --all-features -D warnings` | 0 warnings | `clippy-all` |
| `cargo test --no-default-features` | 512 passed / 0 failed / 4 ignored, 50 suites | `test-minimal` |
| `cargo test --all-features` | 528 passed / 0 failed / 4 ignored, 50 suites | `test-all` |
| `cargo fmt --check` | clean (empty) | `fmt` |
| focused `protocol_artifact` + `protocol_number` | 24 + 9 passed / 0 failed | `focused` |

Logs are `/tmp/quire-artifact-corrections-*.log`. No `deny.toml` exists, so
cargo-deny was not run. No gate was re-run for this recheck: every open finding
was resolvable by reading source, spec and the frozen logs. Green gates confirm
the exercised paths only; they are not evidence against FND-001.
