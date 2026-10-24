---
id: SR-17016
title: "Integrity review of PR #1453 FR-088-AC-8, TC-260, ADR-016 FE-4 ValueTypeRef membership (QSL-1012 follow-up)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language PR #1453 at 82a57836e; spec/functional/FR-088-clause-name-and-type-identity.md, spec/test-cases/TC-260-value-type-ref-is-exactly-three-member-union.md, spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-056*.md, spec/functional/FR-120*.md, spec/functional/FR-189*.md, spec/functional/FR-230*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-260
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
---

## Summary

Ticket: QSL-1012 (follow-up to #1431). This review checks that the spec stays consistent after `ValueTypeRef::Unit(String)` was added.

What holds:

- The edits match the code at the PR's base 965460aa2: `Native`, `Package` and `Unit` (qsl-semantics/src/model/domain_package.rs:344-351).
- The edits are consistent with #1431's unit typeRef feature. FR-056-AC-42 says a `ix://quire/unit/<name>` typeRef is admitted unresolved at intake and resolved by the assembler. That is the `Unit(String)` payload. FR-120-AC-16 and FR-189-AC-15 use `Quantity<unit>` fields that need it.
- The rename keeps the id. TC-260 is referenced only by id (`ix://.../TC-260`, the FR-088-AC-8 Method cell, and the `#[trace]` attribute). No live file links to the old filename. Only historical records in `reviews/qsl-737-*` and `reviews/qsl-738-*` name it.
- `make check-spec-validation` (`quire validate --scope . spec/**/*.md`) exits 0 at both the base and the head, with the same 18 pre-existing warnings, so there are no new failures.
- The grep for other two-member statements found ADR-013 O-14 (line 434, live, FND-002). It also found `spec/reviews/state-model-mapping/failure-domain.md:34` and `spec/reviews/type-ownership/dependency.md:158`, which are historical review records and should stay as written.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-088-AC-8, the FR-088 O-14 line, TC-260 and ADR-016 FE-4 now say `ValueTypeRef` has exactly three members and "no fourth variant". On current main (1f5903829), `ValueTypeRef` already has a fourth, `Clock`, added by #1451 (9cf7144af) for FR-230's `ix://quire/native/Clock` fields. Once merged, the spec contradicts the code, and so does TC-260. FR-230 also says Clock is "a place in a timed model, not a value type", so whether `Clock` belongs in `ValueTypeRef` is a spec-owner decision. Fix (draco:QSL): either list four members `{Clock, Native(NativeValueType), Package(DeclarationKey), Unit(String)}` in AC-8, O-14, TC-260 and FE-4, or state where Clock fields are typed instead and have the code follow. Then rebase this PR. | spec/functional/FR-088-clause-name-and-type-identity.md:107,246; spec/test-cases/TC-260-value-type-ref-is-exactly-three-member-union.md:13-15,21-25,44-45; spec/decisions/ADR-016-state-model-finite-execution-mapping.md:299 |
| FND-002 | low | ADR-013 O-14 "Public type" still reads "Model field types are `ValueTypeRef{Native(NativeValueType), Package(DeclarationKey)}`". This is the live decision row that FR-088's "Type descriptors (O-14)" restates, and the PR updated that restatement and the ADR-016 FE-4 row, but not this one. ADR-013 is the decision of record, not a historical review. Fix: update the O-14 row to match FR-088-AC-8, or add "membership per FR-088-AC-8". | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:434 |
| FND-003 | low | FR-088-AC-8 now lists `Unit(String)` and also says "no model field type is represented by ... a raw string type name". Read literally, the new member contradicts that clause. The String is a unit name that the assembler resolves (FR-056-AC-42), not a type name, but the AC does not say so. Fix: "...`Unit(String)` (the unit name of a `ix://quire/unit/<name>` typeRef, resolved by the assembler, FR-056-AC-42)". | spec/functional/FR-088-clause-name-and-type-identity.md:246 |

## Verdict

FAIL, not mergeable as is. The wording follows #1431 faithfully and validates cleanly, but #1451 made it stale (FND-001): it asserts "exactly three, no fourth" against a four-member enum on main. Spec-owner (draco:QSL) edit needed for FND-001, which must decide on `Clock`. FND-002 and FND-003 are low and can go in the same edit.
