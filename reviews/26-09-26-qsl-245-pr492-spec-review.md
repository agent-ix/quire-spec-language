---
id: SR-747
title: "PR 492 spec review (QSL-245 remainder)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@caa1520a4393c132583accda17aa9f8c01c14949; diff 2df75ab6...caa1520a; FR-096 (lines 129-134, AC-15, Status 437-446, 483-486); TC-428 step 6; spec/tests.md TC-428 row; spec/spec.md FR-096 row; ADR-013; ADR-012"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-428
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#492. Spec review of the FR-096 edits, TC-428 step 6 and the spec.md and tests.md rows. Read against FR-096 on origin/main (2df75ab6) and the code at caa1520a.

Checked and clean:

- **AC-15 and TC-428 step 6.** AC-15 is backed by the new test, and the Behavior line at FR-096:306 ("SHALL become an `InternalFault`") now holds in code.
- **Removed "not built" lines.** Removing the Not built bullet and the QSL-282 "conversion remains unbuilt" sentence is honest.
- **Package-checking bullet (Status 437-440).** It is honest about the mechanism: `PackageDeclarations::check` returns `CheckRefusal`s whose `CheckCause::code()` is `stage_limit_exceeded`, and `qsl-replay/src/spine.rs:936` takes the region from `DeclarationRegions::refusal_region`.
- **spec.md and tests.md.** The FR-096 row (AC-2 to AC-15) and the TC-428 row (AC-15 added, steps 1 to 6) match the code.
- **Id collisions.** FR-096-AC-15 and TC-428 step 6 collide with no open PR: #491 and #493 add neither id.
- **Untouched ranges.** No FR-100 to FR-111 file, TC-450 to TC-469 file or `.github/workflows` file is touched.

## Verdict

**Approve with changes.** Two medium findings: a normative contradiction and a missing locus row. One low wording finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two SHALL statements now disagree. The paragraph's first sentence says every ceiling of a stage's own limits type in S2 to S4 "and a family `check`" SHALL be reported as `LimitExceeded`. The added sentence says a `CheckingLimits` stop during package checking is a `CheckRefusal`. `CheckingLimits` is S3's own limits type, so the same ceiling is required to be both. Qualify the first sentence, for example by exempting package checking or by saying "reported as `stage_limit_exceeded`". | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:129-134 |
| FND-002 | medium | The locus table has no row for a lowering work-budget stop. The new test pins that stop at a node: `if a then b else c + d` for the first lowering charge and `c + d` at path `[2]` for a later one. Row 162 ("`Typer` and lowering") lists only nesting depth and node count. Row 163 (package checking, "the package's work") says the locus is the declaration being charged. So the new sentence "carrying the locus this FR gives its row" is false for this stop. Either add lowering work to row 162, which matches the code, or locate it at the declaration. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:131-133, 162-163; qsl-semantics/src/check/region.rs:478-510 |
| FND-003 | low | AC-15 names the crate-private `Machine::run` and "the seam" without saying which one. The test observes through `ValueFunctionFamily::evaluate`. Name that observable surface, or the `CallFailure::Fault` of `CheckedPackage::call`/`evaluate`, instead of a private function. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:346 |
