---
id: SR-9044
title: "QSL-721 code review of PR #817"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a4ff9dd248c74995588593666070cb63727eed5b; merge-base 199ada0a67f0215b04cc5b90ae3f9c0af38a7be6; base impl/business-model-meanings; qsl-semantics/tests/it/tc235_business_meanings.rs; qsl-semantics/tests/it/tc235_record_event.rs"
review_set: subset
---
## Summary

Ticket: QSL-721. This is a code review with the Rust lane (tests) folded in. It was run read-only over the PR diff in a detached worktree, with no builds.

The PR is test-only. The diff against the merge-base touches two files, both under `qsl-semantics/tests/it/`, and adds 309 lines with no deletions. It adds `Mutation used:` doc notes to M01, M02, M03, M04, M13, M14, M17 and M18. It ports the M01 payload assertions from draft #770 (verbatim, compared against #770's diff) and adds two tests: M17 run 1 (value half) and M18 run 1 (consumption half).

The base has since moved by one commit (#815). That commit does not touch these files, and `git merge-tree` shows no conflict.

Checked clean:

- No production code changes.
- Every in-scope test still carries `#[test]` and ran in gate log `qsl721-gate2.log` on head `a4ff9dd2` (line 1 names the worktree; it is at that sha). The log lists M01, M02, M03, M04, M13, M14, M17 and M18, plus the two new tests, all `ok`. Every section ends `EXIT=0`. The `could not compile fixture` and `no-such-package` lines inside the xtask section are expected-failure fixtures, and that section ends with 182 passed.
- The M04 run 3 note is accurate. `IntakeMissingDeclaration` has `node`, `reference`, `origin` and `path` (a document pointer) and no member-path field. The test asserts the whole typed cause with `assert_eq!`.
- The M17 run 3 two-refusal note matches QSL FR-082 ("refuse specialization for every declaration on the cycle"), and it matches the planner ruling recorded on the ticket.
- These mutation notes trace correctly: M01 (charge loop at `normalize.rs` `build`), M03 runs 1, 2 and 4, M04 runs 2 to 5, M13, M14 (`event.rs` `bind`), M17 runs 2 and 3, and M18 runs 1 and 2. Each note cites assertions, not line numbers, and says it was derived by reading.

## Verdict

**Findings.** FND-001 must be fixed: the M17 run 2 deviation it records is not real, so the test omits the 24-charge assertion that the TC row requires. FND-002 and FND-003 are wording fixes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The M17 run 2 deviation note is false. It says the wrong-meaning supertype "is refused by the reader (`read_refusals`), before `normalize.record` runs, so no 24-charge count is asserted". `read_refusals` calls `read_records`, which returns the semantic-IR error early. The engine's FR-154 boundary is `read_domain_package`, which always returns `Ok`: it keeps `validate_with_semantic_ir`'s refusals in `intake_refusals`, and `build` charges `normalize.record` once per declared source node (`source_record_count`) before it reports them. M17 run 3 uses that path, and its cycle refusal also comes from the semantic-IR reader (`business_generalization_refusal`); it asserts 24 charges with `normalized_refusals(&cycle, 24)`. Run 2 can be asserted the same way. Fix: use `normalized_refusals(&wrong_meaning, 24)`, keep the `assert_malformed_at` locus, delete the run 2 deviation sentence, and run it. | qsl-semantics/tests/it/tc235_record_event.rs:1124-1136,1190-1208; qsl-semantics/src/model/intake.rs:4285-4500 |
| FND-002 | low | Several mutation notes name an assertion that would not fail, or the wrong file. (a) M02: "the 23-charge count fails if either record is not read". The charge count is `source_record_count` over the document, independent of the records the reader returns, so it would not change. (b) M04 run 1 and M18 run 3: removing the rule leaves no refusal, so `normalized_refusals`' `let NormalizeOutcome::Refused ... else panic!` fails first, not `refusals.len() == 1` or the two `assert_malformed_at` loci. (c) New M17 run 1 test: if `planned_equality` stopped comparing declarations, the 2-slot TaxedMoney against the 1-slot Money would fall to the kind-mismatch arm and return `Err(ValueKindMismatch)`. The `.completed().expect(..)` panics; the `!equal` assertion is never reached. (d) M03's note says it was "derived by reading `model/normalize.rs`", but run 3 cites `read_clauses`, which is in `model/intake.rs`. The tests still fail under each mutation, so AC-3 holds, but these notes are the mutation record and must be exact. | qsl-semantics/tests/it/tc235_record_event.rs:900-903,1360-1364; qsl-semantics/tests/it/tc235_business_meanings.rs:1030-1037,1776-1782,1855-1860 |
| FND-003 | low | The new doc comments on M13 and M14 put a Linear ticket id in public code ("deferred to the state-clause work in QSL-723", "deferred to QSL-723"). The repo's convention for unfinished references is `Remaining work: #N` (CLAUDE.md), and the runtime halves are in PR #789. | qsl-semantics/tests/it/tc235_record_event.rs:937-938,1079-1080 |
