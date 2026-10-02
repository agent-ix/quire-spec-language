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
bundle type, verified byte-for-byte against real QSpec ground-truth
vectors held in this repo (TC-195 N01, N02, N05, N07, N09). FR-151 (conformance/redefinition/
dispatch), FR-152 (systems-model binding) and FR-153 (population/lookup) are
not implemented in this delta; see the Findings and the PR body for the exact
per-row scope.

## Verdict

**PASS (mergeable)** — this review was originally CONDITIONAL. An Opus
re-review re-ran every gate against real bundles and reproduced 13 further
findings (F1-F13, P1-P3) beyond the one defect resolved below; all MUST-FIX
correctness and false-coverage-claim findings are now fixed and verified with
new regression tests (see Resolved findings). The three items that remain
genuinely out of scope for this PR (`ModelRefusal.cause` as an enum,
`ModelRefusal`/`Diagnostic` convergence, the dead `record_keys.sort()`, the
`MAX_GENERALIZATION_DEPTH`/`MAX_CHECKING_DEPTH` duplication, and directly-declared-member-shadows-inherited-member semantics) are filed as
[quire-spec-language#141](https://github.com/agent-ix/quire-spec-language/issues/141)
and noted in the PR body, not silently dropped.

## Resolved findings

| Finding | Resolution |
| --- | --- |
| `Index::build()`/`ancestor_paths()` indexed `FieldMemberRecord.owner` and `GeneralizationRecord.specific`/`general` by identity string with no existence check. A field member naming an undeclared owner was silently dropped from every effective view (a "best-effort" partial substitute the task's exit conditions forbid); a generalization naming an undeclared `general` reached `.expect("every generalization general is a declared object type")` and panicked on caller-supplied `Bundle` data. | Fixed by `validate_references()`, called at the top of `build()`, refusing `Code::DanglingReference`/`unknown-owner`\|`unknown-specific`\|`unknown-general` before any further work; three regression tests added (`a_field_member_naming_an_undeclared_owner_refuses_instead_of_dropping`, `a_generalization_naming_an_undeclared_specific_refuses_instead_of_being_ignored`, `a_generalization_naming_an_undeclared_general_refuses_instead_of_panicking`) in `tests/model_normalization.rs`. |
| **F1 (critical)**: `ancestor_paths()`/`build()`/`normalize()` enumerated every ancestor path *before* consulting `ModelNormalizationLimits`, so a diamond/parallel-generalization bundle produced a path count exponential in chain depth before any charge could deny it. Reproduced: a 43-record bundle (15 types, two parallel generalization records per chain level) took ~51-54s; a 55-record bundle exhausted 36GB and was killed. | `remaining_fact_budget`/`fact_budget_exceeded` (`src/model/normalize.rs`) derive a budget from `limits` itself and are consulted *during* `ancestor_paths`'s DFS (`if out.len() >= budget { break; }`) and during the inherited-member loop, so enumeration is bounded by the meter's own configuration, not only checked after full materialization. Both helpers only ever overestimate remaining capacity, so a legitimately-completable run is never wrongly truncated — proved by the unchanged N01/N02 exact-boundary tests still passing. New regression test `f1_deep_parallel_generalization_bounds_enumeration_instead_of_exploding` reproduces the exact 43-record shape under a saturated budget: it completes in ~0.01s here and asserts a typed `Incomplete`; run standalone against `759968d` it takes 54.4s before failing the same assertion (confirmed manually, not part of this PR's test suite). |
| **F2 (high)**: `Index.types`/`member_preimages` were keyed by display identity string alone, discarding `authority`/`revision`/`digest`. Two `ObjectType` records sharing an identity but differing in revision collapsed to one; two `FieldMember` records collapsed similarly while the meter still charged for both (a silent drop breaking FR-150-AC-8's charge/output correspondence). | `Index.types: BTreeSet<ProducerKey>`, `fields_by_owner`/`generals_by_specific`/`member_preimages` all keyed on the *whole* `ProducerKey` now. New regression tests `f2_producer_keys_sharing_an_identity_but_differing_in_revision_both_survive` and `f2_field_members_sharing_an_identity_but_differing_in_revision_both_survive` fail on `759968d` (collapse to 1/2 respectively) and pass on this head (2/3 respectively). |
| **F4 (high)**: producer interface `1.2.0` got `Code::UnknownWire`/`unsupported-wire`, the same refusal as a genuinely unrecognized version, when TC-195 N08/FR-150-AC-7 specify ten `invalid_model_binding`/`unsupplied-producer-record` refusals, one per missing capability item, in a fixed order. | `decode_check` now lets `1.2.0` through the wire gate; `unsupported_capability_refusals`/`unsupplied_capability_items` implement the fixed ten-item order (subtype-closure, subsetting-closure, redefinition-closure, generalization×types, redefinition×members, subsetting×members, interface-signature×types, typed-multiplicity×members) after three `normalize.record` and ten `normalize.unsupplied-item` charges. New test `n08_interface_1_2_0_refuses_every_missing_capability_in_fixed_order` asserts the exact sequence, cause, and detail text of all ten. |
| **F6 (medium)**: an absent revision (empty `revision.namespace`/`revision.value`) was not checked at all. | `revision_is_absent` + a `decode_check` clause refuse `Code::InvalidModelBinding`/`wrong-model-selection` before any charge; new test `n04_absent_revision_refuses_wrong_model_selection`. |
| **F13**: `ProducerKey::fixture`/`ProducerDigest::of_identity`/`ModelSelection::fixture` were `pub` and derived a digest from a display identity in production builds — exactly the name-derived-identity defect this engine exists to exclude. | Gated behind `#[cfg(any(test, feature = "test-support"))]`; new `test-support` Cargo feature, enabled by `cargo test --all-features` (the gate command already specified for this PR) so `tests/model_normalization.rs` can still reach them. A plain `cargo build`/`cargo build --release` no longer exposes them. |
| **P3**: `.expect("every generalization general is a declared object type")` at the old `normalize.rs:441` was guarded at a distance by `validate_references` in a different function — a future caller of `ancestor_paths`/`build` bypassing `validate_references` would panic on caller data. | `ancestor_paths` now uses `record.general.clone()` directly with no `.expect()`; `validate_references` is called once at the top of `build` and nothing downstream re-derives the same invariant by assumption. |
| **F10**: `preimage_to_json`/`EffectiveView::to_json`/`jcs_len` each independently rebuilt the same `serde_json::Value` and re-serialized it to compute an identity and a byte length separately. | `EffectiveDeclarationPreimage::to_json` is `pub(super)` so `normalize.rs` calls it directly instead of round-tripping through JCS bytes and back; `identity_and_jcs_len()` computes both the identity and the JCS length from one `to_json`/serialize pass, used by both the type and member preimage construction sites in `build()`. |
| **F7**: this review previously claimed "only `as u64` casts (`normalize.rs:135,171`)" — there were six (`:135,171,534,553,609,627`), not two. | All six now go through `crate::value::length_amount(usize) -> u64` (`pub(crate)`, re-exported from `src/value/mod.rs`), the same crate-wide `usize`→`u64` persistence conversion `value/collection.rs`/`value/integer.rs`/`value/text.rs` already use, instead of a bare `as u64`. `grep -n "as u64\|as u32\|as usize" src/model/*.rs` now returns nothing. |

## Rust review

- Errors: `ModelRefusal { code: Code, cause: &'static str, detail: String }`
  reuses the crate's shared `Code` catalog (`src/diagnostic.rs`) rather than a
  bespoke string-keyed error; `Incomplete`/`Meter`/`Charge` mirror the
  established `crate::value::accounting` shape over this rung's own,
  independent `ModelNormalizationLimitsV1` counter set.
- Panic surface: the remaining `.expect()` calls in non-test code
  (`key.rs`'s `jcs_bytes`, `normalize.rs`'s two `.expect("built above")`
  removing a key this same function just inserted) round-trip data this
  module built itself, not caller data. The `.expect()` that *did* sit on
  caller-reachable data (F2's old ancestor-key lookup keyed by a
  `GeneralizationRecord`'s `general` identity, and separately P3's
  guarded-at-a-distance generalization lookup) is fixed — see Resolved
  findings.
- Resource bounds: `ancestor_paths()` walks the generalization graph with an
  explicit stack, not native recursion, capped at `MAX_GENERALIZATION_DEPTH =
  128` (`Code::ResourceExhausted`/`generalization-depth-exceeded`), with a
  distinct real-cycle refusal (`Code::InvalidModelBinding`/
  `specialization-cycle`) checked before the depth cap fires — *and* (F1) is
  additionally bounded during enumeration by a `ModelNormalizationLimits`-
  derived fact budget, so an adversarial diamond/parallel-generalization
  bundle cannot force unbounded work before any charge is consulted.
- Numeric conversions: no bare `as` cast remains in `src/model/*.rs`; every
  `usize`→`u64` site goes through `crate::value::length_amount` (F7).
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

Real measured numbers as of this update's head (not the 99-blocks/0-failed
claim this review and the PR body previously repeated without quoting a full
count):

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| `cargo test --all-features` (full suite) | pass — 99 test-result blocks, **1101 tests passed, 0 failed** |
| `cargo test --test model_normalization --all-features` | pass — **21/21** (was 11/11 before this update; 10 new tests for F1/F2/F4/F5/F6/F12) |
| `quoin validate --repo . --strict` | pass — no findings |

F1 and F2 regressions additionally verified by hand against commit `759968d`
(PR #140's start point, checked out into a throwaway worktree, not part of
this repo's own test suite): both `f2_*_both_survive` tests fail there
(collapse to 1/2 declarations respectively); `f1_deep_parallel_generalization_bounds_enumeration_instead_of_exploding`'s
assertions fail there too, after the run actually takes 54.4s (matching the
review's reported ~51s for this exact 43-record shape) rather than the ~0.01s
it takes on this head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-151 (conformance/redefinition/dispatch), FR-152 (systems-model binding) and FR-153 (population/lookup/allInstances) are out of scope for this delta; TC-195 N06 and TC-196/197/198 are not covered by any test in this repo yet (N03/N04/N08 are now covered — see Resolved findings and SR-457). This is a stated scope decision (module docs, PR body), not a silent gap, but it means #120's four inventory FRs are only ~1/4 delivered by row count. | resources/complete-value/quire-specification/spec/functional/type-model/FR-151/152/153*.md; tests/model_normalization.rs |
| FND-002 | **critical (corrected — was mis-graded "low" and dismissed as an accepted tradeoff)** | This is F1: `build()`'s unbounded enumeration before any `ModelNormalizationLimits` charge is consulted was a real, reproducible resource-exhaustion defect, not a scope statement — a 43-record bundle took ~51-54s and a 55-record bundle exhausted 36GB and was killed. Fixed; see Resolved findings above. Recorded here, at its corrected severity, so this review's own history is honest about having graded it wrong the first time rather than silently rewriting the original entry away. | src/model/normalize.rs (module doc, `remaining_fact_budget`/`fact_budget_exceeded`) |
| FND-003 | deferred | `ModelRefusal.cause: &'static str` should be a typed enum (each cause is a distinct condition a caller must distinguish, not a shared string payload); `ModelRefusal` is a second error surface parallel to `Diagnostic` with no conversion between them; the dead `record_keys.sort()` in `charge_all` (sorted order is never used, only the count); `Code::ResourceExhausted`/`MAX_GENERALIZATION_DEPTH` duplicating `MAX_CHECKING_DEPTH` (`src/value/expression/check.rs:32`) as an independent constant. None of these are correctness defects. Filed as [quire-spec-language#141](https://github.com/agent-ix/quire-spec-language/issues/141), referenced in the PR body. | src/model/normalize.rs; issue #141 |
| FND-004 | deferred | Directly-declared-member-shadows-inherited-member merge semantics (a member declared directly on a type and also inherited along an ancestor path) are not addressed by this delta's phase 2/3 implementation; out of scope for this rung, phase-4 territory. Noted in the PR body, not in issue #141 (it is a modeling-semantics question, not implementation cleanup). | src/model/normalize.rs `build()` |

FND-001 is a scope statement for the PR description, not a defect to fix
before merge. FND-002 was a real defect and is fixed. FND-003/FND-004 are
deferred with a ticket/PR-body reference, not silently dropped.
