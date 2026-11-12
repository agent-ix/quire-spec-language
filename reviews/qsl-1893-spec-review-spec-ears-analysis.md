---
id: SR-2689003
title: "EARS review of protocol analysis requirements FR-570..FR-575 (quire-spec-language PR 2689)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-570..FR-575; spec/test-cases/TC-1410..TC-1415; PR 2689 head 47ca5f0de4f20e2f6c299dad23e60da56554ddf4"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-570
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-571
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-572
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-573
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-574
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-575
    type: references
---

# EARS review of protocol analysis requirements FR-570..FR-575 (quire-spec-language PR 2689)

## Summary

Ticket: QSL-1893. Method: spec-review/spec-ears-analysis. Diff: `git diff origin/main...HEAD` at 47ca5f0de4f20e2f6c299dad23e60da56554ddf4, 12 files, spec-only.

Description statements of FR-570..575 each carry one SHALL-led capability. Behavior bullets mostly use 'shall'. Two wording defects follow.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | quire validate reports `[ears:non-singular]` at line 63: the parallel bullet holds three `shall`s, and the choice bullet at line 66 holds two. Split each into one requirement per `shall`. | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:63-72 |
| FND-002 | low | This Behavior bullet ('Corpora cannot use retry loops or non-`all` joins...') states a consequence for corpus authors, with no actor and no `shall`. It restates ADR-036's cost and imposes no QSL obligation. Move it to the Description or a note. | spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md:74-75 |

## Scope and evidence

- FR-570 (examined; spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): QSL SHALL decide the `local-projection` capability kind for a checked global protocol and one role, in the `bounded-static` profile (FR-575), and return either the role's local graph with the evidence that it is sound or a minimal global-path counterexample. It implements ADR-036 PA-9 to PA-12 and PA-25 (projection artifacts) and QSpec FR-174-AC-1 to AC-5. The key of the result is FR-572's; its settlement is FR-573's.
- FR-571 (examined; spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): QSL SHALL decide the `realizability` capability kind for a checked global protocol in the `bounded-static` profile (FR-575) as a game in which the environment chooses admitted external events and channel outcomes and each role chooses from its local history only, and return a replayable strategy witness, a counterexample, or a non-conclusive result. It implements ADR-036 PA-7, PA-8, PA-16 to PA-19 and PA-25 (realizability artifacts) and QSpec FR-175-AC-1 to AC-5. The key is FR-572's; settlement is FR-573's.
- FR-572 (examined; spec/functional/FR-572-key-protocol-analysis-results-structurally.md): Every projection and realizability result SHALL carry the structural key of QSpec FR-174-AC-6 or FR-175-AC-6 as typed members, SHALL never stand in a digest for a member, and SHALL be a function of its key. It implements ADR-036 PA-13 to PA-15 and QSpec FR-174-AC-6 and FR-175-AC-6.
- FR-573 (examined; spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): QSL SHALL settle every projection (FR-570) and realizability (FR-571) result with one QSpec FR-360 label, through the FR-241, FR-242 and FR-243 combination FR-360 names, by the existing native-engine rule and with no rule of its own. It implements ADR-036 PA-12, PA-14 and PA-20 (ADR-011 S6c; ADR-018 SM-7, PC-1).
- FR-574 (examined; spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): Every `decisive-counterexample` result of FR-570 or FR-571 SHALL carry exactly one QSpec FR-351 separating witness record, with the trace position of the step at which the divergence or violation occurs, and its `value_path` derived from QSpec FR-207 as below. It implements ADR-036 PA-24 (ADR-031 SW-5, SW-7 as amended there). quire-protocol FR-302 imports the record byte-exactly and refuses a decisive realizability result without it.
- FR-575 (examined; spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): QSL SHALL decide projection and realizability only in the single profile `bounded-static`, and SHALL answer `unsupported`, naming a located construct cause under the existing `unsupported_projection` code, for every construct outside it, with no substitute graph and no guess. It implements ADR-036 PA-21 and PA-22 and QSpec FR-174-AC-3, and fixes the open item that ADR-036 left to the requirements: the profile word and the cause slugs.

## Verdict

2 finding(s); FND-001 low, FND-002 low.
