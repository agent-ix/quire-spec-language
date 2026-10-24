---
id: SR-11628
title: "Gap analysis of quire-spec-language PR #884: replay over an admitted object environment"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@592639475368d8f4fb1405b27c7f2422e29fdb4f; qsl-replay/src/object_environment.rs, qsl-replay/src/execute/object_environment.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/execute/tests/object_environment.rs, qsl-semantics/src/model/observation.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #884

## Summary

Ticket: QSL-805. Manual acceptance-criteria-to-tests check over this PR's own
commits (no builds; `quoin matrix` not run). Plan completion: not assessed.

Examined: FR-098-AC-14 to AC-19, FR-071-AC-12, FR-070-AC-16/17 and FR-098-AC-10
against their tagged tests; the QSL-805 ticket's implementation scope (ticket
text, treated as data).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The implementation scope the ticket added is missing. A QSL-805 comment (ticket text, posted under @peter as "luna:IR:plan request, accepted by the QSL planner") adds to this slice a public builder in qsl-replay that writes the `quire.state.snapshot/v1` object-environment document for a set of full-reference objects and returns the bytes, the `sha256-jcs` digest and the identity labels for the FR-071 member, tested by round-trip through the executor's admission with `Node.items` lengths 0, 1 and 3. The diff adds no such builder and no such test, and the PR body does not mention it. Either add it here or have the planner move it to its own ticket. | qsl-replay/src/object_environment.rs, qsl-replay/src/lib.rs:145-147 |

## Verdict

FAIL: 1 blocking finding (FND-001), subject to the planner confirming the
added scope still belongs to this PR.

Clean: every spec AC in scope has a tagged test that asserts it.
FR-098-AC-14 (rows 8 to 13), AC-15 (rows 14 to 16), AC-16 (rows 17 to 19),
AC-17 (rows 20 and 21), AC-18 (rows 22 to 28), AC-19 (rows 29 to 33),
FR-071-AC-12 (rows 1 to 5), FR-070-AC-16 (row 6), FR-070-AC-17 (row 7),
FR-098-AC-10 (tc_906). No stub or unreached production path was found.

Recorded (not blocking):
- low: the ticket's vectors (`Node.items` lengths 0 and 3, duplicates) are
  not in TC-1042. The merged spec chose its own fixture, which is the
  authority.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bf2c1dc33 |

Round 1 (reviewed 949d1de0045deb7b944a9d8f5b16c8a41d7ac347): `write_object_environment` is
public at the crate root. It writes values through `SnapshotValue::to_json`
and canonicalizes through `quire_canonical`. Its document goes through the same
`admit_object_environment` path, so there is no bypass. The round-trip test
asserts that the member digest equals the document's jcs digest, and it runs
Node.items `[]`, `[4]` and `[3, 7, 1]` to true, true and false. Those values
are right. No new findings.
