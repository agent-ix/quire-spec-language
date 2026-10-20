---
id: SR-13599
title: "integrity review of the FR-186 and FR-143 spec amendments (PR #1117)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@fd004b67b7c804e521a23e15237462c6d557bb6e; spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md, spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md, spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
---
# SR-13599: integrity review of PR #1117 (QSL-568, QSL-547)

## Summary

Tickets: QSL-568 and QSL-547. This review checks consistency, completeness
and cross-references over `git diff origin/main...fd004b67b` (5 files,
+133/-50). It reads each changed statement against the cited ADR rows
(ADR-013 R-04 and O-09, ADR-018 V-8, PC-1 and PC-3, ADR-020 AX-5, CO-2, CT-4
and roadmap step 4, ADR-024 PM-7 and PM-8), against FR-126, FR-127, FR-144
and FR-160, and against QSpec FR-407 and FR-272. It also reads the code on
`origin/qsl-568-fr186-typing` (20dafcee1) and
`origin/qsl-547-refine-liveness` (38b65f748).

The owner rulings are recorded faithfully where the diff states them, and
they agree with the ADRs. The normal form is declared not an identity
(R-04), the obligation identity stays O-09's CG digest with the occurrence
key, the `over` binder name is excluded, weights, fairness constraints and
reward operations are sets, a strong `F_A` constraint settles `Unsupported`
with the safety half kept, and a holding liveness half settles `proved`
`Uncertified` (CT-4). The issues are in what the rulings leave between
statements. Two FR-143 bullets now contradict each other, and the
hidden-field-first ruling is missing. The holding-half variant is undefined
in the FR it cites. The fault rule cites FR-126 for a behaviour FR-126 does
not state. The strong-`F_A` rule has no ADR anchor. In FR-186, one cause is
used outside QSpec's definition, and the refusals the author left unowned
include one semantic rule that has no statement anywhere.

