---
id: SR-12918
title: 'Failure-domain review of STD-191 (PR #1004): source-length scale bound and time-dimension identity'
type: SpecReview
analysis: failure-domain
scope: 'agent-ix/quire-spec-language@b9c479125; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md; spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md; spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md; context: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-257-build-forms-at-any-depth.md, spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; agent-ix/quire-specification@ad2b37e spec/functional/type-model/FR-142; agent-ix/filament-core-data@e777d8d spec/functional/FR-027'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-393
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
---
# SR-12918: failure-domain review of STD-191 (PR #1004)

## Summary

Ticket: STD-191. This review looks for inputs the new rules decide wrongly or leave undecided: what a meaning-neutral edit does to the S2 bound, and how FR-230's time-dimension identification and unit lookup behave when a package declares several candidates, none, or an affine one.

Units examined (each listed in the Linear comment's `scope` with its text): FR-185, FR-185-AC-2, FR-186-AC-4, FR-230, FR-230-AC-1, FR-230-AC-2, FR-230-AC-5, FR-393, FR-393-AC-8, TC-1078, TC-620, TC-621, TC-685. Context only: FR-091, FR-096, FR-255, FR-257, FR-277.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Admission under the source-length bound depends on meaning-neutral bytes. Adding a comment or whitespace can turn a refused `decimal(1, s)` into an admitted one, and deleting a comment can make an admitted unit refuse. The vectors depend on this: FR-185-AC-2 and TC-620 pad the unit 'with a comment' to reach 19,729 bytes. So the outcome of a unit depends on trivia that every other stage ignores, and a formatter pass (FR-003) can change it. This is a further reason to bound the written value at S3 (FR-185) rather than by source length (base FND-001). | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:156-163; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:158 |
| FND-002 | medium | 'The time dimension SHALL be the admitted dimension whose vector is the single base dimension `time` with exponent 1' has two gaps. First, FR-142 makes base dimensions opaque identities ('Equal names ... do not confer identity'), so 'the base dimension `time`' does not say which identity it is. A name match is not one, and FR-094-AC-8 spells it `Time`. Second, FR-142 admits distinct dimension nodes with equal base maps (its torque/energy case), so a package can declare two dimensions over time^1, for example `Time` and `Duration`. 'The' dimension is then undefined, and a unit of either one would be accepted or refused arbitrarily. Fix: state how the base dimension `time` is identified, and refuse (for example `ambiguous_declaration`) when more than one admitted dimension matches. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:103-108 |
| FND-003 | medium | The order of the two missing-name refusals and the type-mismatch is unstated. AC-5's 'package with no time dimension, `time dense unit ms` refuses ... naming `time`' holds only if the dimension check runs before the unit lookup. It also has to beat AC-2's `type-mismatch` when the package declares `ms` in another dimension. If the package declares neither, either name could be reported. Separately, nothing refuses an affine time unit: a unit whose composed offset to the root is nonzero. FR-142 forbids arithmetic on it, and clock resets, constraints and conversions all need that arithmetic. Fix: state the order (time dimension present, then `u` found, then `u` in the time dimension), and refuse an affine time unit at its span. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:111-119,199 |

## Verdict

Three findings. FND-001 strengthens base FND-001. FND-002 and FND-003 are real gaps in FR-230's new lookup and should be fixed with it.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | AC-5's 'two base dimensions named `Time` refuse `conflicting_declaration`' is stated for a graph declared in the unit's own source, but FR-091's assembler refuses a duplicate dimension name first, with `ambiguous_declaration`/`ambiguous-name` (FR-091 Catalog codes). S3's `conflicting_declaration` is therefore unreachable from source. Two `Time` bases can only arise from the domain package or imports, which do not exist yet. Fix: state AC-5's duplicate vector as FR-091's `ambiguous_declaration`/`ambiguous-name` at assembly, and keep `conflicting_declaration` only for a source/package pair, waiting on STD-192. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:126-128,220; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:483,864 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed at b3a8e6c42. |
| FND-002 | fixed | Fixed at b3a8e6c42. |
| FND-003 | fixed | Fixed at b3a8e6c42. |
| FND-004 | fixed | Round 2 at db8ec3662: AC-5 and TC-685 step 5 now expect FR-091's `ambiguous_declaration`/`ambiguous-name` at assembly; `conflicting_declaration` is kept only for source vs domain package and waits on STD-192. |
