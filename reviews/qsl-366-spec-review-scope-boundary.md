---
id: SR-963
title: "QSL-366 scope-boundary review: the QSL and QSpec split for temporal model checking"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@9b546eb069048b46d0c365f85c4481c0a3012115; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/functional/FR-123..FR-128"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: reviews
---

## Summary

Ticket: QSL-366; QSpec half STD-131. ADR-018 states that QSpec owns these
items, through QS-1 to QS-13:
- the grammar;
- the model subject semantics;
- the wire forms;
- the verdict table;
- the conformance vectors.

This review checks the boundary in both directions.

**QSpec to QSL.** The QS table leaves nothing to QSpec that is QSL's own:
- `ProofBasis`, `ModelCheckOutcome`, the engines and the default limit stay in QSL.
- QS-6 asks QSpec only for the method and depth members on the wire.

**QSL to QSpec.** The ADR keeps spellings illustrative (§6, DL-1). FR-123
and FR-124 do not (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-123 fixes the fairness grammar normatively: "The checker SHALL read a fairness constraint form `fair <kind> [<granularity>] <operation>`. The kind is required". FR-124 does the same for `terminal when <state predicate>` and `terminal any`, with refusal codes for a second member. ADR-018 §6 and DL-1 call both spellings illustrative, and QS-4 and QS-12 give the grammar to QSpec's shared grammar. Reword the FRs to check the checked forms (a constraint with kind, operation and optional granularity; a terminal declaration `When(P)` or `Any`) as the shared grammar spells them. Keep only the semantic rules normative: unmarked means `Whole`, at most one member, `P` is a state predicate. | spec/functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md:65-68; spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:40-41,58-60 |
| FND-002 | low | FR-125, FR-127 and FR-128 state QSpec-owned semantics as SHALLs before the QSpec FRs exist: the model subject, positions, terminal stutter and `over` binding (QS-1, QS-2, QS-5), the verdict table (QS-6), and the counterexample content and replay rules (QS-8). A QSL implementing FR has to state them while STD-131 is open. When the QSpec FRs land, each should cite its QSpec FR as the authority, as FR-101 cites QSpec FR-181, so the two cannot drift. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:59-105; spec/functional/FR-128-replay-a-model-counterexample.md:53-63 |
| FND-003 | low | §7 holds cross-repository delivery order: step 5 waits on IR and CG work, and step 4 is a roadmap of later language features. Its last paragraph names an out-of-scope alternative ("Lowering ... to TLA+ or Quint ... is later research and outside this record"). This repository is public. Ecosystem delivery order belongs in quire-research, and a spec states what is, not what is not. Keep this repository's own sequencing (steps 1 to 3), keep the DS table as architecture, and leave the TLA+/Quint pointer in References only, where RES-40 to RES-50 already sit. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:384-421 |

## Verdict

The split is sound in design: the QS table is complete, and QSL keeps what
is QSL's. FND-001 is the one place where QSL specifies QSpec's grammar
normatively, against the ADR's own statement.

## Dispositions

Round 1, reviewed at ed7bcc8b (re-checked at the #562 head; first drafted at f7180085). This file's findings were posted on QSL-366 under the id SR-951.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 906c3b0b |
| FND-002 | fixed | 17928bf4 |
| FND-003 | fixed | 204b4db1 |
