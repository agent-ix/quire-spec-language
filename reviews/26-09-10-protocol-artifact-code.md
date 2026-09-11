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

Code, rust-review and rust-style review of the new `protocol_artifact` reader,
encoder and wire records at 51506ed (5,678 added source lines, 1,650 added test
lines; the FR-038 numeric slice was already reviewed at SR-316). The delivered
code is careful: closed object-only Serde with `deny_unknown_fields` and an
explicit `Nullable` newtype that refuses omission, a charge-before-allocate
census on both directions, iterative cycle checks, typed error enums with no
message parsing, and no reachable panic, `unsafe`, `as`-narrowing at the wire
boundary, `unwrap` or `allow(...)` in the module. The problem is evidence: both
positive fixtures are one minimal protocol declaration, so most of the refusal
logic that this module exists to provide never executes.

## Verdict

**FAIL** — FND-001 is high. The delivered, *exercised* reader is sound; the
shipped-but-unexecuted half of it is not verified by anything.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Both fixtures use one protocol declaration, `Check`/`Sequence`/`Parallel` controls, empty roles/relationships/channels/compensations, two types and only `ValueOperation::Boolean`; `Choice`, `Repeat`, `Await`, all five `Event` kinds, `Commit`, `Ordering::Fifo`, `compensations`, `temporal`, the predicate/state/temporal bodies and 17 of 18 value operations never run | tests/support/protocol_artifact/mod.rs:356-395; src/protocol_artifact/validate/control.rs:638; src/protocol_artifact/validate.rs:758 | correct-requirement-no-evidence |
| FND-002 | medium | `read` runs the dependency inventory match before the header check, so a same-shaped `quire.compiled-protocol/2` offer carrying one extra dependency refuses as `Invalid(Inventory)` rather than `Unsupported(Wire)`, against FR-042's "typed discriminating causes" for unknown version | src/protocol_artifact/intake.rs:499-500 | implementation-bug-despite-evidence |
| FND-003 | medium | `compensations` never rejects two compensations naming one `forward_effect`, and a non-null `commit` does not forbid the later registration/recovery the contract prohibits | src/protocol_artifact/validate/control.rs:638; docs/compiled-protocol-v1.md:365 | correct-requirement-no-evidence |
| FND-004 | low | `models::validate` sets `work.locus` per value and never clears it, so an `OutputBytes`/`ByteWork` exhaustion raised later in the canonical pass reports the last inspected value's locus as its "available locus" | src/protocol_artifact/models.rs:194; src/protocol_artifact/intake.rs:505 | correct-requirement-no-evidence |
| FND-005 | low | `Local::Control\|Role\|Channel\|Compensation` share one arm, forcing an `unreachable!()` in library code that a narrower split would make statically impossible | src/protocol_artifact/validate.rs:212 | missing-requirement |

## Rust review lanes

**Idioms and style.** One `thiserror` error type per boundary with `#[non_exhaustive]`
typed classification enums; no `Box<dyn Error>`, `String` errors or `anyhow`.
`///` on every public item, `//!` headers citing FR-038/FR-042, SPDX AGPL-3.0-only
on every file. Ownership at the signature is borrowed-in/owned-out throughout;
`Copy` selection structs avoid clones. Exhaustive `match` over every closed enum —
no catch-all `_` arm absorbs a future variant in the wire types. `Nullable<T>`,
`ExactInteger`/`ExactRational` and `Handle` are exactly the newtypes-state-the-
invariant idiom the repo's style skill asks for.

**Panic and unsafe surface.** `unsafe_code = "forbid"` holds. Every raw index is
preceded by a bounds check on the same path: `local`, `ty`, `binding`,
`dependency` and `find_reference` all return `Invalid::Reference` first; `acyclic`
re-checks `child >= graph.len()`; `compare_edges` uses `checked_mul`. Charging
uses `checked_add` and `saturating_add`. The only `unreachable!` is FND-005.

**Untrusted input and resource bounds.** `deny_unknown_fields` on every payload
record and tagged alternative; struct variants keep the guard on empty tags;
positional-array substitutes are refused through the existing `serde_object`
adapter. Thirteen independent dimensions clamp to hard maxima, honor zero, charge
before allocation, use `try_reserve_exact` for the output buffer, and return a
typed `Exhaustion` naming dimension, prior usage, requested work and limit.
`disable_recursion_limit` is safe only because the census charges `Depth` before
descending — correct, and worth a comment at the call site.

**Seams and test quality.** No `#[cfg(test)]` branch or test-only feature alters
production behavior. Tests reach only the public API, the model is admitted
through the real `model_source::read(...).admit(...)` frontend, and definition and
rule bytes come from the registered immutable resources. Assertions are exact:
`failure()` compares the whole `Error` value, limit tests assert dimension, limit,
usage and a clean fresh retry, and the escaping test asserts a literal byte
prefix. No `is_err`-only assertion, tautology, sleep, `#[ignore]` or
`#[should_panic]` was added.

## Gates

Run serially in one batch under `flock /tmp/quire-heavy-check.lock`, single-job,
`--locked`, `nice -n 10`, shared target dir; `src/lib.rs` mtime touched inside the
lock only. All five passed: clippy `--all-targets --no-default-features -D warnings`
(0), clippy `--all-targets --all-features -D warnings` (0), `cargo test
--no-default-features` (508 passed / 0 failed / 4 ignored, 50 suites including
doctests), `cargo test --all-features` (524 passed / 0 failed / 4 ignored),
`cargo fmt --check` (0). No `deny.toml` exists, so cargo-deny was not run. Logs
are in `/tmp/quire-protocol-artifact-review-*.log`. Green gates confirm the
exercised paths only; they are not evidence for FND-001.
