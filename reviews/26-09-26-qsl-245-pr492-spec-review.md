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

## Dispositions

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.24.1 module=spec-artifacts-process@v0.26.0 id=SR-747 pr=quire-spec-language#492 reviewed=d5cf7b9487eec13c3f08d469cb239a582b54796d base=caa1520a4393c132583accda17aa9f8c01c14949 date=2026-09-26 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open (narrowed) | The d5cf7b94 rewording fixes the contradiction for `Typer` and lowering. It keeps it for the declaration input-bytes ceiling, through a new table row whose producer does not exist in code |
| FND-002 | fixed | d5cf7b94: new locus row for lowering's own work charge (node-located), plus FR-096-AC-16 |
| FND-003 | fixed | d5cf7b94: AC-15 now names `qsl_semantics::check::ValueFunctionFamily::evaluate` |

**FND-001, still open (latest outcome at d5cf7b94).** The reworded paragraph says two things about the same stop:

- A family `check`'s own per-declaration precheck (the table row "S3, a family `check`, for a declaration as a whole": preimage input bytes, node count, work charge) "SHALL be reported as `LimitExceeded`".
- "package checking's own declaration-level precheck (`PackageDeclarations::check`)" is a `CheckRefusal`, "never a standalone `LimitExceeded` value". The new table row "S3, package checking's declaration-level precheck" gives it "a declaration's input bytes".

In code these are one stop. `limits.input_bytes()` has exactly one production reader, check/mod.rs:848, which feeds `contract_limits.input_bytes`. It is checked only by the family's `cx.check_input_bytes` (check/family.rs:1469), which returns `StageFailure::Limit(LimitExceeded)` located at the declaration. `PackageDeclarations::check` then converts that `LimitExceeded` into a `CheckRefusal` whose `StageLimitCause` keeps the region (check/mod.rs:914-935). qsl-replay/src/spine.rs:936 takes the region from `refusal_region` on that `CheckRefusal`. So the input-bytes ceiling is still required to be both `LimitExceeded` and never a standalone `LimitExceeded`, and the table has two rows for one producer. The d5cf7b94 region-test doc says the same thing ("node count, input bytes and work budget are the family's own per-declaration precheck"), so the spec now disagrees with the code's own doc.

Suggested fix, one of:
- Delete the "package checking's declaration-level precheck" row and clause. State that `PackageDeclarations::check` re-reports the family precheck's `LimitExceeded` as a `CheckRefusal`/`stage_limit_exceeded` whose `StageLimitCause` keeps the declaration region.
- Scope the `LimitExceeded` SHALL explicitly to the `ValueFunctionFamily::check` seam.

The loci agree in both readings (the declaration span), so no behaviour is ambiguous. This is spec text only.

**FND-002, verified.** In the new row, lowering's work stop is located at the node, and AC-16 matches the test: `region: None`, the body root at `declared`, and `c + d` at path `[2]` at the derived budget. **Id check:** FR-096-AC-16 and FR-096-AC-17 appear nowhere on origin/main or in open PR #493 (`task/293-embedded-locations`). No Linear issue body or comment on QSL-160, 239, 281, 282, 292, 293 or 294 names them.

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: still-open
    reason: "Typer/lowering contradiction resolved, but the declaration input-bytes ceiling is still required to be both LimitExceeded (family per-declaration precheck clause) and never a standalone LimitExceeded (package checking's declaration-level precheck clause + new row). In code there is one producer: family.rs:1469 check_input_bytes -> LimitExceeded, wrapped into CheckRefusal by PackageDeclarations::check at check/mod.rs:914-935. Remove the phantom row/clause and state the wrapping, or scope the LimitExceeded SHALL to the ValueFunctionFamily::check seam."
    excerpt_at_head: |-
      Every ceiling of a stage's own limits type in compiler stages S2 to S4, the
      I2 reader and a family `check`'s own per-declaration precheck (the row
      "S3, a family `check`, for a declaration as a whole" below) is a stage
      limit and SHALL be reported as `LimitExceeded`, never as `resource_exhausted`.
      A package-level `CheckingLimits` stop -- `Typer`'s nesting depth and
      package-wide node count, lowering's own work charge, and package checking's
      own declaration-level precheck (`PackageDeclarations::check`, the other S3
      rows below) -- is a stage limit by the same rule, but is reported as a
      `CheckRefusal` ... never a standalone `LimitExceeded` value
      | S3, package checking's declaration-level precheck, under `CheckingLimits` | a declaration's input bytes (NFR-011) | `Locus::Region` over the declaration being charged |
  - fnd: FND-002
    outcome: fixed
    fix_sha: d5cf7b94
    after_excerpt: |-
      | S3, lowering's own work charge, under `CheckingLimits` (NFR-011) | the shared work meter lowering charges per node past a declaration's own precheck | `Locus::Region` over the node whose lowering charge crossed the bound, resolved from its `check::Location` |
      | FR-096-AC-16 | A lowering work-budget stop -- the shared work meter denying a per-node charge past a declaration's own precheck -- is a `CheckRefusal`/`stage_limit_exceeded` with kind work budget, `region: None` on its `StageLimitCause`, and `DeclarationRegions::refusal_region` resolving to the specific node whose lowering charge crossed the bound, not the declaration span. | Test (TC-427) |
  - fnd: FND-003
    outcome: fixed
    fix_sha: d5cf7b94
    after_excerpt: |-
      | FR-096-AC-15 | An S6a evaluation of `not x` for `x: Boolean`, called through `qsl_semantics::check::ValueFunctionFamily::evaluate` with an Integer argument that admission would have refused, stops on a kernel `CheckedInvariant`. It returns `Err(InternalFault)` naming stage `S6a` and invariant `checked-program-invariant` ...
```

+++



### Joint disposition-pass verdict

<!-- reviewer-verdict repo=agent-ix/quire-spec-language visibility=public id=SR-745,SR-746,SR-747 pr=quire-spec-language#492 reviewed=d5cf7b9487eec13c3f08d469cb239a582b54796d base=caa1520a4393c132583accda17aa9f8c01c14949 date=2026-09-26 -->

**Disposition pass, PR #492 at d5cf7b94: changes requested.** One spec-text item is still open. Everything else is fixed and verified.

| SR | FND | Sev | Outcome |
| --- | --- | --- | --- |
| SR-745 | FND-001 | low | fixed d5cf7b94 |
| SR-745 | FND-002 | low | fixed d5cf7b94 |
| SR-745 | FND-003 | low | fixed d5cf7b94 |
| SR-745 | FND-004 | low | fixed d5cf7b94 |
| SR-746 | FND-001 | medium | fixed d5cf7b94 (reviewer mutation-proved) |
| SR-746 | FND-002 | low | fixed d5cf7b94 |
| SR-746 | FND-003 | low | fixed d5cf7b94 |
| SR-746 | FND-004 | low | fixed d5cf7b94 |
| SR-747 | FND-001 | medium | **still-open (narrowed)**. The input-bytes ceiling is still both `LimitExceeded` and never a standalone `LimitExceeded`, through the new "package checking's declaration-level precheck" row, which has no producer in code |
| SR-747 | FND-002 | medium | fixed d5cf7b94 |
| SR-747 | FND-003 | low | fixed d5cf7b94 |

The fix round introduced no other new issue. The only code changes are docs, one private fn extraction and tests. `budget_located_at_c_plus_d` is a genuine derivation. FR-096-AC-16 and FR-096-AC-17 collide with nothing. What is left is a one-paragraph FR-096 edit (see the SR-747 dispositions comment). The coder also needs to append the `## Dispositions` sections to the three SR files.


