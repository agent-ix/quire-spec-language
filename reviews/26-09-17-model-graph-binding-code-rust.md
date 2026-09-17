---
id: SR-456
title: "Code and Rust review of model graph binding"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#120; FR-150; TC-195; src/model; tests/model_normalization.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-150
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-195
    type: reviews
---

## Summary

`/code-review` dispatched the QSL #120 Rust delta through `/rust-review`. The
delta implements FR-150 phases 1 (decode), 2 (qualify), 3 (inherit) and 5
(canonicalize) as a pure engine over a self-defined producer-interface `1.3.0`
bundle type, verified byte-for-byte against real vendored QSpec ground-truth
vectors (TC-195 N01, N02, N05, N07, N09). FR-151 (conformance/redefinition/
dispatch), FR-152 (systems-model binding) and FR-153 (population/lookup) are
not implemented in this delta; see the Findings and the PR body for the exact
per-row scope.

## Verdict

**CONDITIONAL** — one real defect found during review was fixed and
re-verified before this review was written (see Resolved findings); no
panic, unsafe, ambient registry or silent-substitute finding remains open in
the code this delta adds. The remaining findings are stated scope decisions
carried into the PR body, not defects.

## Resolved findings

| Finding | Resolution |
| --- | --- |
| `Index::build()`/`ancestor_paths()` indexed `FieldMemberRecord.owner` and `GeneralizationRecord.specific`/`general` by identity string with no existence check. A field member naming an undeclared owner was silently dropped from every effective view (a "best-effort" partial substitute the task's exit conditions forbid); a generalization naming an undeclared `general` reached `.expect("every generalization general is a declared object type")` and panicked on caller-supplied `Bundle` data. | Fixed by `validate_references()`, called at the top of `build()`, refusing `Code::DanglingReference`/`unknown-owner`\|`unknown-specific`\|`unknown-general` before any further work; three regression tests added (`a_field_member_naming_an_undeclared_owner_refuses_instead_of_dropping`, `a_generalization_naming_an_undeclared_specific_refuses_instead_of_being_ignored`, `a_generalization_naming_an_undeclared_general_refuses_instead_of_panicking`) in `tests/model_normalization.rs`. |

## Rust review

- Errors: `ModelRefusal { code: Code, cause: &'static str, detail: String }`
  reuses the crate's shared `Code` catalog (`src/diagnostic.rs`) rather than a
  bespoke string-keyed error; `Incomplete`/`Meter`/`Charge` mirror the
  established `crate::value::accounting` shape over this rung's own,
  independent `ModelNormalizationLimitsV1` counter set.
- Panic surface: the only `.expect()` calls in non-test code (`key.rs:311`,
  `normalize.rs:180`) round-trip bytes this module built itself, not caller
  data. The one `.expect()` that *did* sit on caller-reachable data
  (`normalize.rs`, ancestor-key lookup keyed by a `GeneralizationRecord`'s
  `general` identity) is fixed — see Findings.
- Bounded recursion: `ancestor_paths()` walks the generalization graph with an
  explicit stack, not native recursion, capped at `MAX_GENERALIZATION_DEPTH =
  128` (`Code::ResourceExhausted`/`generalization-depth-exceeded`), with a
  distinct real-cycle refusal (`Code::InvalidModelBinding`/
  `specialization-cycle`) checked before the depth cap fires.
- Numeric conversions: the only `as u64` casts (`normalize.rs:135,171`) widen
  a `usize` byte length that is bounded by the same run's own JCS output, not
  a lossy narrowing cast; no `as` crosses a wire/persistence boundary.
- Matching: `referenced_keys`/`validate_references`/the `LimitKind`/
  `ChargePoint` enums all match exhaustively; no catch-all arm maps to a
  fallback value.
- Atomicity: `Meter::charge()` computes every counter's candidate resulting
  value before mutating any counter and checks `work_units` last, matching
  the spec's field-order-priority denial rule; verified against TC-195 N01's
  three boundary sub-cases.
- Docs: every `pub` item in `src/model/{key,bundle,accounting,normalize}.rs`
  carries a `///`/`//!` doc comment; `normalize.rs`'s module doc states the
  two-pass build/charge design tradeoff and the phase-4 scope decision
  explicitly rather than leaving either implicit.
- No new `#[allow(...)]`, no `unsafe`, no async/lock surface in this delta.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| `cargo test --all-features` (full suite) | pass — 99 test-result blocks, 0 failed |
| `cargo test --test model_normalization` | pass — 11/11 |
| `quoin validate --repo . --strict` | pass — no findings |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-151 (conformance/redefinition/dispatch), FR-152 (systems-model binding) and FR-153 (population/lookup/allInstances) are out of scope for this delta; TC-195 N03/N04/N06/N08 and TC-196/197/198 are not covered by any test in this repo yet. This is a stated scope decision (module docs, PR body), not a silent gap, but it means #120's four inventory FRs are only ~1/4 delivered by row count. | resources/complete-value/quire-specification/spec/functional/type-model/FR-151/152/153*.md; tests/model_normalization.rs |
| FND-002 | low | `build()` is unconstrained by `ModelNormalizationLimits` (pass one computes the complete normalization before pass two replays the charge sequence), so pass one's own memory use is not bounded against an adversarial bundle size — documented explicitly in the module doc as a known, accepted-for-this-rung tradeoff rather than a silent gap. | src/model/normalize.rs:21-32 |

FND-001 and FND-002 are scope statements for the PR description, not defects
to fix before merge; see Resolved findings above for the one defect this
review found and fixed in this same delta.
