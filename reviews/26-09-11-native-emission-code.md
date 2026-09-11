---
id: SR-333
title: "Code and Rust review of native protocol emission"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/native/; src/protocol_artifact/mod.rs; tests/native_protocol_emission.rs; tests/support/native_protocol/mod.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Code review of `ae910c0` (6,291 lines over `8e860b5`) using `agent-skills/code-review`,
which dispatched to `agent-skills/rust-review` for the Rust lane, with
`agent-skills/rust-style` as the portable idiom default. The composed
`Source`/`ComposedUnit`/binding/`TypeReport`/`ProofReport` owners do flow through
`native::admit` into a constructor-private `FamilyAdmission`, and eleven tests take
real source through admit, emit and an independent `read`. Three of the eight
control forms the module lowers never execute in any test.

## Verdict

**FAIL** — one high finding. The delivered emitter is sound on every path the suite
executes and the authority boundary holds, but `choice`, `await` and the positive
`repeat` lowering ship with no executing test while the merged wire contract asserts
the choice behavior as implemented.

## What the review confirmed

- Authority: `FamilyAdmission.package` is private, built only in `admit`, and
  `emit` accepts only that type. The `compile_fail,E0308` doctest at
  `src/protocol_artifact/native/mod.rs:92` proves a `wire::Package` cannot be
  substituted; it runs in the doctest lane (5 doctests, all pass).
- No proof-witness IR reaches executable output: `values.rs` lowers from
  `typed.nodes()` and the original arena only, and refuses `Quantifier`,
  `Reaches`, `Size`, `Contains`, `Query` explicitly.
- Locality and identity: declaration/source/`ExprId` handles are declaration-local
  (`layout.rs`), binder identities come from the original `BinderId` rather than
  spelling, and phase anchors (pre/post, activation, FIFO, finish) are separate
  scopes. `layout.rs:481-491` cross-checks each derived evaluation anchor against
  the upstream `ScopeReport`.
- Selection independence: sources, dependencies, contract, baseline, producer and
  models are separately supplied; each source keeps its opaque artifact revision
  apart from its native/formal revision; a registered definition's semantic
  revision comes from `matching_bytes` over the actual registered bytes
  (`metadata.rs:69-137`), not from the source artifact's revision label.
- Charge-before-work and all-or-nothing: `admit` runs one `Work` budget and
  returns no package on exhaustion; `emit` takes a fresh budget. Verified by
  `missing_selected_model_and_independent_limits_cannot_emit_partial_packages`,
  which derives the exact byte ceiling by independent serialization.
- Unsupported prerequisites are explicit: recovery/registration/retry anchors,
  relationships, compensations, absent producer exports and non-constant decisions
  all return `Unsupported::{FamilyProof,Export,Feature}` rather than omitting graph
  content.
- Hygiene: `#![forbid(unsafe_code)]` and `clippy::all = "deny"` hold; no `unwrap`,
  `expect`, `panic!`, `todo!`, `dbg!`, `#[allow]`, `TODO` or stub in the new source;
  SPDX header and `//!` requirement citation on all ten files.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | `tests/native_protocol_emission.rs` is the only caller of `native::admit`/`native::emit` and contains no `choice` and no `await`, and reaches `repeat` only through its `Invalid::Control` refusal. The Choice lowering, the Await lowering (clock requirement, progress/closure bindings, `AwaitAnchor`, `AwaitSuccess`/`AwaitTimeout` edges), the emitted `Repeat` control and its `RepeatProgress` edge, the `Progress::NeedsAuthority` refusal, and every non-literal arm of the closed Boolean interpreter therefore never execute. `docs/compiled-protocol-v1.md` nonetheless states that supported constant decisions establish coverage and non-overlap directly | src/protocol_artifact/native/families.rs:184; src/protocol_artifact/native/families.rs:235; src/protocol_artifact/native/families.rs:312; src/protocol_artifact/native/controls.rs:94; src/protocol_artifact/native/controls.rs:115; src/protocol_artifact/native/controls.rs:140; docs/compiled-protocol-v1.md:502 | correct-requirement-no-evidence |
