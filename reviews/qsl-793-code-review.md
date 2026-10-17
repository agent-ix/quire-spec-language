---
id: SR-11758
title: "Code review of quire-spec-language PR #916: EffectiveView state_machine and conforms"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@741ec99c0afe8a37acb81e4946a96c39a1bd7e91; PR #916: qsl-semantics/src/model/effective_state_machine.rs, qsl-semantics/src/model/index.rs, qsl-semantics/src/model/mod.rs, qsl-semantics/tests/it/tc235_state_machine.rs, qsl-semantics/tests/it/tc235_business_meanings.rs"
review_set: subset
---
# Code review of quire-spec-language PR #916

## Summary

Ticket: QSL-793. One pass of code-review with the rust-review lane over
`git diff origin/main...HEAD` (5 files, +334/-74). Read-only, no build: the
author's `make ci` exited 0 before a final clean merge of main. Major bugs
only.

Examined:
- `EffectiveView::state_machine` (effective_state_machine.rs): machine
  selection by `conforms_admitted(machine, candidate)`, `None` when the key is
  not a `StateMachine` record, sort by `DeclarationKey` (derive Ord, package
  then node, so UTF-8 byte order within one package), then authored position.
- `EffectiveView::conforms` and `RecordIndex::conforms_admitted` (index.rs):
  the same memoized `ancestry` closure `conforms` already reads, with no
  ceiling. A completed view implies phase 3 placed every type (acyclic) and
  counted every direct edge against `ancestor_steps`, so the walk follows only
  admitted edges and terminates (the `expanded` set guards it regardless).
  No meter is reachable from either accessor, so nothing is charged.
- Both accessors take `&self`; the only interior state touched is the
  `OnceCell` ancestry cache, which feeds no identity.
- Tests: M19 runs 1 and 7 and `tc1049_view_exposes_machine_scope_and_conformance_by_key`.

## Verdict

PASS. No major bugs.

Test-oracle strength (asked explicitly):
- M19 runs 1 and 7 assert the exact ordered `Vec` of `(declaring, key,
  position)` for states and transitions, plus names, `from`/`to`/`trigger`,
  `guard` and `emits`. They fail an accessor that returns only the machine's
  own records, drops the ancestor's, drops the inherited guard, or orders by
  record insertion (K2 records `OrderLifecycle` before the pushed
  `ExpressLifecycle`). The test no longer reads source records itself.
- Residual (not a defect, recorded only): the fixture cannot tell
  key-byte ordering from a subtype-first ancestry walk, because
  `ExpressLifecycle` is both the subtype and the smaller key. A fixture whose
  ancestor key sorts first would separate them. The spec's own example has
  the same coincidence.
- The tc1049 test fails a display-name `conforms` (the copy cases), a
  `state_machine` answering object types or absent keys, and states or
  transitions leaking into `declarations()` (the final scan). The
  identity comparison against a second fresh normalization is weak in
  isolation (accessors are `&self`), but the AC asks for exactly that.
- Recorded oddity: run 1 pins 23 `normalize.record` charges, tc1049 pins 25.
  Consistent: the copy adds two declarations (type and `expedite`); states
  and transitions are not declarations. Not a bug.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