The #1116 mismatches the coder is fixing (binder name in the bytes, unsorted
weights, `Debug` bytes, TP-2 advice in the fraction message) are not
findings here. Each spec statement they touch can be met by a direct code
change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-143 "When the half runs" has two bullets that contradict each other. Bullet 3: with `F_A` non-empty and a strong constraint, `liveness` SHALL be `Unsupported`, with no condition on the safety half ("The safety half is decided and reported beside it as usual"). Bullet 4: with `F_A` non-empty, no hidden field and a safety half other than `Holds`, `liveness` SHALL be `None`. A refinement with a strong `F_A` constraint, no hidden field and a violated safety half meets both, so one SHALL is broken whatever the code does. The hidden-field case avoids this because bullet 4 excludes it, and the code returns `Unsupported` there whatever the safety half (refine.rs:213-227). Fix: add "and no strong constraint" to bullet 4 (matching the ruling "safety half kept"), or make bullet 3 conditional on a holding safety half. Either way, add the violated-safety case to FR-143-AC-6. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:89-97; qsl-analyze/src/model_check/refine.rs:213-227 (qsl-547-refine-liveness) |
| FND-002 | medium | The ruling "hidden-field cause first when both" is not in the spec. A refinement with a hidden field and a strong `F_A` constraint meets bullet 2, which requires the hidden-field cause, and bullet 3, which requires the strong-fairness cause. `LivenessHalf::Unsupported` holds one cause, so the two SHALLs cannot both hold, and nothing says which wins. No AC covers the case. Fix: bullet 3 says "with no hidden field" or names the precedence, and FR-143-AC-6 adds `RegisterHistory` without `prev` and with `ensure fair strong Spec::Register::write`, which settles the hidden-field cause. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:85-95; spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:151 |
| FND-003 | medium | FR-143 names the holding half `Checked(HoldsReduced{reductions: []})` and calls `ModelCheckOutcome` "FR-126's outcome type". FR-126 "Outputs" defines no `HoldsReduced` variant: its only holding variant is `Holds { basis: Exhaustive, certificate }`. No spec file outside FR-143 and TC-548 mentions `HoldsReduced`. FR-160 extends the map with `ProofBasis::Reduced{reductions}` for "`Holds` with symmetry or partial-order reduction applied". Read by that row, a reduced holding outcome settles basis `Reduced`. FR-143 says only "settles `proved` labelled `Uncertified`" and never names the basis. The code settles `Proved{basis: Exhaustive, certification: Uncertified}` (refinement_settle.rs:101-105, settle.rs:215-222). The spec does not pin this. A settlement that wrote `Reduced{[]}`, a reduced basis with no reduction, would also satisfy FR-143. Fix: FR-143 states that the holding half settles `Proved{basis: Exhaustive, certification: Uncertified}`. FR-126 "Outputs" (or FR-160) also defines the variant and its empty-reductions meaning, so that the ACs' expected value is a specified type. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:57-67; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:118-123; spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md:82-83 |
| FND-004 | medium | The new fault bullet says `check_refinement` returns a refusal "as FR-126's liveness check does". FR-126 does not say this. Its `ModelCheckRefusal` "holds FR-106's `AdmissionFailure` ... or FR-101's `NotSimulated::RequiresBound`", with no internal-fault member, and its liveness refusals are `GuardedActivation` and `ComponentCertificateNotBuilt`. The return type the bullet relies on has no fault member in any spec, so the cross-reference supports nothing. Fix: add the fault member to FR-126's `ModelCheckRefusal`, or state it in FR-143 Outputs, and drop the "as FR-126's liveness check does" comparison. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:137-140; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:113-114 |
| FND-005 | medium | The strong-`F_A` `unsupported` rule has no ADR anchor, and FR-143 attributes it to the wrong row. The Outputs comment reads "AX-5: hidden abstract fields; a strong F_A constraint", but ADR-020 AX-5 covers only liveness through hidden abstract fields. ADR-020's roadmap step 4 says "Strong fairness in `F_C` or `F_A` comes with the strong-fairness feature (ADR-018 FA-5) with no change here". FR-130 already specifies that feature. The owner ruling is sound as an interim limit. FR-143 is now the only record of a V-8 settlement that ADR-020 says will not exist. Fix: record the ruling as an ADR-020 row or amendment (strong `F_A` settles V-8 until phase 2 carries ADR-019's strong filter) and cite it, and correct the Outputs comment. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md:60; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:231; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:538-540 |
| FND-006 | medium | A `fraction` comparison bound outside `(0, 1)`, such as `3/2` or a negative value, is refused `qualitative-threshold`. That cause is in QSpec FR-272's closed set, and QSpec FR-407 defines it as the refusal of a threshold of `0` or `1`, "naming the corresponding every-behaviour or possible-property form". FR-186's own bullet says such a bound "is no qualitative claim", and it forbids the naming that QSpec's definition requires. FR-407 also lists "a fraction threshold" among the thresholds whose `0` or `1` names the qualitative form. So QSL both widens a QSpec-owned cause and drops its content. Fix: use a QSL-proposed cause for an event fraction bound out of range, added to the STD-188 request, or ask QSpec in STD-188 to rule on fraction bounds. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:164-168; quire-specification spec/functional/analysis/FR-407-state-probabilistic-claims.md:119-121 |
| FND-007 | medium | The ambiguous-reward refusal (asked about in the brief) is a finding, because it rests on a semantic rule that no spec states. The code refuses `ambiguous_declaration`/`ambiguous-name` when operations of the subject's model declare reward `R` with different value types (probabilistic.rs `AmbiguousReward`). FR-185 forbids only two rewards of one name on one operation. So a model may declare `reward duration` as `Millis` on one operation and as an integer on another. FR-186 then speaks of "the reward's dimension" and "the reward's unit" as if there were one. Two implementers would read that differently. Fix: an FR-186 (or FR-185) statement that every declaration of a reward name in a model has one value type, and the refusal and cause when it does not, with an AC case. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:152-159; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:184-185; qsl-semantics/src/check/probabilistic.rs:400-410 (qsl-568-fr186-typing) |
| FND-008 | low | The missing-name refusals the author left unowned are a finding, but a low one. FR-186 requires a model subject, a profile alias, `under W` "naming a workload", a reward "declared by at least one operation" and fairness operations. It states no refusal, code or locus for a name that resolves to nothing. The code refuses each with `missing_declaration`/`missing-name` (`UnresolvedSubject`, `UnresolvedProfile`, `UnresolvedWorkload`, `UnresolvedReward`, and `super::model::unresolved` for a fairness operation). Sibling claim FRs state these refusals themselves (FR-165, FR-172, FR-218-AC-10). The behaviour is the obvious one, so this is a gap in ownership, not in meaning. Fix: one bullet naming the five unresolved-name refusals with their locus, and one AC case. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:115-129; qsl-semantics/src/check/probabilistic.rs:314-410 (qsl-568-fr186-typing) |
| FND-009 | low | FR-186's Description lists the normal form as "its subject, profile, scheduler, form, `over` type and the confidence parameters it states" and leaves out the fairness set. The Normal form section and Outputs include the fairness set as content. Fix: add "fairness set" to the Description's list. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:28-30; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:204-207 |
| FND-010 | low | "Subject" as content is ambiguous. `ModelSubjectRef` carries the `model` alias as written, beside the domain package. The code puts the alias into the normal-form bytes (claim.rs:1281-1289), so the same claim over the same domain package under two alias spellings has two normal forms. The ruling's reason for leaving out the `over` binder name (a local name is spelling, not content) applies to the alias too, but the spec says neither way. Fix: state whether the subject's content is the selected domain package (identity, version, digest) or also its alias. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:68; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:208-212; qsl-semantics/src/check/probabilistic/claim.rs:1281-1289 (qsl-568-fr186-typing) |