| FND-002 | medium | Two `pub(super) fn index(value: usize) -> Result<u32, Error>` helpers with the same 1,048,576 bound and divergent bodies: `types.rs` uses `u32::try_from`, `layout.rs` uses `value as u32`. `values.rs`/`families.rs` import one, `metadata.rs` calls the other by path, so which conversion runs depends on the importing module | src/protocol_artifact/native/layout.rs:112; src/protocol_artifact/native/types.rs:467 | missing-requirement |
| FND-003 | low | Bare `as` conversions that bypass the module's own `index()` bound: `controls.rs:79` additionally shadows the imported `index` function with a local binding of the same name before casting `position()` to `u32` (out-of-range values are caught downstream at `validate/control.rs:262-268`, so no wrong output); `layout.rs:84` casts a `position()` result 28 lines above the helper; `layout.rs:469` subtracts two `u32` span endpoints without `checked_sub` | src/protocol_artifact/native/controls.rs:79; src/protocol_artifact/native/layout.rs:84; src/protocol_artifact/native/layout.rs:469 | missing-requirement |
| FND-004 | low | Charge-before-work is applied inconsistently to generated requirement names. `channels.rs:225`, `controls.rs:575` and `runtime.rs:442` charge bytes before `format!`; `runtime.rs:509`, `controls.rs:158` and `controls.rs:180` allocate first and are charged only afterwards inside `text()`. The totals agree, the ordering rule does not | src/protocol_artifact/native/runtime.rs:509; src/protocol_artifact/native/controls.rs:158; src/protocol_artifact/native/controls.rs:180 | missing-requirement |
| FND-005 | low | Style note, no failing scenario. The lexical containment guard at `layout.rs:511-515` cannot fail for `Capture`/`EventRecord` binders, because their scope region is constructed at `layout.rs:400-405` as the hull of exactly the reads the guard then tests. It is non-vacuous only for phase and `let` scopes, whose regions are authored spans. The comment above it correctly states that `ScopeReport` remains the authority, so the delegation is honest; the guard just reads as a check it is not | src/protocol_artifact/native/layout.rs:400; src/protocol_artifact/native/layout.rs:511 | missing-requirement |
| FND-006 | low | Style note, no failing scenario. `DeclLayout::value_scope` is a pass-through to `DeclLayout::scope` with no added behavior, and `metadata.rs:290-297` indexes `sources[..]` and `proofs.bindings()[..]` directly while the rest of the module returns typed errors through `.get().ok_or(..)`. No reachable input reaches either index out of range with a `discharge`-produced report | src/protocol_artifact/native/layout.rs:57; src/protocol_artifact/native/layout.rs:68; src/protocol_artifact/native/metadata.rs:293 | missing-requirement |

## Severity calibration against SR-331

SR-331 recorded the equivalent reader-side gap at `medium` because the wire
fixtures did cover all eight control operations. That defence does not carry over:
those fixtures are manually authored wire records, and this review treats them as
no evidence for the compiler that now produces the same records. On the emitter
side the coverage is zero for three forms, which is why FND-001 is `high`.

## Gates

Root-supplied local logs; the commands themselves are not recorded in the logs.

- `/tmp/quire-native-emission-fmt.log` — empty, clean.
- `/tmp/quire-native-emission-clippy-minimal.log`, `-clippy-all.log` — no diagnostics.
- `/tmp/quire-native-emission-test-minimal.log` — 51 suites, 524 passed, 0 failed.
- `/tmp/quire-native-emission-test-all.log` — 51 suites, 540 passed, 0 failed, 4 ignored
  (3 `fixture_audit`, 1 `native_backend`), including `native_protocol_emission` 11/11 and
  5 doctests with the new `native::emit` compile-fail case. Covers the final source.
- `/tmp/quire-native-emission-focused.log` — `native_protocol_emission` 11/11; predates
  only the Clippy-required collapse of a nested Boolean `if` into the equivalent
  short-circuit condition at `families.rs:196`.
- No `deny.toml` exists, so `cargo deny` does not apply.

No gate was re-run for this review; no unresolved finding required a focused check.
