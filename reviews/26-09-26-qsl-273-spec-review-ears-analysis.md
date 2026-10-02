---
id: SR-661
title: "QSL-273 EARS review of state clauses on the spine"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@d8b74aba7d349ccb3989583cc4e608aad301c38b; spec/functional/FR-102 to FR-109 (focus FR-103, FR-105, FR-106, FR-107); quire validate grammar warnings over the changed files"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: reviews
---

## Summary

Ticket: QSL-273 (PR agent-ix/quire-spec-language#462). This review checks the
requirement statements in FR-102 to FR-109 against the EARS patterns.
`quire validate` reports 18 grammar warnings, all in FR-103, FR-105, FR-106
and FR-107. For each one, this review decides whether it changes meaning.

Clean: the Description leads of FR-102, FR-103, FR-104 and FR-105 are
well-formed event-driven statements. FR-107 and FR-109 are ubiquitous
statements with one actor each. No statement uses a weak modal.

Meaning-changing: FR-106 checks 1 and 6 (non-singular) and check 7
(agentless, with an ambiguous term). FND-001 and FND-002 cover them.

Meaning-preserving: FR-103:72 and :77, FR-105:89, FR-107:58, :73 and :74, and
FR-106:182. In each, the subject is implied by the section (the assembler,
intake, S4, the entry, S6a, admission). FND-003 lists them.

Verdict: changes requested (two medium).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-106 checks 1 and 6 are non-singular (quire `ears:non-singular` at lines 132, 136 and 161). Each packs several SHALL conditions and one pooled "Failures" list into one statement. The meaning changes because FR-106's "stop at the first that fails" rule then has no order among the conditions inside a check. Nothing says whether the digest is checked before the member set, or which object's bound violation is reported. Failure scenario: two conforming implementations report different single records for one multi-defect snapshot (SR-660 FND-002). Split each into ordered "If <condition>, then admission shall return <code>/<cause>" statements. | spec/functional/FR-106-admit-snapshots-and-invocations.md:132-147, 159-167 |
| FND-002 | medium | FR-106 check 7, "Every population the clause requires ... SHALL be marked complete" (quire `quality:agentless-passive`, line 170), states an obligation on the input document, not on the system. Its defining phrase is ambiguous: "each population a reference-typed field of a required population's type targets". A field targets a type, not a population, so "targets" can be read by declared type or by the populations the reference values name. The meaning changes: under the by-type reading, every population whose member type is `ConfigVersion` becomes required, and an unrelated incomplete population makes a healthy case `Incomplete`. Failure scenario: two implementations disagree on whether a snapshot with a second, incomplete `ConfigVersion` population is `Incomplete` or admitted. Rewrite it as "If a required population is not marked complete, then admission shall return Incomplete ...". Define "required" as the transitive closure over the populations named by the reference values read from `self`'s population, or state the by-type rule explicitly. | spec/functional/FR-106-admit-snapshots-and-invocations.md:168-172 |
| FND-003 | low | These statements have no subject (`ears:missing-subject`, `ears:unclassifiable` or `quality:agentless-passive`), with the meaning unchanged: FR-103:72 ("An operation SHALL be declared", i.e. the assembler), FR-103:77 (the domain package's `pre`/`post` texts "SHALL still be shape-checked", i.e. intake), FR-105:89 ("Both directions of the mapping ... SHALL be total", a property of `CheckedClauseKind`, whose incompleteness SR-662 FND-001 covers), FR-107:58 ("A name ... SHALL refuse", i.e. `evaluate_clause`), FR-107:73-74 (S6a), and FR-106:182 (the frame check). FR-106's Description, "QSL SHALL define the spine clause-execution input", states no verifiable behaviour; its second and third sentences carry the requirement. Failure scenario: tracing a SHALL to an allocated component finds none for these lines. Name the actor in each. | spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:72, 76-78; spec/functional/FR-105-emit-state-nodes.md:85-89; spec/functional/FR-106-admit-snapshots-and-invocations.md:25-31, 186-192; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:55-60, 70-73 |

## Verdict

Changes requested: FND-001 and FND-002 change what an implementation must
report.

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@c35a6a49` (fix commit `c35a6a49`, "QSL-273 spec: fix SR-660 to SR-664 review findings", rebased onto main 5e7a2615). Each outcome was re-checked against the spec and code at that head, not taken from the commit message. `quire validate` over the changed spec files and these reviews exits 0 with no EARS warnings.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed c35a6a49 | Checks 1 and 6 are split into ordered If/then sub-conditions (1.1 to 1.8, 6.1 to 6.5). |
| FND-002 | fixed c35a6a49 | Check 7 is an If/then statement, and "required population" is defined by the references read from `self`'s population, with TC-465 row 25 as its adverse case. |
| FND-003 | fixed c35a6a49 | Each statement names its actor, and FR-106's Description is event-driven. `quire validate` reports no EARS warnings on FR-102 to FR-109. |
