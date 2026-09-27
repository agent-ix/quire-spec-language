---
id: SR-753
title: "QSL-302 gap analysis of PR 499 (QSpec FR-340 frame-body admission evidence)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@172f4111aa0c515977abb650c364a7a929c7b556; Makefile; qsl-package/src/checked_v2/tests.rs; spec/functional/FR-087-typestate-and-cross-package-node-key.md; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md; spec/model-linking/tests.md (TC-053 row); spec/test-cases/TC-053-prove-guard-fact-soundness.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-053
    type: references
---
## Summary

Ticket: QSL-302. PR: quire-spec-language#499 at 172f4111. This PR has no plan
bundle. Its scope is the rescoped ticket, as the team-leader's comment on
QSL-302 describes it: FR-340 frame-body admission in the checked-package-v2
I2 reader, with QSpec's `frame_mutations` as the conformance evidence.

Plan completion: not applicable, because there is no plan bundle. On the
ticket's own terms, the admission logic already exists in the pinned IR, and
QSL's I2 read delegates to it (verified in SR-752). The evidence is
delivered: 26 of 26 vectors are replayed with exact code, cause and locus,
and three always-run unit tests are added.

Matrix: `quire coverage --scope . --json` at this head reports two things
about the new tests:

- `untracked_symbols`: all four new tests carry trace id `FR-340`. No QSL
  matrix row owns that id, because FR-340 is a QSpec requirement and not a
  QSL one.
- `shared_trace_ids`: `TC-053` is shared between these four tests and QSL's
  own TC-053, "Check Boolean guard-fact soundness against an independent
  truth table" (FR-016-AC-1, AC-7; marked Passed in
  spec/model-linking/tests.md:198). The frame tests have nothing to do with
  guard-fact soundness.

The sibling I2 conformance tests in the same file trace to QSL's own
`TC-253`/`FR-087-AC-3` (the full I2 read), for example tests.rs:1487 and
tests.rs:1572.

Underspecified code: none added. Only test code and a Makefile step changed.

Semantic review: done inline with SR-752, because this is a single-file,
test-only diff. The tests exercise the real IR path, and their assertions
match QSpec FR-340-AC-2, AC-3, AC-6, AC-8 and AC-9 through the vectors.

## Verdict

CONDITIONAL. There is one medium finding: the trace tags are wrong. They
attach frame-body evidence to an unrelated QSL test case and to an id that
QSL's matrix does not own. The fix is cheap and belongs in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | All four new tests use `#[trace("TC-053", "FR-340")]`. In this repo, TC-053 is FR-016's guard-fact soundness property test (spec/test-cases/TC-053-prove-guard-fact-soundness.md), so the tag falsely backs TC-053 with frame tests. `quire coverage` lists TC-053 under `shared_trace_ids`. FR-340 is a QSpec id, and QSL's matrix has no row for it (`untracked_symbols`). QSpec's own evidence for FR-340 is its TC-233, which is also a different test case in QSL. Suggested fix: retag to the QSL requirement that owns the I2 read, as the sibling I2 conformance tests do (`TC-253`/`FR-087-AC-3`), or add a QSL TC row for "I2 read over QSpec FR-340 frame vectors" and tag that. Rename the `tc_053_` function prefixes to match. | qsl-package/src/checked_v2/tests.rs:2033; qsl-package/src/checked_v2/tests.rs:2257; qsl-package/src/checked_v2/tests.rs:2274; qsl-package/src/checked_v2/tests.rs:2311 |
| FND-002 | low | Three QSpec FR-340 ACs have no always-run test: AC-1 (an all-empty frame body is admitted), AC-4 (a misordered array refuses `invalid_semantic_graph`) and AC-9 (the lower-digest frame wins). They are covered only when `QSPEC_DIR` is set (vectors `deletes-misordered` and `cross-frame-lower-digest-wins`; the published positive fixtures for AC-1), so the default `make ci` run never checks them. That is acceptable as long as `make conformance` is part of the gate. Optionally, add an empty-body case to `frame_fixture`, since it costs one line. | qsl-package/src/checked_v2/tests.rs:2224-2250 |

## Coverage

- Vectors: 26 of 26 `frame_mutations` replayed. Every vector is a refusal,
  and each is checked for code, cause and locus digest. The run against QSpec
  0d53cf2 passed. QSpec main e56756f has changed this contract (SR-752
  FND-001).
- Always-run unit tests: 3. One is an admit case, one refuses
  `missing_declaration`, and one refuses `invalid_model_binding`. Each asserts
  exact code, cause and locus.
- `quire coverage`: 4 new untracked symbols (trace id `FR-340`) and 1 new
  shared trace id (`TC-053`).
- Semantic review: done inline, because the diff is small and test-only.
