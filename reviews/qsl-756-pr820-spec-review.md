---
id: SR-9059
title: "Spec review of quire-spec-language PR #820: FR-082 cycle paragraph, FR-082-AC-15 and TC-1020"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@65cc4b55e0c01e7ea2a0031c4a22d60284d61d23; PR #820: spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md (cycle paragraph, FR-082-AC-15), spec/test-cases/TC-1020-a-specialization-cycle-is-refused-once-per-declaration-and-cycle.md, spec/tests.md (TC-1020 row)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1020
    type: reviews
---
# Spec review of quire-spec-language PR #820

## Summary

Ticket: QSL-756. Spec review (base, with integrity and EARS checks) of the new
FR-082 paragraph, FR-082-AC-15 and TC-1020.

The paragraph states the planner's rule completely. A cycle is identified by
its edge set. A declaration is refused once per cycle it lies on. A cycle is
reported at its first closing extension in charge order, and a repeat closing
is charged but not reported. Refusals follow detection order and then
declaration-key order, and the cause carries the declaration and the rotated
path-order keys. This matches the code for object types and processes.
AC-15's three vectors are correct under the rule: the multi-cycle expectation
of four refusals, the 3-cycle's `[B, C, D]` with `A` excluded, and the 2-cycle.
TC-1020 matches AC-15 step for step. The tests.md row (Unit, P1,
FR-082-AC-15) follows the same convention as TC-219. The ids are unique: AC-14
is held by another branch, and TC-1020 is used nowhere else. QSpec is cited
only by id in the PR's trace tags. The PR's spec files and test doc comments
quote no QSpec rule, run outcome or private path. The paragraph's wording is
the exception (FND-002).

## Verdict

Changes requested: one medium oracle-strength finding, one medium
public-repo wording finding and one low EARS finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-082-AC-15 cannot tell path order from key order. Its 3-cycle `B -> C -> D -> B` has path order equal to sorted order, and its 2-cycles are trivially sorted. An implementation that lists the keys sorted, or orders a cycle's refusals by path order instead of declaration key, satisfies every vector, though the paragraph requires path order for the listing and key order for the refusals. Replace or add a vector whose path order is not sorted, for example `B -> D -> C -> B`: refusals at `B`, `C`, `D`, each listing `[B, D, C]`. Mirror it in TC-1020. | spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md:309; spec/test-cases/TC-1020-a-specialization-cycle-is-refused-once-per-declaration-and-cycle.md:19-27 |
| FND-002 | medium | Public-repo rule. The paragraph's opening sentence (line 108) matches a private QSpec definition's sentence word for word, and the rest follows that source clause by clause. QSL needs to state this rule, but in its own words: rewrite the paragraph in QSL terms, citing QSpec by id only if at all. | spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md:108-117 |
| FND-003 | low | The new paragraph is normative but has no SHALL. Every rule in it is phrased as a fact ("A declaration is refused once per cycle..."). Phrase it as a requirement on the model checker, for example "the model checker SHALL refuse each declaration once per cycle it lies on", so the EARS reading is unambiguous. | spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md:108-117 |
