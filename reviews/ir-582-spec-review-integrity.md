---
id: SR-1300
title: "Spec review of PR #632 (IR-582): spec edits for the extracted shared leaves"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@f40729a9a9adafaeb233fadcb938245f2861776c; spec/decisions/ADR-011, ADR-029; spec/functional/FR-059, FR-060, FR-068, FR-089, FR-106, FR-284, FR-272 (unchanged); spec/test-cases/TC-156, TC-157, TC-292, TC-295, TC-297 (deleted), TC-388, TC-390, TC-407, TC-408, TC-411, TC-441, TC-904 (deleted); spec/tests.md; spec/spec.md"
review_set: subset
---

## Summary

Ticket: IR-582. Checked each deleted spec item for a successor with matching
ids in the owning repository:

- FR-089-AC-6 / TC-297: moved to agent-ix/quire-exact (`FR-089-kernel-refuses-a-population-pair.md`,
  `TC-297-...`). Same ids, same AC text, three trace-tagged tests. QSL FR-089
  adds `depends_on ix://agent-ix/quire-exact/FR-089`.
- FR-106-AC-10 / TC-904: moved to agent-ix/quire-semantic-value. Same ids,
  same AC text. QSL FR-106 adds `depends_on ix://agent-ix/quire-semantic-value/FR-106`.
- FR-060-AC-5 / TC-157 step 8: moved to quire-semantic-value `FR-060-AC-5`,
  verified by clippy `disallowed-methods` analysis instead of a TC-157 test.
  Same AC id.
- tests.md rows for TC-297 and TC-904 removed; TC-157's range is now AC-1..AC-4.
- FR-059, FR-284, ADR-029 and ADR-011's dependency table describe the leaves
  as their own repositories, consistent with the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-272 is not amended. Its Inputs and Behavior say the gate scans workspace members plus "every resolved package that FR-059's `graph::classify` assigns to an ecosystem repository"; the code now also scans `quire-exact` and `quire-semantic-value` by name, labelled as workspace members in a QSL run. The code goes beyond the spec | spec/functional/FR-272-fail-on-a-second-definition-of-a-canonical-type.md:29-34,79-82 |
| FND-002 | low | ADR-011's SV rule now says T-12 rule (b) "no longer reaches this crate", and QSL FR-060 drops AC-5 with no pointer. Neither names the successor (quire-semantic-value FR-060-AC-5, clippy `disallowed-methods`), so a reader concludes the property is unenforced | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:831-834; spec/functional/FR-060-check-qsl-api-surface-boundary.md:166-169 |
| FND-003 | low | QSL test cases keep inspection steps that verify quire-exact's own source: TC-292 steps 1-2 (the `Value::Population` payload and quire-exact's `Cargo.toml`), and step 5 of TC-388, TC-407 and TC-408 (quire-exact's `outcome.rs` enums). Those steps verify quire-exact properties and belong to quire-exact's spec | spec/test-cases/TC-292-kernel-population-variant-carries-no-model-type.md:30-33; spec/test-cases/TC-388-wrong-anchor-reaches-the-caller-as-a-coded-refusal.md:39 |

## Verdict

Changes requested (no high). The three deleted ACs and two deleted TCs each
moved to their owning repository with matching ids. FND-001 must land with
SR-1298 FND-001's code fix so that FR-272 states the leaves' classification.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | FR-090-AC-7, AC-11 and AC-12 still end with kernel enum claims (`quire_exact::Refusal` has no variant naming `WrongSnapshotCause`; `quire_exact::Undefined` has no `PreconditionFalse` / `AbsentKey` variant). The fix round deleted the TC-388/407/408 step 5 that verified them, so in QSL those clauses no longer have a verifying step. They now belong to quire-exact's FR-090 (quire-exact#3). Drop the kernel clause from each of the three QSL ACs and point to `ix://agent-ix/quire-exact/FR-090` | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:461,465,466 |

## Dispositions

Round 1, reviewed at c989d92266f8025c27bfdbe0c65bcc54a331f9e8.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e36d68e99: FR-272 Inputs and a new Behavior bullet scan quire-exact and quire-semantic-value as their own ecosystem repositories; `identifier`/`re-export` never apply inside them |
| FND-002 | fixed | e36d68e99: ADR-011 and FR-060 name quire-semantic-value FR-060-AC-5 and FR-060-AC-6 as the replacement |
| FND-003 | fixed | e36d68e99: TC-292 and FR-089-AC-2 deleted from QSL and the step 5 inspections removed from TC-388/407/408; their successors are in quire-exact#3 (0edd08bf) |

Round 2, reviewed at fbade25d8c9be802105caca2ffc429e9b4e33031 (the code
change since round 1 is 67f46ae64's deref drop; make ci exit=0 on 67f46ae64 in
`qsl-ir582-make-ci3.log`, and everything after it is spec only).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | 3f11ecc7f: AC-7/11/12 drop the kernel-enum clauses and FR-090 gains `depends_on ix://agent-ix/quire-exact/FR-090`; c3f386ffe and fbade25d8 repoint the remaining kernel-enum prose |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-090-AC-9's last sentence still says "Neither `quire-exact`, `qsl-foundation` nor `qsl-cst` depends on the crate that defines the three". This PR dropped quire-exact's `Cargo.toml` from TC-390 step 4, so nothing in QSL verifies the `quire-exact` part. Drop `quire-exact` from the sentence. quire-exact's own FR-089-AC-2/TC-292 (no caller model crate in its dependencies) and its `deny.toml` `allow-git`, which omits quire-spec-language, already hold it. The round-0 review missed this | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:463 |
