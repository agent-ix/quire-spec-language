---
id: SR-13608
title: "ears-conformance review of the probabilistic spec follow-ups (PR #1124)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@8a1f1ba219d7c8344c06301b2758cfad1a03fa23; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md, spec/functional/FR-196-build-the-probabilistic-product.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
---
# SR-13608: ears-conformance review of PR #1124 (QSL-573, QSL-568, QSL-571)

## Summary

Ticket: QSL-573 (also QSL-568, QSL-571). This review covers the
requirement statements the diff adds or edits: FR-186's `over` bullet and
the `unbounded-event` exception; FR-192's EN-4 refusal bullet; FR-195's
Reach operand bullet, the expected-reward threshold bullet, the timed
bullet, the negotiation-gate and settlement bullets; and FR-196's build
entry prose, `resolved` rule, resolution-reward rule, the unactivated and
stutter paragraphs, and the Undefined and resolution Behavior bullets.
`make check-spec-validation` gives no `[ears:*]` or `[quality:*]` warning on
any touched file. Semantic review found four low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The expected-reward threshold bullet is compound and mostly not normative: one SHALL (`c` SHALL be a literal) and one SHALL-no (no sign or range check) sit among definitions and passive refusals with no SHALL ("A threshold that is no literal is refused `ill_typed`/`type-mismatch`, as is a bare number...", "A quantity for an integer reward ... are refused `ill_typed`/`unit-mismatch`"), plus rationale ("A quotient such as 6/5 is not available, because ..."). FR-195-AC-2 tests the passive refusals as requirements. Fix: one ubiquitous statement for the admitted threshold types, then one unwanted-behaviour statement per refusal ("If c is not a literal, S3 SHALL refuse ... `type-mismatch`"; "If c is a quantity for an integer reward, or of another dimension, S3 SHALL refuse ... `unit-mismatch`"); move the `/` and unary-minus rationale out of the bullet. | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:89-108 |
| FND-002 | low | Two new paragraphs of the bounded-measure semantics carry testable behaviour with no SHALL: "Unactivated is a decided, absorbing control phase of its own ..." and "A stutter step is a model step like any other: an active `steps` measure adds 1 on it ...". FR-196-AC-11 tests both. Fix: state them as requirements in Behavior ("At the horizon, EN-5 SHALL decide a measure never activated as `Unactivated`, absorbing, with amount zero"; "On a terminal stutter step EN-5 SHALL advance an active `steps` measure by 1") and keep the explanation as prose. | spec/functional/FR-196-build-the-probabilistic-product.md:379-392 |
| FND-003 | low | The new Behavior bullet "A resolution edge SHALL carry every reward of the model at 0, and the intermediate state's resolution SHALL advance the monitor and the accumulator once, with the draw's rewards" holds two SHALLs with different subjects (the edge's reward payload; the resolution's control update). The same applies to the `Undefined` bullet above it, which adds "Over every scheduler the path SHALL carry `resolved` ..." to a statement with its own SHALL. Fix: split each into one statement per SHALL. | spec/functional/FR-196-build-the-probabilistic-product.md:706-708 |
| FND-004 | low | The new FR-186 bullet states the requirement and its rationale in one sentence ("An `over` parameter SHALL have an object type, because FR-125 'Instancing' binds `over` ..."), and then states the same condition again as the unwanted-behaviour refusal. FR-195's `not-state-predicate` bullet does the same ("A constant is refused as well as an operator because EN-5 labels each model state ..."). Fix: keep the unwanted-behaviour statement ("If an `over` parameter's type is not an object type ..., S3 SHALL refuse ...") and move the rationale to prose. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:126-131 |

## Verdict

Mergeable as far as EARS goes. All four findings are wording fixes for the
same fix round. The other edited statements conform: the `over-not-object-type`,
`not-state-predicate` and gate refusals are well-formed unwanted-behaviour
and event-driven statements.
