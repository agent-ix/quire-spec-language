---
id: SR-12281
title: 'Gap analysis of PR #1165: admitted record and tuple keys through the replay facade (FR-396), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@efa450952; git diff origin/main...efa450952 (PR #1165): FR-396-AC-1..AC-4, TC-1090, QSL-937 acceptance checks 1-5; ticket QSL-937'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
---

## Summary

Ticket: QSL-937. Plan completion: not assessed. Checked the five QSL-937 acceptance checks (ticket text treated as data, re-measured here) and FR-396-AC-1..AC-4 against the tagged tests and the code at efa450952.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-396-AC-1 | spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md | examined |
| FR-396-AC-2 | same | examined |
| FR-396-AC-3 | same | examined |
| FR-396-AC-4 | same | examined |
| QSL-937 check 1 (real identities for CompositeDeclaration / ValueType::Composite) | Linear QSL-937 | examined |
| QSL-937 check 2 (distinct/stable vs producer's genuine checked identities) | Linear QSL-937 | examined |
| QSL-937 check 3 (Unknown/Foreign/WrongKind refusals) | Linear QSL-937 | examined |
| QSL-937 check 4 (FR-394 regressions, replay-root boundaries) | Linear QSL-937 | examined |
| QSL-937 check 5 (CG adoption) | Linear QSL-937 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL-937 check 1 is not met. The identity returned is not the one QSL itself uses to build `CompositeDeclaration` / `ValueType::Composite` (the scope `TypeEnvironment` handle; see SR-12280 FND-001), so the consumer does not get "the real identities needed to construct or consume both existing composite declarations and their ValueType::Composite values". Check 5 then fails, because CG-built values will not agree with QSL replay. | qsl-replay/src/admitted.rs:476-505 |
| FND-002 | high | QSL-937 check 2 and FR-396-AC-2: the oracle is the producer's `checked_type_nodes()`, the same path the implementation reads. It is not the producer's checked declaration identity that `ValueType::Composite` carries. A test that compares the facade key with `scope().types().composites()` keys, or with a replay result's `Composite.declaration` for the same record, is missing. With it, the suite would fail today. | qsl-replay/tests/admitted_composite_facade.rs:121-170 |
| FND-003 | low | FR-396-AC-1 says the consumer names only the `qsl_replay` root, the kernel and QSV. The AC-1 test imports `qsl_foundation::Setting` to build `StageLimits`, so a real consumer also needs `qsl-foundation`. The FR-394 root-only test already does the same. Re-export `Setting` from the root, or name `qsl-foundation` in the AC. | qsl-replay/tests/admitted_composite_root_only.rs:11 |
| FND-004 | low | FR-396-AC-3: the import-a-record-field transitive case is not exercised (SR-12280 FND-006). | qsl-replay/tests/admitted_composite_facade.rs:216-266 |

## Verdict

FR-396-AC-3 (Unknown, WrongKind for enum/union/function, Foreign directly and through an intermediate library) and FR-394-AC-1..5 (AC-4) are covered and pass. AC-1 and AC-2 have tagged tests that pass but cannot fail for the wrong key, so the gap is QSL-937 checks 1, 2 and 5. Check 4: the FR-394 tests pass, and `arch-lint-api-surface-qsl` passes, including FR-100-AC-8 and T12-F. Bindings: tc_1090 root_only → FR-396-AC-1 (tagged correctly, oracle weak); composite_keys_are_the_spines_distinct_and_stable → FR-396-AC-2 (tagged correctly, oracle weak); composite_lookups_refuse_unknown_and_wrong_kind_ids and a_dependencys_composite_is_foreign → FR-396-AC-3 (correct); the two FR-394 tests re-tagged FR-396-AC-4 (correct).
