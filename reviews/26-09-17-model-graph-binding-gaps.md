---
id: SR-457
title: "Model graph binding gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "quire-spec-language#120; FR-150/151/152/153; TC-195/196/197/198; src/model; tests/model_normalization.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-150
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-151
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-152
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-153
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-195
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-196
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-197
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-198
    type: reviews
---

## Summary

`/gap-analysis` finds QSL #120 (issue's four normative FRs: FR-150-153; four
test cases: TC-195-198) partially delivered. FR-150 (preserve original and
effective model declarations, closed lookup key material) is implemented for
phases 1/2/3/5 and verified against real TC-195 ground-truth vectors. FR-151
(conformance/redefinition/dispatch), FR-152 (systems-model structures) and
FR-153 (population/lookup/allInstances) are not implemented; TC-196, TC-197
and TC-198 have no executing test in this repo.

## Verdict

**CONDITIONAL** — the delivered slice (FR-150 phases 1/2/3/5) has no
implementation, test or traceability gap of its own; the undelivered slice
(FR-151/152/153, most of TC-195, all of TC-196/197/198) is real remaining
scope, reported here rather than silently left implicit or claimed complete.

## Coverage

FR-150 phases 1 (decode), 2 (qualify), 3 (inherit) and 5 (canonicalize) are
implemented in `src/model/{key,bundle,accounting,normalize}.rs` and traced by
`tests/model_normalization.rs` to TC-195. Verified against real vendored
QSpec ground-truth vectors (`model-effective-declaration-vectors.json`) for
N01 (simple inheritance, exact charge-boundary `Incomplete` cases), N02
(diamond inheritance retaining both paths), N07 (record-order independence).
Two additional TC-195 vectors this delta could reach cheaply within FR-150's
existing shape are also covered: N05 (cross-domain digest substitution
refuses `stale_dependency`/`digest-domain-mismatch`) and N09 (identity-domain
separation: no effective/universe identity ever equals a producer digest).
Three tests found and fixed during review (not part of any TC-195 vector, but
required by the task's own exit conditions) cover dangling `owner`/
`specific`/`general` references refusing instead of silently dropping data or
panicking.

FR-150 phase 4 (`quire.model.normalize.subset/v1`/`quire.model.normalize.redefine/v1`
— explicit subsetting and redefinition, TC-195 N06) is not implemented:
`src/model/bundle.rs`'s `BundleRecord` carries no subsetting or redefinition
record variant, so a bundle describing one is not representable, and
`ChargePoint` has no `normalize.redefinition-check`/`normalize.conflict-check`.
This is stated in `normalize.rs`'s module doc, not a silent gap.

TC-195 N03 (ambiguous member-name checking) and N04's second clause (checking
a `variant` type reference in source) require integrating the model with the
existing expression/type checker, outside this rung's engine. N04's first
clause (a producer record missing its `revision` member) and N08 (producer
interface `1.2.0`'s ten `unsupplied-producer-record` charges) require a wire
decoder this rung does not have — `Bundle` is a typed value the caller
constructs directly, not a wire-format decode target (module doc,
`src/model/bundle.rs`), so "a required member absent from the wire" is not a
constructible input here and is not silently approximated.

FR-151, FR-152 and FR-153 are not started: no `src/model/` file addresses
conformance dispatch resolution (FR-151), systems-model reference binding
(FR-152), or population binding/lookup/allInstances (FR-153). TC-196, TC-197
and TC-198 accordingly have zero executing tests in this repo. This is the
larger remaining share of QSL #120's issue-level scope: by primary-capability
row count (V1-TYPE-021 through -029, V1-EXPR-017 through -022 — 15 rows), the
rows this delta backs are a minority; the PR body enumerates the exact
per-row split.

Native `quoin validate --repo . --strict` reports no repository finding.

## Reverse trace

| Changed behavior | Owning requirement | Executing evidence |
| --- | --- | --- |
| Producer-key identity, comparison order, and `quire.model.*` JCS/SHA-256 identity domains | FR-150 | `tests/model_normalization.rs` (all cases) |
| Phase 2 qualify facts (one per original declaration/owner) | FR-150-AC-1 | `n01_normalizes_f1_to_the_exact_ground_truth_identities` |
| Phase 3 inherit facts, ancestor-path DFS, diamond-path retention | FR-150-AC-4, FR-150-AC-6 | `n02_normalizes_f2_diamond_inheritance_to_the_exact_ground_truth_identities` |
| Record-order independence of the effective view | FR-150-AC-4 | `n07_record_order_does_not_affect_identity_or_view` |
| `ModelNormalizationLimitsV1` exact-bound completion and one-less-unit `Incomplete` at the named charge point | FR-150-AC-8 | `n01_exact_limits_complete_and_the_charge_totals_match_ground_truth`, `n01_one_less_work_unit_is_incomplete_at_the_view_hash` |
| Cross-domain digest substitution refusal | FR-150-AC-3 | `n05_digest_domain_mismatch_refuses_before_any_effective_view` |
| Unsupported producer interface version refuses before any charge | FR-150-AC-2 | `unsupported_interface_version_refuses_before_any_charge` |
| Dangling owner/specific/general references refuse rather than drop or panic | task exit condition 4 (no partial substitute) | three `a_*_refuses_instead_of_*` tests |
| Phase 4 (subsetting/redefinition) | FR-150 (TC-195 N06) | **not implemented** — no `BundleRecord` variant, no charge points |
| Conformance, redefinition resolution, closed most-specific dispatch | FR-151 | **not implemented** — no test, no `src/model/` module |
| Systems-model structure binding | FR-152 | **not implemented** — no test, no `src/model/` module |
| Population binding, `lookup<T>`, `allInstances<T>` | FR-153 | **not implemented** — no test, no `src/model/` module |

Source stubs: 0. Test stubs: 0. Untraced scoped production behavior in the
delivered slice: 0. Caller-mintable ambient registries: 0 (`Bundle` is a
plain value the caller constructs and passes in; nothing in `src/model/` is
reachable except through it, per its module doc).

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| `cargo test --all-features` (full suite) | pass — 99 test-result blocks, 0 failed |
| `cargo test --test model_normalization` | pass — 11/11 |
| Native `quoin validate --repo . --strict` | pass — no findings |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-151 (conformance/redefinition/dispatch) is entirely unimplemented; TC-196 has no executing test. | FR-151; TC-196 |
| FND-002 | medium | FR-152 (systems-model structure binding) is entirely unimplemented; TC-197 has no executing test. | FR-152; TC-197 |
| FND-003 | medium | FR-153 (population binding, `lookup<T>`, `allInstances<T>`) is entirely unimplemented; TC-198 has no executing test. | FR-153; TC-198 |
| FND-004 | low | FR-150 phase 4 (subsetting/redefinition, TC-195 N06) is a stated, representable-input scope decision, not a silent gap: `BundleRecord` has no subsetting/redefinition variant. | FR-150; TC-195 N06 |
| FND-005 | low | TC-195 N03/N04/N08 need either checker integration or a wire decoder this rung does not have; not silently approximated. | TC-195 N03/N04/N08 |
