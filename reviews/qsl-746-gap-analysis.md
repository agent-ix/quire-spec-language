---
id: SR-8729
title: "QSL-746 gap analysis of PR #810"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2ce03c67ef33f41288507b9b5fe411f2042d57ac; base impl/business-model-meanings; qsl-semantics/src/model/normalize.rs; qsl-semantics/tests/it/business_meanings.rs; qsl-semantics/tests/it/model_normalization.rs; qsl-semantics/tests/it/model_systems.rs; spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md; QSpec TC-195 N01, N02, N14-N18; QSpec FR-150-AC-1, FR-150-AC-6, FR-150-AC-8"
review_set: subset
---
## Summary

Ticket: QSL-746. PR #810 at `2ce03c67ef33f41288507b9b5fe411f2042d57ac`. This is a gap analysis scoped to the PR diff. Each ticket acceptance criterion is mapped to a test, and the oracle behind each is judged.

- AC1 (tests-first vector that reproduces n01-member-B-x): `n01_normalizes_f1_to_stable_deterministic_identities` asserts the inherit-only derivation of `(B, A/x)` and its published prefix `42cf1c2c`. The prefixes of A, B, `(A, A/x)`, the view and the universe are also asserted. The red log shows it, and the three other N01 tests and two N02 tests, failing on the old code for the extra qualify fact. Backed.
- AC2 (inherit fact only; the #244 cases follow the same rule): the source change, plus `inherited_business_members_derive_the_inherit_fact_only`, covers N15 record value types, N16 event types, N17 state machines (and the persistence interface with no inherit fact) and N18 `quire` clauses. Each is checked by rule list and inputs `[owner, member]`. N14 (variant types) defines no inherited member, because a variant's `supertypes` list is ignored. The inherit-only rule is vacuous there, so N14 needs no test for this ticket. Backed.
- AC3 (N>0, no new failures, digest uses the published value): `it` has 607 passed, `lib` 518 passed and xtask 182 passed. Using the published 8-hex prefixes is acceptable, because that is TC-195's own notation. Backed.
- AC4 (main): the PR body records that main has the same bug and that the fix reaches main with the final integration PR. Backed.

Trace tags: the new test carries `QSpec-TC-195` and `QSpec-FR-150-AC-1`. The changed tests keep their existing tags. `quire matrix` over the PR tree binds the local FR-081-AC-1 to the changed systems tests. The `QSpec-*` tags name external criteria, so they were checked by reading.

Semantic review was performed: the brief asks for test-oracle strength.

## Verdict

CONDITIONAL. Every ticket AC is backed. One medium coverage gap and one low spec-prose drift remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSpec FR-150-AC-6 requires every effective identity to equal its published vector, including the diamond. This fix changes the N02 member identities `(B, A/x)`, `(C, A/x)` and `(D, A/x)` and the N02 view, but no test compares them with the published `42cf1c2c`, `ea039070`, `36d8ed69` and `e10820a6`. The N02 test checks only that `(D, A/x)` has two facts. It does not check their order (`[B, A, A/x]` before `[C, A, A/x]`), and its doc says it deliberately avoids published digests. A wrong path order or wrong inputs in the diamond would pass. | qsl-semantics/tests/it/model_normalization.rs:777-820 |
| FND-002 | low | NFR-012's default-derivation rationale says "An effective declaration's derivation holds one qualify fact and one inherit fact per ancestor path". An inherited member now holds inherit facts only. The ceiling still holds, but the sentence no longer describes the model. | spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md:87-90 |

## Coverage

- Ticket ACs examined: 4; backed: 4.
- Changed expectations re-derived from the specs: N01 (facts, work units, bytes, JCS length 560, view 2789, three limit boundaries), N02 (15 facts, 46 units, 12112 bytes), N10 mutation index, the N06, operation and interleave fixtures (one unit per inherited member), and the lib bounded-work steps (3,3). One expectation was not derivable as written: see SR-8728 FND-001.
- Reverse gaps: none. The change removes behaviour and adds none.
- Stubs: none.
- Plan completion: not assessed

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4108116476401c3043e4325b24d7f4c3f46cedb6 |
| FND-002 | fixed | 4108116476401c3043e4325b24d7f4c3f46cedb6 |
