---
id: SR-9050
title: "QSL-755 quire-spec-language PR 818 gap analysis: clause and operation collision"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7665c03365a107fc52bd7b0e741e61a4fee68a24; QSL-755 AC-1..3; QSpec TC-235 / FR-154-AC-12 (cited by id); qsl-semantics/src/model/intake.rs; qsl-semantics/tests/it/tc235_state_machine.rs"
review_set: subset
---

## Summary

Plan completion: not assessed

Ticket: QSL-755. The criteria are owned by the private QSpec repository and
are cited by id only. `quoin matrix` was not run: QSL tags QSpec criteria
with a `QSpec-` prefix and its own spec tree does not own FR-154. The check
below is a manual mapping of the ticket ACs to the test and code.

- QSL-755 AC-1 (tests-first vector): met. The red log shows the new test
  fails on the base with `malformed-declaration` against
  `conflicting-binding` (exit 101). The green log shows it pass (exit 0). The
  fixture adds a clause named like an operation to K2's `OrderLifecycle`,
  whose only existing clause is `NotYetPlaced`, so the appended clause is
  `clauses[1]` and the asserted member path `TypeDefinition.clauses[1]` is
  right. The charge count is derived from the fixture. Assertions are typed
  (`Code`, cause string, `ConflictingBinding.member`). None reads prose.
- QSL-755 AC-2 (the mapping handles `/clauses/j/identity`; no change to
  state, transition and step behaviour): met. The three existing arms keep
  their meaning check, key and member path.
- QSL-755 AC-3 (lib and it N>0, no new failure): met per the coder's gate
  log at the reviewed head: lib 522 and it 654 pass for qsl-semantics,
  plus eval, replay, root and xtask. Not re-run (reviewer-only).

Test-oracle strength: the test fails if the src change is reverted. It
asserts exactly one refusal, so a duplicate `malformed-declaration` next to
the new refusal would fail it. It does not assert the refusal key; code, cause and member are the
contract, so that is acceptable.

Checked for regressions from the wider arm, which also maps a clause that
repeats a field or another clause: no QSL test builds a clause that
duplicates an identity. The M03, M05, M06, M07, M13, M14, M19, M30 and M31
tests either add a fresh clause name or change a clause's `clauseId`,
language or text, so none reaches this arm. The coder's gate passed with
no change in any existing test.

## Verdict

PASS WITH FINDINGS (one medium, deferred; one low). All three ticket ACs
are met. FND-001 is an existing gap outside this ticket's ACs. This PR
does not cause it, so it should not block the merge. It needs a follow-up
ticket in the QSL code lane.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Under QSpec TC-235 / FR-154-AC-12 (cited by id) QSL maps duplicate identities to `conflicting-binding`, but a field and an operation of one owner with one name, or two fields or two operations with one name, are reported by the reader at `/fields/j/identity`, `/operations/k/identity` or `/operations/k/params/s/identity`. `duplicate_process_step_refusal` still returns `None` for those positions, so they fall through to the generic `malformed-declaration`. This is an existing gap, not a regression. Recommend a new QSL ticket that maps these positions with a vector, or settles the intended cause with the spec owner. | qsl-semantics/src/model/intake.rs:1666-1672 |
| FND-002 | low | The new test has no `#[trace("QSpec-TC-235", "QSpec-FR-154-AC-12")]` tag, which neighbouring M19 tests carry. No QSL test is tagged with FR-154-AC-12, so the computed matrix will not count this binding. | qsl-semantics/tests/it/tc235_state_machine.rs:895-899 |
