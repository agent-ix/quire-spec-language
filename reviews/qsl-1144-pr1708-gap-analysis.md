---
id: SR-1708002
title: "Gap analysis of quire-spec-language PR #1708: drain-bearing source-default memory classes (QSL-1144, semantics slice)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@edb5ff719; PR #1708 against QSL-1144, FR-375-AC-23, FR-375-AC-26, FR-432-AC-3, TC-310 CC-12/CC-15 (QSpec origin/main 792993ab)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-375
    type: reviews
---
# Gap analysis of quire-spec-language PR #1708

## Summary

Ticket: QSL-1144. Plan completion: not assessed.

**Covered.** FR-375-AC-26's admitted half (CC-12) is covered by
`each_source_default_model_defines_a_drain_class` and
`drains_are_keyed_by_model_ordered_and_unique`:
- three drains ordered `ra`, `sc`, `tso`, then `flush(tso)`;
- one entry per model whatever the source order;
- two `tso` controls giving one drain.

**Out of this slice.** QSL-1144's own goal is S3/S4 emission and readback
in the V2 `class_catalog`. QSL-1181 (Backlog) tracks that, since protocol
refinements are not emitted yet. CC-13 and CC-15's refusal half (reordered
drains, a repeated pair, a misshaped memory entry) belongs to the reader,
so it moves with QSL-1181.

**The author's open question: is a `step memory drain` row mandatory like a
flush row?** Yes, by FR-432's general rule, and no AC pins it:
- FR-432 Description says the checker "SHALL decide every concrete protocol
  step class by exactly one explicit deciding `step` row".
- FR-432 says "Nothing defaults: a concrete step class with no row refuses
  at check time `missing_declaration`/`missing-name`, naming the … memory
  step kind".
- FR-432's class table lists "cross-model drain class | `step memory
  drain`, available under source defaults `sc`, `tso` and `ra`".
- FR-375 makes `memory drain` "a class of each source-default model" in the
  catalog of every subject with a source `parallel`.

So every concrete protocol with any `parallel`, even under `sc`, needs a
`step memory drain` row. Only FR-432-AC-3 says this for flush ("no `step
memory flush` row refuses"). For drain it says only that the row "is a
valid class", and no AC or TC row tests a missing drain row. Given how far
this reaches (every existing concrete protocol with a `parallel` would need
a new row), the spec owner should state it as an AC before QSL enforces it.

## Verdict

**The slice is met for the admitted catalog.** The drain class is missing
from rows and internal lists, and its mandatoriness is unspecified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Under FR-432's "Nothing defaults" rule and class table, a `step memory drain` row is required for every concrete protocol whose `parallel` selects any model. Yet FR-432-AC-3 and TC-310 test only the flush row, and the drain sentence says only "valid". QSL neither requires nor admits the row (SR-1708001 FND-001). Raise a spec ticket asking the owner to confirm the drain row is mandatory and add an AC/TC row, then implement it with the row support. | QSpec spec/functional/protocol/FR-432-map-every-protocol-step-in-refinements.md (Description, "Nothing defaults", class table, AC-3); FR-375 "Checked class catalog" |
| FND-002 | low | The ticket's emission and readback ACs, and CC-13/CC-15's refusal half, are not in this slice. QSL-1181 tracks emission, but the PR body does not name it, and QSL-1144 does not record that the reader-side refusals move there. Record both on QSL-1144. | QSL-1144; QSL-1181; TC-310 CC-13, CC-15 |

## Dispositions

Round 1, fix commit e5a023bad. The gates are those in SR-1708001's dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Drain rows are admitted and optional for now, under the ruling that they become mandatory when titan's QSpec AC lands. That is a one-line switch to `Need::Row`, and the test that checks a refinement with no drain row will flip with it. The open item is the spec AC. |
| FND-002 | fixed | The PR body now says: "In QSL-1181, not here: V2 wire emission and read-back of the `class_catalog`, the drain entries included, and the refusal halves of TC-310's CC-13 and CC-15." |