## Verdict

Not mergeable. FND-001 is a contradiction between two SHALL statements in
FR-143, and FND-002 to FND-007 are medium gaps. Each is a wording or
cross-reference change in this PR, except FND-005, which needs an ADR-020
row. FND-008 to FND-010 are low.

Recorded clean: FR-186 Description and Normal form against ADR-013 R-04 and
O-09 (the normal form is declared not an identity, and the obligation
identity stays CG's digest over the S4 node and the occurrence key). The
binder, weights, fairness and reward-order rules match the rulings. FR-185
"Package emission" and FR-186's S4 bullet agree on `probabilistic-claim`.
FR-186's kind sentence replaces the old classify bullet without loss. FR-143's
holding half against ADR-018 PC-3 and ADR-020 CT-4, and its `Unsupported`
wire cause and code against ADR-018 V-8 and the code's
`unsupported_projection` mapping, are correct. The new causes
`trace-subject`, `unbounded-event` and `not-temporal-profile` are listed with
the other STD-188 causes.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | c524fa192: bullet 3 now applies 'whatever the safety half is' and bullet 4 adds 'no strong constraint'; FR-143-AC-6 adds the violated-safety case. |
| FND-002 | fixed | c524fa192: bullet 3 is conditioned on 'no hidden field', Outputs and ADR-020 AX-6 name the hidden-field cause when both apply, and FR-143-AC-6 adds RegisterHistory with a strong ensure row. |
| FND-003 | fixed | c524fa192: FR-143 Outputs defines the holding half as an empty reduction list and pins its settlement to Proved{Exhaustive, Uncertified}, never Reduced. The FR-160 row it cites is the constraint-only row, which describes a different source of an empty list, but the explicit settlement removes the ambiguity. |
| FND-004 | fixed | c524fa192: the FR-126 comparison is gone; FR-143 states the refusal carries the fault and is not a RefinementOutcome. |
| FND-005 | fixed | c524fa192: ADR-020 gains AX-6 (next in the AX-1..AX-5 sequence, no other AX-6 in the repo), and FR-143's Outputs comment and Dependencies cite AX-5 and AX-6 separately. |
| FND-006 | fixed | c524fa192: a fraction bound outside (0, 1) is now the QSL-proposed cause fraction-bound-out-of-range (STD-188), and qualitative-threshold is restricted to the QSpec FR-407 thresholds. |
| FND-007 | fixed | c524fa192: one-value-type rule for a reward name with the ambiguous-name refusal; FR-186-AC-6 covers it. |
| FND-008 | fixed | c524fa192: one statement owns the five missing-name refusals, located at the name; FR-186-AC-6 covers them. |
| FND-009 | fixed | c524fa192: the Description lists the fairness set. |
| FND-010 | fixed | c524fa192: the subject's content is stated as the model alias as written and the domain package it selects. |
