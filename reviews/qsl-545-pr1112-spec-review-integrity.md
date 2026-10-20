---
id: SR-13594
title: "integrity review of QSL-545 refinement history spec follow-ups (PR #1112)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@cc890d9998cd4ca09d16522620b1ef03907ba528; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/functional/FR-138-check-and-compute-history-fields.md, spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md, spec/functional/FR-363-type-navigation-over-a-relationship-end.md, spec/test-cases/TC-540-s3-checks-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/test-cases/TC-543-history-fields-check-and-compute-without-changing-the-concrete-model.md, spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-149
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-363
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-540
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-543
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-556
    type: reviews
---
# SR-13594: integrity review of QSL-545 (PR #1112)

## Summary

Ticket: QSL-545. Consistency and conflict analysis over
`git diff origin/main...cc890d999` (8 files, +60/-19), checked against the
code on origin/main (4e5825762) and QSpec on main (4ea88c83e): QSpec FR-181,
FR-272, FR-375 and FR-378.

Code checked: `qsl-semantics/src/check/refinement.rs` (`UnsupportedFeature`,
`RefinementCause::Unsupported { subject }`, `history_names`),
`qsl-semantics/src/check/refinement/history.rs` (`history_headers`,
`unkeyable`, `bounded`), `qsl-semantics/src/check/state_model.rs:287`,
`qsl-eval/src/simulation/model/encode.rs:257-261`,
`qsl-eval/src/simulation/model/refine.rs:160`,
`qsl-eval/src/simulation/model/contracts.rs:164`,
`qsl-replay/src/certificate/simulation.rs` (`IncompleteLimit`, `matches`,
the rule order) and quire-exact `LimitKind::as_str` at the pinned revision
(`integer_bits`, `work_units`).

What matches the code: the enum, decimal and option-of-those refusal at
the type span with `subject` (history.rs:174-183); its place after the
name-conflict check and before the bounded-type and literal checks; no
`subject` on `protocol-abstract-side` (refinement.rs:798); FR-120's
`EvaluationMeter` label for any `Incomplete` (contracts.rs:164); FR-149's
limit spelling (`evaluation-meter` for `WorkUnits`, otherwise the quire-exact
snake-case name); FR-149's History bullet (`matches` compares the concrete
key, the history key and the abstract keys); FR-363-AC-2's exception
(state_model.rs:287 refuses missing-name only when no field and no end
match and the name is a unit-wide history name). The closed four feature
names of QSpec FR-375 are kept, with no fifth name. FR-149's History bullet
relies on QSL's product-key order, and FR-138's References already records
that order's difference from QSpec FR-378.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The reason given for refusing enum and decimal history types is wrong. FR-135 and FR-138 say FR-181 gives these values no typed canonical form. QSpec FR-181 does define both forms: decimal at line 87 and enum at line 104 of `FR-181-simulate-finite-executions.md`. The gap is in QSL's encoder: `encode.rs:257` and `:261` return `Unencodable`, and QSL-823 tracks it. The bare "FR-181" also resolves to QSL FR-181, the existential-claim requirement. Read against QSpec FR-181, FR-135's condition "the declared types FR-181 gives a typed canonical value form" covers enum and decimal, which contradicts the next sentence that refuses them. Fix: name the refused kinds directly, and give the reason as this build's value-key encoder (QSpec FR-181's enum and decimal forms are not built). | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:199-201; spec/functional/FR-138-check-and-compute-history-fields.md:69 |
| FND-002 | medium | FR-135 redefines what spells `history-row`: "a `history` row whose declared type has no typed canonical value form". The sentence says it only names QSpec FR-375's set, and FR-375 spells `history-row` as any `history` row. Under the new spelling, line 199's "SHALL hold `history-row` for the declared types FR-181 gives a typed canonical value form" says nothing, because no keyable row would spell the feature. Fix: keep FR-375's spelling and state the partial support only in the support-set bullet. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:177-178 |
| FND-003 | medium | FR-135's general rule is that the refusal "is decided before the feature's own checks, so no other diagnostic about that row is reported". The PR leaves that line unchanged. The new FR-138 bullet places the history-row refusal after the name-conflict checks, and the code also runs object-type resolution and type-form resolution first (history.rs:128-170). So an enum row with a conflicting name reports `conflicting-binding` and no `not-yet-implemented`, which breaks FR-135's rule. Fix: make the history-row type refusal an explicit exception in FR-135, citing FR-138's order. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:196-198; spec/functional/FR-138-check-and-compute-history-fields.md:70-71 |
| FND-004 | medium | The PR departs from QSpec in three ways and records none of them. FR-135 itself says FR-375 "owns ... the not-yet-built refusal". (a) FR-375 places the `history-row` refusal at the `history` keyword; QSL places it at the declared type. (b) FR-375 models a feature as in or out of the support set, and FR-375-AC-9 admits a full-path enum-member history literal in a bounded type; QSL holds `history-row` only for some types. (c) FR-272's `not-yet-implemented` payload is the feature, the declaration and the profile, and `subject` already means a declaration identity there (`conflicting_declaration`); QSL adds `subject` as a kind of type. FR-138's References records the FR-378 key-order difference in the same way this should be recorded. Fix: record the three differences in FR-135/FR-138 References and route them to QSpec on a ticket. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:190-207; spec/functional/FR-138-check-and-compute-history-fields.md:65-71 |
| FND-005 | low | FR-138 says the enum/decimal refusal "comes after the name conflict checks and before the bounded-type and initial-literal checks". It leaves out the other refusals that also come first: the unresolved object type (`missing-name`) and a type form that does not resolve (history.rs:128-170). A reader could take the name-conflict check as the only one that comes first. | spec/functional/FR-138-check-and-compute-history-fields.md:70-71 |
| FND-006 | low | FR-138's References (unchanged by the PR) still says QSpec TC-310 mutation 10 covers the enum-member literal case. With this PR, QSL refuses an enum-typed history row before its literal is checked, so QSL never admits an enum-member history literal. The reference should say that QSL refuses this case for now. | spec/functional/FR-138-check-and-compute-history-fields.md:174-177 |
| FND-007 | low | FR-149 calls `evaluation-meter` "FR-126's spelling", but FR-126 writes only `EvaluationMeter`. The kebab-case string is written in the code (`qsl-replay/src/model_check/limits.rs:99`), and FR-127 uses it only in prose. Cite the requirement that actually defines the wire spelling. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:128 |

## Verdict

Not mergeable as is. One high finding and three medium ones; all of them
are spec text fixes plus one QSpec routing.

The behavior the PR describes matches the merged code everywhere I checked.
The defects are the stated reason (FND-001), the feature's spelling and the
general ordering rule, which now contradict the new rule (FND-002, FND-003),
and the departures from QSpec, which are not recorded (FND-004). FR-120,
FR-149's limit spelling and History bullet, and FR-363-AC-2 are clean for
consistency.
