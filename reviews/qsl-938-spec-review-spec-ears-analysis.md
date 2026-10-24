---
id: SR-13674
title: "EARS review of QSL-938 FR-395, FR-062 owner and FR-075 identity statements (PR #1147)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@29f11fe9b396165cbfd7fe9c52f50aec5074cbdc; spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md, spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md, spec/functional/FR-062-implement-checked-family-contract.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1089
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
# SR-13674: EARS review of QSL-938 FR-395, FR-062 owner and FR-075 identity statements (PR #1147)

## Summary

Ticket: QSL-938. EARS conformance of the requirement statements the diff
adds: FR-395's Outputs and Behavior bullets, FR-062's Owner paragraph,
FR-075's request-index identity paragraph and FR-100's pointer paragraph.
The quire validator raises no EARS warning on these files. It checks only
`shall` statements, and the new text has none, so the gate does not see the
issue. `grep -c SHALL` on FR-395 is 0 (FR-394, the cited pattern, is also 0;
FR-100 and FR-062's Records paragraph use SHALL).

Examined: FR-395 Outputs (3 bullets) and Behavior (8 bullets), FR-062 Owner
paragraph, FR-075's new paragraph, FR-100's new paragraph.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-395 states every obligation as present-tense description with no `SHALL` and often no agent. Examples: "The members are exactly the package's records whose owner is the selected function"; "The entry compiles the source once"; "is a broken check-stage invariant: `Fault`"; "`run` keeps its signature". Write each Behavior bullet as one EARS statement with an agent, for example "`run_with_requirements` SHALL return as members exactly the records whose owner is the selected function", and "If a `value-validity` record has no owner, two owners or an unknown owner, then `run_with_requirements` SHALL refuse `Fault` ...". One `shall` per obligation. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:76-128 |
| FND-002 | medium | FR-062's Owner paragraph adds a new check-stage obligation in descriptive form: "The check stage records the owner with the record". The neighbouring Records paragraph states each obligation as "`check` SHALL record ...". Write it as "`check` SHALL record with each `value-validity` record the `Value` function whose body holds its claim site, and SHALL record no owner for an `operation-contract` record". | spec/functional/FR-062-implement-checked-family-contract.md:241-246 |
| FND-003 | low | FR-075's new paragraph mixes a definition ("An item's request index is also its identity") with an obligation on the item writer ("the item written at that index carries that key"). State the obligation as "`items_from_requirements` SHALL write at request index i the item whose `occurrence()` is the i-th record key", and keep the identity sentence as a definition. | spec/functional/FR-075-compute-candidates-from-registered-backends.md:151-154 |

## Verdict

Not EARS-conformant as written. FND-001 and FND-002 are the substantive
ones: FR-395 has no `SHALL`, so neither the validator nor a reader can tell
which sentences bind. FR-100's new paragraph is a pointer with no
obligation of its own, so it is fine.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 29f11fe9b: - **Association rule.** The facade SHALL return as members exactly the package's records whose owner is the selected function, one member per record. Two claim sites of one application node are two records and two members (FR-062). A function with no scalar operation application has no record, and the facade SHALL return an empty member list for it, not a refusal. |
| FND-002 | fixed | 29f11fe9b: **Owner.** The owner of a record is the qualified name of the `Value` function declaration whose body holds its claim site, as the checked graph holds that name. The checker SHALL record the owner as a single field of each `value-validity` record, from the function it was checking when it found the site, so a consumer never recovers it from the occurrence key. The checker SHALL record no owner for an `operation-contract` record (FR-104) and none for a `temporal-satisfaction` record: a temporal clause is not a function body, and a function it names through a predicate does not hold the clause's |
| FND-003 | fixed | 29f11fe9b: An item's request index is its position in the package order. FR-395's call association names an item by the package's `package_id` and this request index, and the item written at that index carries the member's key. For a consumer that pairs a call's member with an item, the item SHALL come from a package of the same `package_id` as the member's (FR-395, "Pairing with items"). |
