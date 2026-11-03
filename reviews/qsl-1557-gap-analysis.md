---
id: SR-20261103230158
title: "Gap analysis of QSL-1557 PR #2220 clock-selections flag"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d5b2e550570cc3f2a2369b8ef7ea96e8e61e8b72; PR #2220 diff: FR-517 Inputs and AC-15, TC-1312 row 15, src/emit_checked_v2.rs, tests/it/checked_handoff.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1312
    type: reviews
---

## Summary

Ticket: QSL-1557. Plan completion: not assessed. The scope is the PR diff only. FR-517-AC-15 is new, and one test is tagged for it: `#[trace("FR-517-AC-15", "TC-1312")]` on `the_clock_selections_flag_emits_the_requests_bytes_and_bad_files_refuse`. The test asserts every clause of the AC. For each of the three recipe kinds, it checks parse equality with the struct `Request` and checks that `run` writes the same stdout bytes as `emit`. For each of the four malformed files, it checks for an `Error::Clocks` that names the file, exit 20, and empty stdout. The new production code is `Error::Clocks`, `read_clock_selections`, the parse arm, and the `NamedEntries` rename, and all of it is owned by the FR-517 Inputs paragraph. Running `quire trace` timed out after 120 s and was stopped, so the trace was checked by reading the tag. Quoin 0.29.1.

## Verdict

**PASS with one low finding.** No stub, no code without a requirement, and no inflated coverage. The test asserts real bytes and real error variants.

## Scope

- `FR-517-AC-15`: examined. Tagged test present and asserting.
- `TC-1312` row 15: examined. It matches the test's procedure.
- FR-517 Inputs, the `--clock-selections` paragraph: examined against the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-517 Inputs paragraph names two outcomes that no test reaches through the command line. The first is a value that is not a well-formed clock configuration, which also covers a missing required member such as `{"kind": "timestamped_event"}` and a wrongly typed member. The second is a name that matches no clause, which should give `UnknownClause` through the CLI. The AC-15 test covers only an unknown `kind` and an extra member. `UnknownClause` is covered only through the struct path, in `bad_requests_get_their_own_error`. The code handles both cases through serde and the shared request field. Fix: add a missing-member file to the malformed list, and add one CLI case with an unknown name that asserts `Error::UnknownClause`. | tests/it/checked_handoff.rs:580-604 |
