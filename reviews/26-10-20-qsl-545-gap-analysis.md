---
id: SR-13461
title: 'Gap analysis of PR #1093: FR-138, FR-140-AC-4, FR-141, FR-142, FR-145 history and hidden-field coverage'
type: SpecReview
scope: 'agent-ix/quire-spec-language@498b438cf; git diff refs/remotes/origin/main...498b438cf (PR #1093, branch qsl-545-history); specs FR-135..FR-146, ADR-020 §4, TC-543/545/546/547/549/550/551/554/555; QSpec FR-377/FR-378 at ad2b37e; ticket QSL-545'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-138
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-141
  type: reviews
---

## Summary

Ticket: QSL-545. PR: quire-spec-language#1093, head 498b438cf. Method: gap-analysis, planless. Suites green (see SR-13460). Mutation probes run against `cargo test -p qsl-replay -p qsl-analyze --test it refinement`, each reverted after. Killed: candidate union limited to the first member (M01), first completion only (M02), history failures ignored (M06), duplicate update admitted (M07), literal membership skipped (M08), unbounded type admitted (M09), the missing-name branch removed (M10), out-of-type admitted (M11), check_initial comparing the full state (M13), the FR-146 hook removed (M14), `pre()` reading the post-state (M16). Survived: writing the update to every object of the type (M03), dropping the history part of the product key (M04, M17), dropping the candidates part (M05), an `any` stutter with hidden fields dropping the set (M12), and resolving the step only for abstract rows (M15). The coder's five spec gaps are judged below: gaps 1, 2, 4 and 5 are spec follow-ups, and gap 3 is a code bug (SR-13460 FND-003).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-138-AC-1 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-AC-2 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-AC-3 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-AC-4 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138-AC-5 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138 Layer 5 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-138 S3 | spec/functional/FR-138-check-and-compute-history-fields.md | examined |
| FR-135-AC-5 | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-140-AC-4 | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-141-AC-4 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-6 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-10 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-11 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-142-AC-4 | spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md | examined |
| FR-142-AC-5 | spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md | examined |
| FR-142-AC-8 | spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md | examined |
| FR-145-AC-1 | spec/functional/FR-145-replay-a-refinement-counterexample.md | examined |
| FR-145-AC-5 | spec/functional/FR-145-replay-a-refinement-counterexample.md | examined |
| FR-146-AC-2 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| ADR-020 AX-2, AX-3 | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md | context_only |
| QSpec FR-378 | QSpec spec/functional/analysis/FR-378-check-refinement-auxiliary-state.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Gap 1, spec follow-up: FR-138-AC-5's `1 / x` does not type in a refinement row (integer `/` needs a Rational context, and a row selects no division law). The fixture's `(1 mod x) + 1` keeps the intent (1 at x = 1, division-by-zero at x = 0). FR-138-AC-5, FR-141-AC-6, FR-142-AC-8 and TC-554 should be respelled. | spec/functional/FR-138-check-and-compute-history-fields.md:131; tests/support/refinement_aux.rs:581-586 |
| FND-002 | medium | Gap 2, spec follow-up: FR-138 S3 admits a history field only through `pre(...)` in an update, but does not say whether `self.last` outside `pre` is refused or what it reads. The code admits it and reads the pre-state value (history.rs module doc); FR-138 should rule on it. | spec/functional/FR-138-check-and-compute-history-fields.md:77-81; qsl-replay/src/refinement/history.rs:15-16 |
| FND-003 | low | Gap 4, spec follow-up: FR-138-AC-2 (missing-name) against FR-363-AC-2 (an unknown field is type-mismatch). The unit-wide name list in `Scope::history_names` also turns an unrelated undeclared field of another type with the same name into missing-name. FR-138/FR-363 should state the rule. | qsl-semantics/src/check/state_model.rs:287-290 |
| FND-004 | medium | Gap 5, spec follow-up: FR-135 still lists `history-row` as a feature this build does not check, and FR-135-AC-5's history half can no longer run. The PR deleted its only tagged test, so FR-135-AC-5 has no test at all, even though its protocol-abstract-side half is still tested by an untagged TC-540 test. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:177,223; qsl-semantics/tests/it/refinement_declarations.rs:1273 |
| FND-005 | medium | FR-138 Layer 5 "write the value to the receiver's history field only. Every other history value SHALL be kept" and the created/deleted-object rule have no test: every fixture has one object and creates or deletes none, and mutation M03 (writing every object of the type) survives. | qsl-replay/src/refinement/history.rs:128-154, 299-301 |
| FND-006 | low | No test pins the history or candidate parts of the product key, or the hidden-field `any` stutter path, or step resolution for a non-abstract row with an update: M04/M17, M05, M12 and M15 survive, because each fixture's mapped state already determines its history and candidates. | qsl-analyze/src/model_check/refine.rs:433-439; qsl-replay/src/refinement/step.rs:397-407, 288 |
| FND-007 | low | QSpec FR-378 orders the product key's history values "in history-row order", while QSL FR-138 and the code sort by object key and field. Equality inside QSL is unaffected, but the two specs disagree on a key that a certificate (FR-149) will compare. | qsl-replay/src/refinement/history.rs:60-80 |

## Verdict

Not mergeable as is, because of SR-13460's two high findings. Coverage of the ticket's ACs is honest: each tagged test asserts its AC's stated outcome, and the tags name the right TCs. FR-138-AC-1..5, FR-140-AC-4, FR-141-AC-4/6/10/11, FR-142-AC-4/5/8, FR-145-AC-1/5 and the FR-146 hook (TC-551 step 2) each have a test that fails under a mutation of the rule it covers. Spec follow-ups FND-001..004 and FND-007 need a spec ticket ruling. FND-005 and FND-006 are test gaps the fix round can close.
